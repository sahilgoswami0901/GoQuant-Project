//! # Collateral Vault Backend Service
//!
//! This is the main entry point for the backend service that manages
//! the Collateral Vault system. It provides:
//!
//! - REST API for user interactions (deposit, withdraw, balance queries)
//! - WebSocket connections for real-time updates
//! - Background services for monitoring and reconciliation
//! - Database storage for transaction history
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        BACKEND SERVICE                           │
//! │                                                                  │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
//! │  │  REST API   │  │  WebSocket  │  │   Background Services   │  │
//! │  │  (Actix)    │  │  Server     │  │  • Vault Monitor        │  │
//! │  │             │  │             │  │  • Balance Tracker      │  │
//! │  │  /deposit   │  │  /ws        │  │  • Reconciliation       │  │
//! │  │  /withdraw  │  │             │  │                         │  │
//! │  │  /balance   │  │             │  │                         │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
//! │         │                │                     │                 │
//! │         └────────────────┴─────────────────────┘                 │
//! │                          │                                       │
//! │  ┌───────────────────────┴───────────────────────────────────┐  │
//! │  │                    SERVICE LAYER                           │  │
//! │  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────────┐   │  │
//! │  │  │VaultManager  │ │TxBuilder     │ │BalanceTracker    │   │  │
//! │  │  └──────────────┘ └──────────────┘ └──────────────────┘   │  │
//! │  └───────────────────────────────────────────────────────────┘  │
//! │                          │                                       │
//! │         ┌────────────────┴────────────────┐                     │
//! │         │                                 │                      │
//! │  ┌──────┴──────┐                   ┌──────┴──────┐              │
//! │  │  PostgreSQL │                   │   Solana    │              │
//! │  │  Database   │                   │   RPC       │              │
//! │  └─────────────┘                   └─────────────┘              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! 1. Set up PostgreSQL and create the database
//! 2. Copy `.env.example` to `.env` and configure
//! 3. Run migrations: `sqlx migrate run`
//! 4. Start the server: `cargo run`
//!
//! ## Environment Variables
//!
//! See `.env.example` for all required configuration.

use std::sync::Arc;
use actix_web::{web, App, HttpServer, middleware};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod api;
mod config;
mod db;
mod models;
mod services;
mod solana;
mod utils;
mod websocket;

use config::AppConfig;
use db::Database;
use services::{VaultManager, BalanceTracker, VaultMonitor};
use solana::SolanaClient;
use websocket::WsRegistry;

/// Application state shared across all handlers.
///
/// This struct contains all the shared resources that API handlers
/// and background services need access to.
///
/// ## Why Arc?
/// `Arc` (Atomic Reference Counting) allows us to share ownership
/// of these resources across multiple threads safely.
pub struct AppState {
    /// Database connection pool for PostgreSQL
    pub db: Database,
    
    /// Solana RPC client for blockchain interactions
    pub solana: SolanaClient,
    
    /// Vault management service
    pub vault_manager: VaultManager,
    
    /// Balance tracking service
    pub balance_tracker: BalanceTracker,
    
    /// Application configuration
    pub config: AppConfig,
    
    /// WebSocket connection registry for real-time updates
    pub ws_registry: WsRegistry,
}

/// Main entry point for the backend service.
///
/// This function:
/// 1. Loads configuration from environment
/// 2. Initializes database connection
/// 3. Sets up Solana client
/// 4. Starts background monitoring services
/// 5. Launches the HTTP server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // =========================================
    // STEP 1: Initialize Logging
    // =========================================
    // Set up structured logging with tracing
    // This gives us nice formatted logs with timestamps
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("🚀 Starting Collateral Vault Backend Service");

    // =========================================
    // STEP 2: Load Configuration
    // =========================================
    // Load from environment variables (from .env file)
    dotenvy::dotenv().ok(); // It's okay if .env doesn't exist
    
    let config = AppConfig::from_env()
        .expect("Failed to load configuration");
    
    info!("📋 Configuration loaded");
    info!("   Solana RPC: {}", config.solana_rpc_url);
    info!("   Program ID: {}", config.vault_program_id);

    // =========================================
    // STEP 3: Initialize Database
    // =========================================
    let db = Database::connect(&config.database_url)
        .await
        .expect("Failed to connect to database");
    
    info!("🗄️  Database connected");

    // Run migrations to ensure schema is up to date
    db.run_migrations()
        .await
        .expect("Failed to run migrations");
    
    info!("📦 Database migrations complete");

    // =========================================
    // STEP 4: Initialize Solana Client
    // =========================================
    let solana = SolanaClient::new(&config)
        .expect("Failed to create Solana client");
    
    info!("⛓️  Solana client initialized");

    // =========================================
    // STEP 5: Initialize Services
    // =========================================
    let vault_manager = VaultManager::new(
        db.clone(),
        solana.clone(),
        config.clone(),
    );
    
    let balance_tracker = BalanceTracker::new(
        db.clone(),
        solana.clone(),
        config.clone(),
    );

    info!("🔧 Services initialized");

    // =========================================
    // STEP 6: Initialize WebSocket Registry
    // =========================================
    let ws_registry = WsRegistry::new();
    info!("🔌 WebSocket registry initialized");

    // =========================================
    // STEP 7: Create Application State
    // =========================================
    let app_state = Arc::new(AppState {
        db: db.clone(),
        solana: solana.clone(),
        vault_manager,
        balance_tracker: balance_tracker.clone(),
        config: config.clone(),
        ws_registry: ws_registry.clone(),
    });

    // =========================================
    // STEP 8: Start Background Services
    // =========================================
    // Clone for background task
    let monitor_state = app_state.clone();
    
    // Spawn vault monitor in background
    tokio::spawn(async move {
        let monitor = VaultMonitor::new(
            monitor_state.db.clone(),
            monitor_state.solana.clone(),
            monitor_state.config.clone(),
        );
        monitor.start().await;
    });
    
    info!("👁️  Vault monitor started");

    // Start balance tracker background reconciliation
    let tracker_clone = balance_tracker.clone();
    tokio::spawn(async move {
        tracker_clone.start_reconciliation_loop().await;
    });
    
    info!("📊 Balance tracker started");

    // =========================================
    // STEP 8: Start HTTP Server
    // =========================================
    let server_host = config.server_host.clone();
    let server_port = config.server_port;

    info!("🌐 Starting HTTP server on {}:{}", server_host, server_port);

    HttpServer::new(move || {
        App::new()
            // Attach shared application state
            .app_data(web::Data::new(app_state.clone()))
            
            // Add logging middleware
            .wrap(middleware::Logger::default())
            
            // Configure API routes
            .configure(api::configure_routes)
            
            // Configure WebSocket routes
            .configure(websocket::configure_routes)
    })
    .bind(format!("{}:{}", server_host, server_port))?
    .run()
    .await
}

