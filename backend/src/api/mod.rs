//! # REST API Module
//!
//! This module defines all HTTP endpoints for the Collateral Vault API.
//!
//! ## Endpoint Overview
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | POST | `/vault/initialize` | Create new vault |
//! | POST | `/vault/deposit` | Deposit USDT |
//! | POST | `/vault/withdraw` | Withdraw USDT |
//! | GET | `/vault/balance/:user` | Get vault balance |
//! | GET | `/vault/transactions/:user` | Transaction history |
//! | GET | `/vault/tvl` | Total Value Locked |
//! | GET | `/health` | Health check |
//!
//! ## Request/Response Format
//!
//! All requests and responses use JSON:
//!
//! ```json
//! // Success response
//! {
//!     "success": true,
//!     "data": { ... }
//! }
//!
//! // Error response
//! {
//!     "success": false,
//!     "error": {
//!         "code": "ERROR_CODE",
//!         "message": "Human readable message"
//!     }
//! }
//! ```

pub mod routes;
pub mod handlers;

pub use routes::configure_routes;

