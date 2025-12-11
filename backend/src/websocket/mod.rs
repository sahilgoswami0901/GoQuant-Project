//! # WebSocket Module
//!
//! This module provides real-time updates to connected clients via WebSocket.
//!
//! ## Features
//!
//! - Real-time balance updates
//! - Transaction notifications
//! - TVL updates
//! - Lock/unlock event notifications
//!
//! ## Connection Flow
//!
//! ```text
//! 1. Client connects to /ws/:user
//!              ↓
//! 2. Server authenticates (optional)
//!              ↓
//! 3. Server subscribes to user's vault events
//!              ↓
//! 4. Events are pushed as they occur:
//!    - balance_update
//!    - transaction_confirmed
//!    - collateral_locked
//!    - collateral_unlocked
//! ```
//!
//! ## Message Format
//!
//! All messages are JSON:
//!
//! ```json
//! {
//!     "event": "balance_update",
//!     "data": {
//!         "totalBalance": 1000000000,
//!         "lockedBalance": 200000000,
//!         "availableBalance": 800000000
//!     },
//!     "timestamp": "2024-01-15T12:00:00Z"
//! }
//! ```

use std::sync::Arc;
use std::collections::HashMap;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use chrono::Utc;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast};
use tracing::{info, warn, debug, error};

use crate::AppState;

/// WebSocket event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WsEventType {
    /// Vault balance was updated.
    BalanceUpdate,
    /// Transaction was confirmed.
    TransactionConfirmed,
    /// Collateral was locked for a position.
    CollateralLocked,
    /// Collateral was unlocked.
    CollateralUnlocked,
    /// System health update.
    HealthUpdate,
    /// TVL update.
    TvlUpdate,
    /// Ping/pong for keepalive.
    Ping,
    /// Error message.
    Error,
}

/// WebSocket message wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsMessage<T> {
    /// Event type.
    pub event: WsEventType,
    /// Event data.
    pub data: T,
    /// Timestamp.
    pub timestamp: chrono::DateTime<Utc>,
}

impl<T: Serialize> WsMessage<T> {
    /// Create a new WebSocket message.
    pub fn new(event: WsEventType, data: T) -> Self {
        Self {
            event,
            data,
            timestamp: Utc::now(),
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Balance update event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceUpdateData {
    pub owner: String,
    pub total_balance: i64,
    pub locked_balance: i64,
    pub available_balance: i64,
}

/// Transaction confirmed event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionConfirmedData {
    pub transaction_id: String,
    pub transaction_type: String,
    pub amount: i64,
    pub signature: String,
}

/// Collateral locked event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralLockedData {
    pub owner: String,
    pub amount: i64,
    pub position_id: String,
    pub locked_balance: i64,
    pub available_balance: i64,
}

/// Collateral unlocked event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralUnlockedData {
    pub owner: String,
    pub amount: i64,
    pub position_id: String,
    pub locked_balance: i64,
    pub available_balance: i64,
}

/// WebSocket connection registry.
/// 
/// Tracks active WebSocket sessions per user and provides
/// methods to send messages to specific users or broadcast to all.
#[derive(Clone)]
pub struct WsRegistry {
    /// Map of user pubkey -> broadcast sender
    /// Each user can have multiple connections (multiple tabs/devices)
    sessions: Arc<Mutex<HashMap<String, Vec<broadcast::Sender<String>>>>>,
}

impl WsRegistry {
    /// Create a new WebSocket registry.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new WebSocket connection for a user.
    /// Returns a receiver that will receive messages for this user.
    pub async fn register(&self, user: String) -> broadcast::Receiver<String> {
        let mut sessions = self.sessions.lock().await;
        let (tx, rx) = broadcast::channel(100); // Buffer up to 100 messages
        
        sessions
            .entry(user.clone())
            .or_insert_with(Vec::new)
            .push(tx);
        
        info!("Registered WebSocket for user: {} (total connections: {})", 
            user, sessions.get(&user).map(|v| v.len()).unwrap_or(0));
        
        rx
    }

    /// Unregister a WebSocket connection for a user.
    /// Note: Senders are automatically cleaned up when receivers are dropped.
    /// We just remove entries that have no active receivers.
    pub async fn unregister(&self, user: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(senders) = sessions.get_mut(user) {
            // Remove senders with no active receivers
            senders.retain(|tx| tx.receiver_count() > 0);
            
            if senders.is_empty() {
                sessions.remove(user);
                info!("Unregistered all WebSocket connections for user: {}", user);
            } else {
                info!("Unregistered one WebSocket connection for user: {} (remaining: {})", 
                    user, senders.len());
            }
        }
    }

    /// Send a message to a specific user.
    ///
    /// All active WebSocket connections for that user will receive the message.
    /// If the user has no active connections, this is not an error (returns Ok).
    ///
    /// ## Arguments
    ///
    /// * `user` - User's public key
    /// * `event` - Event type
    /// * `data` - Event data (must implement Serialize)
    ///
    /// ## Returns
    ///
    /// * `Ok(())` - Message sent (or user not connected, which is not an error)
    /// * `Err(String)` - Failed to serialize message
    pub async fn send_to_user<T: Serialize>(
        &self,
        user: &str,
        event: WsEventType,
        data: T,
    ) -> Result<(), String> {
        let message = WsMessage::new(event, data);
        let json = message.to_json()
            .map_err(|e| format!("Failed to serialize message: {}", e))?;

        let mut sessions = self.sessions.lock().await;
        
        if let Some(senders) = sessions.get_mut(user) {
            let mut sent_count = 0;
            let mut dead_senders = Vec::new();
            
            // Check each sender and send message, marking dead ones for removal
            for (idx, sender) in senders.iter().enumerate() {
                // Check if sender has any active receivers before trying to send
                if sender.receiver_count() == 0 {
                    dead_senders.push(idx);
                    continue;
                }
                
                match sender.send(json.clone()) {
                    Ok(_) => sent_count += 1,
                    Err(_) => {
                        // Sender is closed, mark for removal
                        dead_senders.push(idx);
                    }
                }
            }
            
            // Remove dead senders (in reverse order to maintain indices)
            for &idx in dead_senders.iter().rev() {
                senders.remove(idx);
            }
            
            // If no senders left, remove the user entry
            if senders.is_empty() {
                sessions.remove(user);
            }
            
            if sent_count > 0 {
                debug!("Sent message to user {} ({} connections)", user, sent_count);
            }
            
            Ok(())
        } else {
            debug!("No active WebSocket connections for user: {}", user);
            Ok(()) // Not an error - user just not connected
        }
    }

/// Broadcast a message to all connected users.
///
    /// Sends the message to all active WebSocket connections across all users.
    ///
    /// ## Arguments
    ///
    /// * `event` - Event type
    /// * `data` - Event data (must implement Serialize)
    ///
    /// ## Returns
    ///
    /// * `Ok(())` - Message broadcasted
    /// * `Err(String)` - Failed to serialize message
    pub async fn broadcast<T: Serialize>(
        &self,
        event: WsEventType,
        data: T,
    ) -> Result<(), String> {
        let message = WsMessage::new(event, data);
        let json = message.to_json()
            .map_err(|e| format!("Failed to serialize message: {}", e))?;

        let sessions = self.sessions.lock().await;
        let mut total_sent = 0;
        
        for (_user, senders) in sessions.iter() {
            for sender in senders.iter() {
                if sender.send(json.clone()).is_ok() {
                    total_sent += 1;
                }
            }
        }
        
        info!("Broadcasted message to {} connections", total_sent);
        Ok(())
    }

    /// Get the number of active connections for a user.
    pub async fn connection_count(&self, user: &str) -> usize {
        let sessions = self.sessions.lock().await;
        sessions.get(user).map(|v| v.len()).unwrap_or(0)
    }

    /// Get total number of active connections across all users.
    pub async fn total_connections(&self) -> usize {
        let sessions = self.sessions.lock().await;
        sessions.values().map(|v| v.len()).sum()
    }
}

impl Default for WsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Note: We don't need to implement Clone manually since we're using Arc internally

/// Configure WebSocket routes.
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/ws/{user}", web::get().to(websocket_handler));
}

/// WebSocket connection handler.
///
/// Handles the initial WebSocket handshake and then manages
/// the bidirectional communication.
///
/// ## Endpoint
///
/// `GET /ws/:user`
///
/// ## Path Parameters
///
/// - `user` - User's wallet public key to subscribe to
///
/// ## Example (JavaScript)
///
/// ```javascript
/// const ws = new WebSocket('ws://localhost:8080/ws/7xKt9Fj2...');
///
/// ws.onmessage = (event) => {
///     const message = JSON.parse(event.data);
///     console.log('Event:', message.event);
///     console.log('Data:', message.data);
/// };
/// ```
pub async fn websocket_handler(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Payload,
    state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, actix_web::Error> {
    let user = path.into_inner();
    info!("WebSocket connection request for user: {}", user);

    // Upgrade to WebSocket connection
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    // Register this connection
    let ws_registry = state.ws_registry.clone();
    let mut rx = ws_registry.register(user.clone()).await;

    // Spawn a task to handle the WebSocket
    let user_clone = user.clone();
    let ws_registry_clone = ws_registry.clone();
    
    actix_rt::spawn(async move {
        info!("WebSocket connected for user: {}", user_clone);

        // Send initial connection confirmation
        let welcome = WsMessage::new(
            WsEventType::HealthUpdate,
            serde_json::json!({
                "status": "connected",
                "user": user_clone,
                "message": "You will receive real-time updates for your vault"
            }),
        );
        
        if let Ok(json) = welcome.to_json() {
            if let Err(e) = session.text(json).await {
                error!("Failed to send welcome message: {}", e);
            }
        }

        // Spawn a task to forward messages from registry to WebSocket
        let mut session_clone = session.clone();
        let user_for_task = user_clone.clone();
        let ws_registry_for_cleanup = ws_registry_clone.clone();
        actix_rt::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                match session_clone.text(msg).await {
                    Ok(_) => {
                        // Message sent successfully
                    }
                    Err(e) => {
                        debug!("WebSocket session closed for {}: {}. Stopping message forwarding.", user_for_task, e);
                        // Unregister this connection since it's dead
                        ws_registry_for_cleanup.unregister(&user_for_task).await;
                        break;
                    }
                }
            }
        });

        // Handle incoming messages
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Ping(bytes) => {
                    debug!("Received ping from {}", user_clone);
                    let _ = session.pong(&bytes).await;
                }
                Message::Pong(_) => {
                    debug!("Received pong from {}", user_clone);
                }
                Message::Text(text) => {
                    debug!("Received text from {}: {}", user_clone, text);
                    
                    // Handle incoming messages (e.g., subscription requests)
                    // For now, we just echo back
                    let response = WsMessage::new(
                        WsEventType::Ping,
                        serde_json::json!({ 
                            "received": text.to_string(),
                            "message": "WebSocket is active and listening for events"
                        }),
                    );
                    
                    if let Ok(json) = response.to_json() {
                        let _ = session.text(json).await;
                    }
                }
                Message::Binary(_) => {
                    warn!("Received unexpected binary message from {}", user_clone);
                }
                Message::Close(reason) => {
                    info!("WebSocket closed for {}: {:?}", user_clone, reason);
                    break;
                }
                _ => {}
            }
        }

        // Unregister on disconnect
        ws_registry_clone.unregister(&user_clone).await;
        info!("WebSocket disconnected for user: {}", user_clone);
    });

    Ok(response)
}

/// Send a message to a specific user via WebSocket.
///
/// This is a convenience function that uses the registry from AppState.
/// Send a message to a specific user via WebSocket.
///
/// This is a convenience function that uses the registry from AppState.
/// It's the recommended way to send WebSocket messages from API handlers.
///
/// ## Arguments
///
/// * `state` - Application state containing the WebSocket registry
/// * `user` - User's public key
/// * `event` - Event type
/// * `data` - Event data (must implement Serialize)
///
/// ## Returns
///
/// * `Ok(())` - Message sent (or user not connected)
/// * `Err(String)` - Failed to send message
pub async fn send_to_user<T: Serialize>(
    state: &Arc<AppState>,
    user: &str,
    event: WsEventType,
    data: T,
) -> Result<(), String> {
    state.ws_registry.send_to_user(user, event, data).await
}

/// Broadcast a message to all connected users.
/// 
/// This is a convenience function that uses the registry from AppState.
#[allow(dead_code)]
pub async fn broadcast<T: Serialize>(
    state: &Arc<AppState>,
    event: WsEventType,
    data: T,
) -> Result<(), String> {
    state.ws_registry.broadcast(event, data).await
}

