//! # API Request Handlers
//!
//! This module contains the handler functions for each API endpoint.
//! Each handler:
//! 1. Extracts request data
//! 2. Validates input
//! 3. Calls the appropriate service
//! 4. Returns a formatted response
//!
//! ## Error Handling
//!
//! All errors are caught and returned as JSON:
//!
//! ```json
//! {
//!     "success": false,
//!     "error": {
//!         "code": "INSUFFICIENT_BALANCE",
//!         "message": "Not enough available balance"
//!     }
//! }
//! ```

use std::sync::Arc;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use tracing::{info, error, warn};

use crate::AppState;
use crate::models::{
    ApiResponse,
    InitializeVaultRequest,
    DepositRequest,
    WithdrawRequest,
    LockCollateralRequest,
    UnlockCollateralRequest,
    TransferCollateralRequest,
    MintUsdtRequest,
    TransactionQuery,
    VaultBalanceResponse,
    TransactionResponse,
    TransactionListResponse,
    TvlResponse,
    HealthResponse,
};
use crate::services::TokenMinter;
use crate::websocket::{send_to_user, WsEventType, BalanceUpdateData, TransactionConfirmedData, CollateralLockedData, CollateralUnlockedData};
use serde_json::json;

/// API information endpoint (root).
///
/// Returns information about available API endpoints.
///
/// ## Endpoint
///
/// `GET /`
pub async fn api_info() -> HttpResponse {
    let info = json!({
        "name": "Collateral Vault API",
        "version": "0.1.0",
        "description": "Backend API for Collateral Vault Management System",
        "endpoints": {
            "health": {
                "method": "GET",
                "path": "/health",
                "description": "Health check endpoint"
            },
            "vault": {
                "initialize": {
                    "method": "POST",
                    "path": "/vault/initialize",
                    "description": "Initialize a new vault for a user"
                },
                "deposit": {
                    "method": "POST",
                    "path": "/vault/deposit",
                    "description": "Deposit USDT into vault"
                },
                "withdraw": {
                    "method": "POST",
                    "path": "/vault/withdraw",
                    "description": "Withdraw USDT from vault"
                },
                "balance": {
                    "method": "GET",
                    "path": "/vault/balance/{user}",
                    "description": "Get vault balance for a user"
                },
                "transactions": {
                    "method": "GET",
                    "path": "/vault/transactions/{user}",
                    "description": "Get transaction history for a user"
                },
                "tvl": {
                    "method": "GET",
                    "path": "/vault/tvl",
                    "description": "Get Total Value Locked across all vaults"
                }
            }
        }
    });

    HttpResponse::Ok()
        .content_type("application/json")
        .json(ApiResponse {
            success: true,
            data: Some(info),
            error: None,
        })
}

/// Health check endpoint.
///
/// Check if the backend is running and healthy.
///
/// ## Endpoint
///
/// `GET /health`
///
/// ## Example
///
/// ```bash
/// curl http://127.0.0.1:8080/health
/// ```
///
/// ## Response
///
/// ```json
/// {
///     "success": true,
///     "data": {
///         "status": "healthy",
///         "database": true,
///         "solanaRpc": true,
///         "version": "0.1.0",
///         "timestamp": "2025-12-08T12:00:00Z"
///     }
/// }
/// ```
pub async fn health_check(
    state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    // Check database
    let db_healthy = state.db.pool()
        .get()
        .await
        .is_ok();

    // Check Solana RPC
    let solana_healthy = state.solana
        .get_health()
        .await
        .unwrap_or(false);

    let overall_healthy = db_healthy && solana_healthy;

    let response = HealthResponse {
        status: if overall_healthy { "healthy" } else { "unhealthy" }.to_string(),
        database: db_healthy,
        solana_rpc: solana_healthy,
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
    };

    let status_code = if overall_healthy {
        actix_web::http::StatusCode::OK
    } else {
        actix_web::http::StatusCode::SERVICE_UNAVAILABLE
    };

    HttpResponse::build(status_code)
        .json(ApiResponse::success(response))
}

/// Initialize a new vault.
///
/// Create a new vault for your wallet. This creates the PDA account on-chain.
///
/// ## Endpoint
///
/// `POST /vault/initialize`
///
/// ## Option A: Auto-Submit (Recommended for Devnet/Testing)
///
/// The backend can automatically sign and submit the transaction if you provide your keypair path:
///
/// ```bash
/// curl -X POST http://127.0.0.1:8080/vault/initialize \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userPubkey": "YOUR_WALLET_ADDRESS",
///     "userKeypairPath": "~/.config/solana/id.json"
///   }'
/// ```
///
/// **Response (Auto-Submit):**
/// ```json
/// {
///     "success": true,
///     "data": {
///         "transactionId": "...",
///         "status": "submitted",
///         "signature": "5Ht3Rjabc123...",
///         "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
///     }
/// }
/// ```
///
/// ## Option B: Manual Sign & Submit
///
/// If you don't provide `userKeypairPath`, you'll receive an unsigned transaction:
///
/// ```bash
/// curl -X POST http://127.0.0.1:8080/vault/initialize \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userPubkey": "YOUR_WALLET_ADDRESS"
///   }'
/// ```
///
/// **Response (Manual Sign):**
/// ```json
/// {
///     "success": true,
///     "data": {
///         "transactionId": "550e8400-e29b-41d4-a716-446655440000",
///         "status": "pending",
///         "unsignedTransaction": "base64_encoded_transaction...",
///         "message": "Sign and submit this transaction with your wallet"
///     }
/// }
/// ```
pub async fn initialize_vault(
    state: web::Data<Arc<AppState>>,
    body: web::Json<InitializeVaultRequest>,
) -> HttpResponse {
    info!("Initialize vault request for: {}", body.user_pubkey);

    match state.vault_manager.initialize_vault(body.into_inner()).await {
        Ok(result) => {
            HttpResponse::Ok().json(ApiResponse::success(result))
        }
        Err(e) => {
            error!("Initialize vault failed: {}", e);
            HttpResponse::BadRequest().json(
                ApiResponse::<()>::error("INITIALIZATION_FAILED", &e.to_string())
            )
        }
    }
}

/// Deposit USDT into vault.
///
/// Deposit USDT tokens into your vault.
///
/// **⚠️ Important:** Before depositing, you must have USDT in your token account. The vault does NOT provide USDT - you need to acquire it first through:
/// - Buying USDT on a DEX (Jupiter, Raydium, Orca)
/// - Receiving USDT from another user
/// - Bridging USDT from another blockchain
/// - Using the `/vault/mint-usdt` endpoint (devnet only)
///
/// See `USDT_ACQUISITION_GUIDE.md` for detailed instructions.
///
/// **Note:** Amount is in smallest units (6 decimals).
/// - 1 USDT = 1,000,000
/// - 100 USDT = 100,000,000
///
/// ## Endpoint
///
/// `POST /vault/deposit`
///
/// ## Option A: Auto-Submit (Recommended for Devnet/Testing)
///
/// The backend can automatically sign and submit the transaction if you provide your keypair path:
///
/// ```bash
/// curl -X POST http://127.0.0.1:8080/vault/deposit \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userPubkey": "YOUR_WALLET_ADDRESS",
///     "amount": 100000000,
///     "userKeypairPath": "~/.config/solana/id.json"
///   }'
/// ```
///
/// **Response (Auto-Submit):**
/// ```json
/// {
///     "success": true,
///     "data": {
///         "transactionId": "...",
///         "status": "submitted",
///         "signature": "5Ht3Rjabc123...",
///         "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
///     }
/// }
/// ```
///
/// ## Option B: Manual Sign & Submit
///
/// If you don't provide `userKeypairPath`, you'll receive an unsigned transaction:
///
/// ```bash
/// curl -X POST http://127.0.0.1:8080/vault/deposit \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userPubkey": "YOUR_WALLET_ADDRESS",
///     "amount": 100000000
///   }'
/// ```
///
/// **Response (Manual Sign):**
/// ```json
/// {
///     "success": true,
///     "data": {
///         "transactionId": "...",
///         "status": "pending",
///         "unsignedTransaction": "base64...",
///         "message": "Sign and submit this transaction"
///     }
/// }
/// ```
pub async fn deposit(
    state: web::Data<Arc<AppState>>,
    body: web::Json<DepositRequest>,
) -> HttpResponse {
    info!(
        "Deposit request: {} USDT for {}",
        body.amount as f64 / 1_000_000.0,
        body.user_pubkey
    );

    // Validate amount
    if body.amount == 0 {
        return HttpResponse::BadRequest().json(
            ApiResponse::<()>::error("INVALID_AMOUNT", "Amount must be greater than 0")
        );
    }

    let request = body.into_inner();
    let user_pubkey = request.user_pubkey.clone();
    let amount = request.amount;
    
    match state.vault_manager.deposit(request).await {
        Ok(result) => {
            // Send WebSocket notification if transaction was confirmed
            if let Some(signature) = &result.signature {
                // Small delay to ensure database transaction is committed
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                // Get updated balance for notification
                match state.vault_manager.get_vault_balance(&user_pubkey).await {
                    Ok(balance) => {
                        info!("📊 Sending WebSocket events for deposit - Balance: {} USDT", 
                            balance.total_balance as f64 / 1_000_000.0);
                        
                        // Send balance update
                        if let Err(e) = send_to_user(
                            &state,
                            &user_pubkey,
                            WsEventType::BalanceUpdate,
                            BalanceUpdateData {
                                owner: user_pubkey.clone(),
                                total_balance: balance.total_balance,
                                locked_balance: balance.locked_balance,
                                available_balance: balance.available_balance,
                            },
                        ).await {
                            warn!("Failed to send balance update via WebSocket: {}", e);
                        }
                        
                        // Send transaction confirmed
                        if let Err(e) = send_to_user(
                            &state,
                            &user_pubkey,
                            WsEventType::TransactionConfirmed,
                            TransactionConfirmedData {
                                transaction_id: result.transaction_id.to_string(),
                                transaction_type: "deposit".to_string(),
                                amount: amount as i64,
                                signature: signature.clone(),
                            },
                        ).await {
                            warn!("Failed to send transaction confirmed via WebSocket: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to fetch balance for WebSocket notification: {}", e);
                    }
                }
            }
            
            HttpResponse::Ok().json(ApiResponse::success(result))
        }
        Err(e) => {
            error!("Deposit failed: {}", e);
            
            let (code, message) = match &e {
                crate::services::vault_manager::VaultError::VaultNotFound(_) => {
                    ("VAULT_NOT_FOUND", e.to_string())
                }
                crate::services::vault_manager::VaultError::InsufficientBalance { .. } => {
                    ("INSUFFICIENT_BALANCE", e.to_string())
                }
                _ => ("DEPOSIT_FAILED", e.to_string())
            };
            
            HttpResponse::BadRequest().json(
                ApiResponse::<()>::error(code, &message)
            )
        }
    }
}

/// Withdraw USDT from vault.
///
/// Withdraw USDT from your vault back to your wallet.
///
/// ## Endpoint
///
/// `POST /vault/withdraw`
///
/// ## Option A: Auto-Submit (Recommended for Devnet/Testing)
///
/// ```bash
/// curl -X POST http://127.0.0.1:8080/vault/withdraw \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userPubkey": "YOUR_WALLET_ADDRESS",
///     "amount": 50000000,
///     "userKeypairPath": "~/.config/solana/id.json"
///   }'
/// ```
///
/// **Response:**
/// ```json
/// {
///     "success": true,
///     "data": {
///         "transactionId": "...",
///         "status": "submitted",
///         "signature": "5Ht3Rjabc123...",
///         "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
///     }
/// }
/// ```
///
/// ## Option B: Manual Sign & Submit
///
/// ```bash
/// curl -X POST http://127.0.0.1:8080/vault/withdraw \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userPubkey": "YOUR_WALLET_ADDRESS",
///     "amount": 50000000
///   }'
/// ```
///
/// ## Errors
///
/// - `VAULT_NOT_FOUND` - Vault doesn't exist
/// - `INSUFFICIENT_BALANCE` - Not enough available balance
/// - `INVALID_AMOUNT` - Amount is 0
pub async fn withdraw(
    state: web::Data<Arc<AppState>>,
    body: web::Json<WithdrawRequest>,
) -> HttpResponse {
    info!(
        "Withdraw request: {} USDT for {}",
        body.amount as f64 / 1_000_000.0,
        body.user_pubkey
    );

    // Validate amount
    if body.amount == 0 {
        return HttpResponse::BadRequest().json(
            ApiResponse::<()>::error("INVALID_AMOUNT", "Amount must be greater than 0")
        );
    }

    let request = body.into_inner();
    let user_pubkey = request.user_pubkey.clone();
    let amount = request.amount;
    
    match state.vault_manager.withdraw(request).await {
        Ok(result) => {
            // Send WebSocket notification if transaction was confirmed
            if let Some(signature) = &result.signature {
                // Small delay to ensure database transaction is committed
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                // Get updated balance for notification
                match state.vault_manager.get_vault_balance(&user_pubkey).await {
                    Ok(balance) => {
                        info!("📊 Sending WebSocket events for withdrawal - Balance: {} USDT", 
                            balance.total_balance as f64 / 1_000_000.0);
                        
                        // Send balance update
                        if let Err(e) = send_to_user(
                            &state,
                            &user_pubkey,
                            WsEventType::BalanceUpdate,
                            BalanceUpdateData {
                                owner: user_pubkey.clone(),
                                total_balance: balance.total_balance,
                                locked_balance: balance.locked_balance,
                                available_balance: balance.available_balance,
                            },
                        ).await {
                            warn!("Failed to send balance update via WebSocket: {}", e);
                        }
                        
                        // Send transaction confirmed
                        if let Err(e) = send_to_user(
                            &state,
                            &user_pubkey,
                            WsEventType::TransactionConfirmed,
                            TransactionConfirmedData {
                                transaction_id: result.transaction_id.to_string(),
                                transaction_type: "withdraw".to_string(),
                                amount: amount as i64,
                                signature: signature.clone(),
                            },
                        ).await {
                            warn!("Failed to send transaction confirmed via WebSocket: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to fetch balance for WebSocket notification: {}", e);
                    }
                }
            }
            
            HttpResponse::Ok().json(ApiResponse::success(result))
        }
        Err(e) => {
            error!("Withdraw failed: {}", e);
            
            let (code, message) = match &e {
                crate::services::vault_manager::VaultError::VaultNotFound(_) => {
                    ("VAULT_NOT_FOUND", e.to_string())
                }
                crate::services::vault_manager::VaultError::InsufficientBalance { .. } => {
                    ("INSUFFICIENT_BALANCE", e.to_string())
                }
                _ => ("WITHDRAW_FAILED", e.to_string())
            };
            
            HttpResponse::BadRequest().json(
                ApiResponse::<()>::error(code, &message)
            )
        }
    }
}

/// Lock collateral for a trading position.
///
/// Locks a portion of a user's available balance so it cannot be withdrawn. Must be signed by the **position manager** (authorized program).
///
/// **⚠️ Note:** The `positionId` is a unique identifier for the trading position that requires this collateral lock. It can be any string (e.g., "pos_123abc", "position_001", etc.).
///
/// ## Endpoint
///
/// `POST /vault/lock-collateral`
///
/// ## Request Body
///
/// ```json
/// {
///     "userPubkey": "USER_VAULT_OWNER",
///     "amount": 50000000,
///     "positionId": "pos_123abc",
///     "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
/// }
/// ```
///
/// ## Example
///
/// ```bash
/// curl -X POST http://127.0.0.1:8080/vault/lock-collateral \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userPubkey": "YOUR_WALLET_ADDRESS",
///     "amount": 50000000,
///     "positionId": "pos_1",
///     "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
///   }'
/// ```
///
/// **Response:**
/// ```json
/// {
///     "success": true,
///     "data": {
///         "transactionId": "...",
///         "status": "submitted",
///         "signature": "5Ht3Rjabc123...",
///         "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
///     }
/// }
/// ```
pub async fn lock_collateral(
    state: web::Data<Arc<AppState>>,
    body: web::Json<LockCollateralRequest>,
) -> HttpResponse {
    info!(
        "Lock collateral request: {} USDT for position {} (user: {})",
        body.amount as f64 / 1_000_000.0,
        body.position_id,
        body.user_pubkey
    );

    // Validate amount
    if body.amount == 0 {
        return HttpResponse::BadRequest().json(
            ApiResponse::<()>::error("INVALID_AMOUNT", "Amount must be greater than 0")
        );
    }

    let user_pubkey = body.user_pubkey.clone();
    let amount = body.amount;
    let position_id = body.position_id.clone();

    match state.vault_manager.lock_collateral(
        &body.user_pubkey,
        body.amount,
        &body.position_id,
        Some(&body.position_manager_keypair_path),
    ).await {
        Ok(result) => {
            // Send WebSocket notifications
            if let Some(signature) = &result.signature {
                // Fetch the latest balance after the update
                // Add a small delay to ensure database transaction is committed
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                match state.vault_manager.get_vault_balance(&user_pubkey).await {
                    Ok(balance) => {
                        info!("📊 Sending WebSocket events with balance - Locked: {} USDT, Available: {} USDT", 
                            balance.locked_balance as f64 / 1_000_000.0,
                            balance.available_balance as f64 / 1_000_000.0);
                        
                        // Send collateral locked event
                        let _ = send_to_user(
                            &state,
                            &user_pubkey,
                            WsEventType::CollateralLocked,
                            CollateralLockedData {
                                owner: user_pubkey.clone(),
                                amount: amount as i64,
                                position_id: position_id.clone(),
                                locked_balance: balance.locked_balance,
                                available_balance: balance.available_balance,
                            },
                        ).await;

                        // Send balance update
                        let _ = send_to_user(
                            &state,
                            &user_pubkey,
                            WsEventType::BalanceUpdate,
                            BalanceUpdateData {
                                owner: user_pubkey.clone(),
                                total_balance: balance.total_balance,
                                locked_balance: balance.locked_balance,
                                available_balance: balance.available_balance,
                            },
                        ).await;
                        
                        // Send transaction confirmed event
                        let _ = send_to_user(
                            &state,
                            &user_pubkey,
                            WsEventType::TransactionConfirmed,
                            TransactionConfirmedData {
                                transaction_id: result.transaction_id.to_string(),
                                transaction_type: "lock".to_string(),
                                amount: amount as i64,
                                signature: signature.clone(),
                            },
                        ).await;
                    }
                    Err(e) => {
                        warn!("Failed to fetch balance for WebSocket notification: {}", e);
                    }
                }
            }

            HttpResponse::Ok().json(ApiResponse::success(result))
        }
        Err(e) => {
            error!("Lock collateral failed: {}", e);
            
            let (code, message) = match &e {
                crate::services::vault_manager::VaultError::VaultNotFound(_) => {
                    ("VAULT_NOT_FOUND", e.to_string())
                }
                crate::services::vault_manager::VaultError::InsufficientBalance { .. } => {
                    ("INSUFFICIENT_BALANCE", e.to_string())
                }
                _ => ("LOCK_FAILED", e.to_string())
            };
            
            HttpResponse::BadRequest().json(
                ApiResponse::<()>::error(code, &message)
            )
        }
    }
}

/// Unlock collateral after position close.
///
/// Releases previously locked collateral back to available balance. Must be signed by the **position manager** (authorized program).
///
/// **⚠️ Note:** The `positionId` must match the position ID used when the collateral was originally locked.
///
/// ## Endpoint
///
/// `POST /vault/unlock-collateral`
///
/// ## Request Body
///
/// ```json
/// {
///     "userPubkey": "USER_VAULT_OWNER",
///     "amount": 50000000,
///     "positionId": "pos_1",
///     "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
/// }
/// ```
///
/// ## Example
///
/// ```bash
/// curl -X POST http://127.0.0.1:8080/vault/unlock-collateral \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userPubkey": "YOUR_WALLET_ADDRESS",
///     "amount": 50000000,
///     "positionId": "pos_1",
///     "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
///   }'
/// ```
///
/// **Response:**
/// ```json
/// {
///     "success": true,
///     "data": {
///         "transactionId": "...",
///         "status": "submitted",
///         "signature": "5Ht3Rjabc123...",
///         "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
///     }
/// }
/// ```
pub async fn unlock_collateral(
    state: web::Data<Arc<AppState>>,
    body: web::Json<UnlockCollateralRequest>,
) -> HttpResponse {
    info!(
        "Unlock collateral request: {} USDT for position {} (user: {})",
        body.amount as f64 / 1_000_000.0,
        body.position_id,
        body.user_pubkey
    );

    // Validate amount
    if body.amount == 0 {
        return HttpResponse::BadRequest().json(
            ApiResponse::<()>::error("INVALID_AMOUNT", "Amount must be greater than 0")
        );
    }

    let user_pubkey = body.user_pubkey.clone();
    let amount = body.amount;
    let position_id = body.position_id.clone();

    match state.vault_manager.unlock_collateral(
        &body.user_pubkey,
        body.amount,
        &body.position_id,
        Some(&body.position_manager_keypair_path),
    ).await {
        Ok(result) => {
            // Send WebSocket notifications
            if let Some(signature) = &result.signature {
                if let Ok(balance) = state.vault_manager.get_vault_balance(&user_pubkey).await {
                    // Send collateral unlocked event
                    let _ = send_to_user(
                        &state,
                        &user_pubkey,
                        WsEventType::CollateralUnlocked,
                        CollateralUnlockedData {
                            owner: user_pubkey.clone(),
                            amount: amount as i64,
                            position_id: position_id.clone(),
                            locked_balance: balance.locked_balance,
                            available_balance: balance.available_balance,
                        },
                    ).await;

                    // Send balance update
                    let _ = send_to_user(
                        &state,
                        &user_pubkey,
                        WsEventType::BalanceUpdate,
                        BalanceUpdateData {
                            owner: user_pubkey.clone(),
                            total_balance: balance.total_balance,
                            locked_balance: balance.locked_balance,
                            available_balance: balance.available_balance,
                        },
                    ).await;
                    
                    // Send transaction confirmed event
                    let _ = send_to_user(
                        &state,
                        &user_pubkey,
                        WsEventType::TransactionConfirmed,
                        TransactionConfirmedData {
                            transaction_id: result.transaction_id.to_string(),
                            transaction_type: "unlock".to_string(),
                            amount: amount as i64,
                            signature: signature.clone(),
                        },
                    ).await;
                }
            }

            HttpResponse::Ok().json(ApiResponse::success(result))
        }
        Err(e) => {
            error!("Unlock collateral failed: {}", e);
            
            let (code, message) = match &e {
                crate::services::vault_manager::VaultError::VaultNotFound(_) => {
                    ("VAULT_NOT_FOUND", e.to_string())
                }
                crate::services::vault_manager::VaultError::InsufficientBalance { .. } => {
                    ("INSUFFICIENT_LOCKED_BALANCE", e.to_string())
                }
                _ => ("UNLOCK_FAILED", e.to_string())
            };
            
            HttpResponse::BadRequest().json(
                ApiResponse::<()>::error(code, &message)
            )
        }
    }
}

/// Transfer collateral between vaults.
///
/// Moves collateral between two vaults (e.g., settlement or liquidation). Must be signed by the **liquidation engine** keypair (authorized program). `reason` helps auditing (`settlement`, `liquidation`, `fee`).
///
/// ## Endpoint
///
/// `POST /vault/transfer-collateral`
///
/// ## Request Body
///
/// ```json
/// {
///     "fromPubkey": "SOURCE_VAULT_OWNER",
///     "toPubkey": "DESTINATION_VAULT_OWNER",
///     "amount": 50000000,
///     "reason": "settlement",
///     "liquidationEngineKeypairPath": "~/.config/solana/liquidation-engine.json"
/// }
/// ```
///
/// ## Example
///
/// ```bash
/// curl -X POST http://127.0.0.1:8080/vault/transfer-collateral \
///   -H "Content-Type: application/json" \
///   -d '{
///     "fromPubkey": "SOURCE_WALLET",
///     "toPubkey": "DEST_WALLET",
///     "amount": 50000000,
///     "reason": "settlement",
///     "liquidationEngineKeypairPath": "~/.config/solana/liquidation-engine.json"
///   }'
/// ```
///
/// ## Reasons
///
/// - `settlement`: Trade settlement (winner receives from loser)
/// - `liquidation`: Position was liquidated
/// - `fee`: Protocol fee collection
pub async fn transfer_collateral(
    state: web::Data<Arc<AppState>>,
    body: web::Json<TransferCollateralRequest>,
) -> HttpResponse {
    info!(
        "Transfer collateral request: {} USDT from {} to {} (reason: {})",
        body.amount as f64 / 1_000_000.0,
        body.from_pubkey,
        body.to_pubkey,
        body.reason
    );

    // Validate amount
    if body.amount == 0 {
        return HttpResponse::BadRequest().json(
            ApiResponse::<()>::error("INVALID_AMOUNT", "Amount must be greater than 0")
        );
    }

    let from_pubkey = body.from_pubkey.clone();
    let to_pubkey = body.to_pubkey.clone();

    match state.vault_manager.transfer_collateral(
        &body.from_pubkey,
        &body.to_pubkey,
        body.amount,
        &body.reason,
        Some(&body.liquidation_engine_keypair_path),
    ).await {
        Ok(result) => {
            // Send WebSocket notifications to both users
            if let Ok(from_balance) = state.vault_manager.get_vault_balance(&from_pubkey).await {
                let _ = send_to_user(
                    &state,
                    &from_pubkey,
                    WsEventType::BalanceUpdate,
                    BalanceUpdateData {
                        owner: from_pubkey.clone(),
                        total_balance: from_balance.total_balance,
                        locked_balance: from_balance.locked_balance,
                        available_balance: from_balance.available_balance,
                    },
                ).await;
            }

            if let Ok(to_balance) = state.vault_manager.get_vault_balance(&to_pubkey).await {
                let _ = send_to_user(
                    &state,
                    &to_pubkey,
                    WsEventType::BalanceUpdate,
                    BalanceUpdateData {
                        owner: to_pubkey.clone(),
                        total_balance: to_balance.total_balance,
                        locked_balance: to_balance.locked_balance,
                        available_balance: to_balance.available_balance,
                    },
                ).await;
            }

            HttpResponse::Ok().json(ApiResponse::success(result))
        }
        Err(e) => {
            error!("Transfer collateral failed: {}", e);
            
            let (code, message) = match &e {
                crate::services::vault_manager::VaultError::VaultNotFound(_) => {
                    ("VAULT_NOT_FOUND", e.to_string())
                }
                crate::services::vault_manager::VaultError::InsufficientBalance { .. } => {
                    ("INSUFFICIENT_BALANCE", e.to_string())
                }
                _ => ("TRANSFER_FAILED", e.to_string())
            };
            
            HttpResponse::BadRequest().json(
                ApiResponse::<()>::error(code, &message)
            )
        }
    }
}

/// Get vault balance.
///
/// Check your vault balance (total, locked, and available).
///
/// ## Endpoint
///
/// `GET /vault/balance/{user}`
///
/// ## Example
///
/// ```bash
/// curl http://127.0.0.1:8080/vault/balance/YOUR_WALLET_ADDRESS
/// ```
///
/// ## Path Parameters
///
/// - `user` - User's wallet public key (base58)
///
/// ## Response
///
/// ```json
/// {
///     "success": true,
///     "data": {
///         "owner": "GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M",
///         "totalBalance": 100000000,
///         "lockedBalance": 0,
///         "availableBalance": 100000000,
///         "totalDeposited": 100000000,
///         "totalWithdrawn": 0
///     }
/// }
/// ```
pub async fn get_balance(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> HttpResponse {
    let user = path.into_inner();
    info!("Balance request for: {}", user);

    match state.vault_manager.get_vault_balance(&user).await {
        Ok(balance) => {
            HttpResponse::Ok().json(ApiResponse::success(balance))
        }
        Err(e) => {
            error!("Get balance failed: {}", e);
            
            let (code, status) = match &e {
                crate::services::vault_manager::VaultError::VaultNotFound(_) => {
                    ("VAULT_NOT_FOUND", actix_web::http::StatusCode::NOT_FOUND)
                }
                _ => ("BALANCE_QUERY_FAILED", actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
            };
            
            HttpResponse::build(status).json(
                ApiResponse::<()>::error(code, &e.to_string())
            )
        }
    }
}

/// Get transaction history.
///
/// View all transactions for your vault.
///
/// ## Endpoint
///
/// `GET /vault/transactions/{user}?limit=20&offset=0`
///
/// ## Example
///
/// ```bash
/// curl "http://127.0.0.1:8080/vault/transactions/YOUR_WALLET_ADDRESS?limit=20&offset=0"
/// ```
///
/// ## Path Parameters
///
/// - `user` - User's wallet public key
///
/// ## Query Parameters
///
/// - `limit` - Number of transactions (default: 20, max: 100)
/// - `offset` - Skip N transactions (for pagination)
/// - `type` - Filter by type (deposit, withdrawal, lock, unlock, etc.)
///
/// ## Example with filters
///
/// ```bash
/// curl "http://127.0.0.1:8080/vault/transactions/GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M?limit=10&type=deposit"
/// ```
///
/// ## Response
///
/// ```json
/// {
///     "success": true,
///     "data": {
///         "transactions": [...],
///         "total": 42,
///         "offset": 0,
///         "limit": 20
///     }
/// }
/// ```
pub async fn get_transactions(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    query: web::Query<TransactionQuery>,
) -> HttpResponse {
    let user = path.into_inner();
    let limit = query.limit.min(100); // Cap at 100
    let offset = query.offset.max(0);

    info!(
        "Transaction history request for: {} (limit: {}, offset: {})",
        user, limit, offset
    );

    match state.vault_manager.get_transactions(&user, limit, offset).await {
        Ok(transactions) => {
            // Convert to response format
            let tx_responses: Vec<TransactionResponse> = transactions
                .into_iter()
                .map(|tx| TransactionResponse {
                    id: tx.id,
                    transaction_type: tx.transaction_type,
                    amount: tx.amount,
                    formatted_amount: VaultBalanceResponse::format_usdt(tx.amount),
                    signature: tx.signature,
                    status: tx.status,
                    balance_before: tx.balance_before,
                    balance_after: tx.balance_after,
                    counterparty: tx.counterparty,
                    note: tx.note,
                    created_at: tx.created_at,
                    confirmed_at: tx.confirmed_at,
                })
                .collect();

            let response = TransactionListResponse {
                transactions: tx_responses,
                total: 0, // TODO: Get actual count
                offset,
                limit,
            };

            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => {
            error!("Get transactions failed: {}", e);
            HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("TRANSACTION_QUERY_FAILED", &e.to_string())
            )
        }
    }
}

/// Get Total Value Locked.
///
/// See the total value locked across all vaults.
///
/// ## Endpoint
///
/// `GET /vault/tvl`
///
/// ## Example
///
/// ```bash
/// curl http://127.0.0.1:8080/vault/tvl
/// ```
///
/// ## Response
///
/// ```json
/// {
///     "success": true,
///     "data": {
///         "totalValueLocked": 500000000,
///         "activeVaults": 5,
///         "totalLocked": 200000000,
///         "totalAvailable": 300000000,
///         "timestamp": "2025-12-08T12:00:00Z"
///     }
/// }
/// ```
pub async fn get_tvl(
    state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    info!("TVL request");

    match state.balance_tracker.get_tvl().await {
        Ok((total_tvl, total_locked, total_available, active_vaults)) => {
            let formatted = format!("{:.2} USDT", total_tvl as f64 / 1_000_000.0);
            
            let response = TvlResponse {
                total_value_locked: total_tvl,
                formatted_tvl: formatted,
                active_vaults: active_vaults as i64,
                total_locked,
                total_available,
                timestamp: Utc::now(),
            };

            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => {
            error!("Get TVL failed: {}", e);
            HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("TVL_QUERY_FAILED", &e)
            )
        }
    }
}

/// Mint test USDT tokens (devnet only).
///
/// Mint test USDT tokens to your token account. **This endpoint only works on devnet!**
///
/// **⚠️ Important:**
/// - Only works on devnet/localhost
/// - Requires backend keypair to have mint authority for the USDT mint
/// - For testing purposes only
/// - Amount is in smallest units (6 decimals)
///
/// ## Endpoint
///
/// `POST /vault/mint-usdt`
///
/// ## Example
///
/// ```bash
/// curl -X POST http://127.0.0.1:8080/vault/mint-usdt \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userPubkey": "YOUR_WALLET_ADDRESS",
///     "amount": 1000000000
///   }'
/// ```
///
/// **Note:** This will automatically create the user's token account if it doesn't exist.
///
/// ## Request Body
///
/// ```json
/// {
///     "userPubkey": "YOUR_WALLET_ADDRESS",
///     "amount": 1000000000
/// }
/// ```
///
/// ## Response
///
/// ```json
/// {
///     "success": true,
///     "data": {
///         "signature": "5Ht3Rjabc...",
///         "amount": 1000000000,
///         "formattedAmount": "1000.00 USDT",
///         "userPubkey": "GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M",
///         "message": "Successfully minted 1000.00 USDT"
///     }
/// }
/// ```
pub async fn mint_usdt(
    state: web::Data<Arc<AppState>>,
    body: web::Json<MintUsdtRequest>,
) -> HttpResponse {
    info!(
        "Mint USDT request: {} USDT for {}",
        body.amount as f64 / 1_000_000.0,
        body.user_pubkey
    );

    // Validate amount
    if body.amount == 0 {
        return HttpResponse::BadRequest().json(
            ApiResponse::<()>::error("INVALID_AMOUNT", "Amount must be greater than 0")
        );
    }

    // Create token minter
    let minter = match TokenMinter::new(
        state.config.clone(),
    ) {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to create TokenMinter: {}", e);
            return HttpResponse::BadRequest().json(
                ApiResponse::<()>::error("MINTER_INIT_FAILED", &e.to_string())
            );
        }
    };

    // Mint tokens
    match minter.mint_usdt(&body.user_pubkey, body.amount).await {
        Ok(signature) => {
            let formatted_amount = format!("{:.2} USDT", body.amount as f64 / 1_000_000.0);
            
            let response = json!({
                "signature": signature,
                "amount": body.amount,
                "formattedAmount": formatted_amount,
                "userPubkey": body.user_pubkey,
                "message": format!("Successfully minted {}", formatted_amount)
            });

            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => {
            error!("Mint USDT failed: {}", e);
            
            let error_msg = e.to_string();
            let (code, message) = match &e {
                crate::services::token_minter::TokenMinterError::NotDevnet(_) => {
                    ("NOT_DEVNET", "Minting is only allowed on devnet")
                }
                crate::services::token_minter::TokenMinterError::InvalidPubkey(_) => {
                    ("INVALID_PUBKEY", error_msg.as_str())
                }
                crate::services::token_minter::TokenMinterError::MintError(_) => {
                    ("MINT_FAILED", error_msg.as_str())
                }
                _ => ("MINT_ERROR", error_msg.as_str())
            };
            
            HttpResponse::BadRequest().json(
                ApiResponse::<()>::error(code, message)
            )
        }
    }
}

