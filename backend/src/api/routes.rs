//! # API Route Configuration
//!
//! This module sets up all the HTTP routes for the API.

use actix_web::web;

use super::handlers;

/// Configure all API routes.
///
/// This function is called from main.rs to set up
/// all the endpoint routes.
///
/// ## Route Structure
///
/// ```text
/// /
/// ├── /health              GET - Health check
/// └── /vault
///     ├── /initialize      POST - Create vault
///     ├── /deposit         POST - Deposit USDT
///     ├── /withdraw        POST - Withdraw USDT
///     ├── /balance/:user   GET - Get balance
///     ├── /transactions/:user  GET - Transaction history
///     └── /tvl             GET - Total Value Locked
/// ```
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Root endpoint - API information
        .route("/", web::get().to(handlers::api_info))
        
        // Health check endpoint
        .route("/health", web::get().to(handlers::health_check))
        
        // Vault endpoints
        .service(
            web::scope("/vault")
                // Initialize a new vault
                .route("/initialize", web::post().to(handlers::initialize_vault))
                
                // Deposit USDT into vault
                .route("/deposit", web::post().to(handlers::deposit))
                
                // Withdraw USDT from vault
                .route("/withdraw", web::post().to(handlers::withdraw))
                
                // Lock collateral (internal - position manager)
                .route("/lock-collateral", web::post().to(handlers::lock_collateral))
                
                // Unlock collateral (internal - position manager)
                .route("/unlock-collateral", web::post().to(handlers::unlock_collateral))
                
                // Transfer collateral (internal - settlement/liquidation)
                .route("/transfer-collateral", web::post().to(handlers::transfer_collateral))
                
                // Get vault balance
                .route("/balance/{user}", web::get().to(handlers::get_balance))
                
                // Get transaction history
                .route(
                    "/transactions/{user}",
                    web::get().to(handlers::get_transactions),
                )
                
                // Get Total Value Locked
                .route("/tvl", web::get().to(handlers::get_tvl))
                
                // Mint test USDT (devnet only)
                .route("/mint-usdt", web::post().to(handlers::mint_usdt))
        );
}

