//! # Configuration Module
//!
//! This module handles loading and validating configuration from
//! environment variables. All settings are centralized here.
//!
//! ## Usage
//!
//! ```rust,ignore
//! let config = AppConfig::from_env()?;
//! println!("RPC URL: {}", config.solana_rpc_url);
//! ```
//!
//! ## Environment Variables
//!
//! | Variable | Description | Example |
//! |----------|-------------|---------|
//! | `DATABASE_URL` | PostgreSQL connection string | `postgres://user:pass@localhost/db` |
//! | `SOLANA_RPC_URL` | Solana RPC endpoint | `https://api.devnet.solana.com` |
//! | `VAULT_PROGRAM_ID` | Deployed program ID | `AVRBwuFHdU51...` |
//! | `SERVER_HOST` | HTTP server host | `127.0.0.1` |
//! | `SERVER_PORT` | HTTP server port | `8080` |

use std::env;
use thiserror::Error;

/// Errors that can occur when loading configuration.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// A required environment variable is missing
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    /// An environment variable has an invalid value
    #[error("Invalid value for {0}: {1}")]
    InvalidValue(String, String),

    /// Failed to parse a value
    #[error("Failed to parse {0}: {1}")]
    ParseError(String, String),
}

/// Application configuration loaded from environment variables.
///
/// This struct contains all the settings needed to run the backend service.
/// Values are loaded from environment variables at startup.
///
/// ## Example
///
/// ```rust,ignore
/// // Load configuration from environment
/// let config = AppConfig::from_env()?;
///
/// // Access settings
/// println!("Database: {}", config.database_url);
/// println!("Solana RPC: {}", config.solana_rpc_url);
/// ```
#[derive(Debug, Clone)]
pub struct AppConfig {
    // ==========================================
    // DATABASE SETTINGS
    // ==========================================
    
    /// PostgreSQL connection URL.
    ///
    /// Format: `postgres://username:password@host:port/database`
    ///
    /// Example: `postgres://postgres:secret@localhost:5432/collateral_vault`
    pub database_url: String,

    // ==========================================
    // SOLANA SETTINGS
    // ==========================================
    
    /// Solana RPC endpoint URL.
    ///
    /// This is the HTTP endpoint used to send transactions
    /// and query blockchain state.
    ///
    /// Common values:
    /// - Devnet: `https://api.devnet.solana.com`
    /// - Mainnet: `https://api.mainnet-beta.solana.com`
    /// - Local: `http://localhost:8899`
    pub solana_rpc_url: String,

    /// Solana WebSocket endpoint URL.
    ///
    /// Used for real-time event subscriptions.
    ///
    /// Common values:
    /// - Devnet: `wss://api.devnet.solana.com`
    /// - Mainnet: `wss://api.mainnet-beta.solana.com`
    pub solana_ws_url: String,

    /// The deployed Collateral Vault program ID.
    ///
    /// This is the public key of your deployed Anchor program
    /// on Solana.
    pub vault_program_id: String,

    /// USDT token mint address.
    ///
    /// The SPL Token mint address for USDT on Solana.
    /// Different on devnet vs mainnet!
    pub usdt_mint: String,

    /// Path to the keypair file for signing transactions.
    ///
    /// The backend uses this keypair as its authority
    /// for administrative operations.
    pub keypair_path: String,

    // ==========================================
    // SERVER SETTINGS
    // ==========================================
    
    /// HTTP server host address.
    ///
    /// Use `127.0.0.1` for localhost only, `0.0.0.0` to accept
    /// connections from any interface.
    pub server_host: String,

    /// HTTP server port number.
    ///
    /// Default: 8080
    pub server_port: u16,

    // ==========================================
    // MONITORING SETTINGS
    // ==========================================
    
    /// How often to check vault balances (in seconds).
    ///
    /// The balance tracker runs this often to detect
    /// any discrepancies between on-chain and off-chain state.
    pub balance_check_interval: u64,

    /// How often to run full reconciliation (in seconds).
    ///
    /// Reconciliation compares all vault balances with
    /// the blockchain and logs any differences.
    pub reconciliation_interval: u64,

    /// Threshold for low balance alerts (in USDT).
    ///
    /// When a vault's available balance drops below this,
    /// an alert is triggered.
    pub low_balance_threshold: u64,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// This reads all required environment variables and validates them.
    /// Use `dotenvy::dotenv()` before calling this to load from `.env` file.
    ///
    /// ## Returns
    ///
    /// - `Ok(AppConfig)` - Configuration loaded successfully
    /// - `Err(ConfigError)` - A required variable is missing or invalid
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// dotenvy::dotenv().ok(); // Load .env file
    /// let config = AppConfig::from_env()?;
    /// ```
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            // Database
            database_url: get_env("DATABASE_URL")?,

            // Solana
            solana_rpc_url: get_env_or_default(
                "SOLANA_RPC_URL",
                "https://api.devnet.solana.com",
            ),
            solana_ws_url: get_env_or_default(
                "SOLANA_WS_URL",
                "wss://api.devnet.solana.com",
            ),
            vault_program_id: get_env("VAULT_PROGRAM_ID")?,
            usdt_mint: get_env_or_default(
                "USDT_MINT",
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
            ),
            keypair_path: get_env_or_default(
                "KEYPAIR_PATH",
                "~/.config/solana/id.json",
            ),

            // Server
            server_host: get_env_or_default("SERVER_HOST", "127.0.0.1"),
            server_port: get_env_or_default("SERVER_PORT", "8080")
                .parse()
                .map_err(|e| ConfigError::ParseError(
                    "SERVER_PORT".to_string(),
                    format!("{}", e),
                ))?,

            // Monitoring
            balance_check_interval: get_env_or_default("BALANCE_CHECK_INTERVAL", "30")
                .parse()
                .unwrap_or(30),
            reconciliation_interval: get_env_or_default("RECONCILIATION_INTERVAL", "300")
                .parse()
                .unwrap_or(300),
            low_balance_threshold: get_env_or_default("LOW_BALANCE_THRESHOLD", "100")
                .parse()
                .unwrap_or(100),
        })
    }
}

/// Get a required environment variable.
///
/// Returns an error if the variable is not set.
fn get_env(key: &str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::MissingEnvVar(key.to_string()))
}

/// Get an environment variable with a default value.
///
/// Returns the default if the variable is not set.
fn get_env_or_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_env_or_default() {
        // Should return default when not set
        let value = get_env_or_default("NONEXISTENT_VAR_12345", "default_value");
        assert_eq!(value, "default_value");
    }
}

