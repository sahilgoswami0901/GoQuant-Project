//! # Database Module
//!
//! This module handles all database operations for the Collateral Vault backend.
//! We use PostgreSQL for storing:
//!
//! - Vault account records (cached from blockchain)
//! - Transaction history (deposits, withdrawals, locks, etc.)
//! - Balance snapshots (for analytics)
//! - Reconciliation logs (for auditing)
//!
//! ## Why Store Off-Chain?
//!
//! Even though all vault data is on the Solana blockchain, we store it
//! locally for several reasons:
//!
//! 1. **Speed**: Database queries are ~10ms, blockchain queries are ~200ms
//! 2. **History**: Blockchain doesn't store full transaction history easily
//! 3. **Analytics**: Complex queries are impossible on-chain
//! 4. **Reliability**: Works even if Solana RPC is slow/unavailable
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      DATABASE LAYER                              │
//! │                                                                  │
//! │  ┌──────────────────────────────────────────────────────────┐   │
//! │  │                   Connection Pool                         │   │
//! │  │                  (deadpool-postgres)                    │   │
//! │  └──────────────────────────────────────────────────────────┘   │
//! │                              │                                   │
//! │         ┌────────────────────┼────────────────────┐             │
//! │         ▼                    ▼                    ▼             │
//! │  ┌────────────┐      ┌────────────┐       ┌────────────┐       │
//! │  │  Vaults    │      │Transactions│       │ Snapshots  │       │
//! │  │  Table     │      │   Table    │       │   Table    │       │
//! │  └────────────┘      └────────────┘       └────────────┘       │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod models;
pub mod queries;

use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::{NoTls, Config as TokioConfig};
use thiserror::Error;
use tracing::{debug, info, warn, error};

/// Database-related errors.
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Failed to connect to the database
    #[error("Database connection failed: {0}")]
    ConnectionError(String),

    /// Query execution failed
    #[error("Query failed: {0}")]
    QueryError(#[from] tokio_postgres::Error),

    /// Migration failed
    #[error("Migration failed: {0}")]
    MigrationError(String),

    /// Record not found
    #[error("Record not found: {0}")]
    NotFound(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

/// Database connection wrapper.
///
/// This struct wraps the connection pool and provides
/// methods for common database operations.
///
/// ## Connection Pooling
///
/// We use deadpool-postgres for connection pooling.
/// This efficiently manages database connections.
///
/// ## Usage
///
/// ```rust,ignore
/// let db = Database::connect("postgres://...").await?;
/// let vault = queries::get_vault_by_owner(db.pool(), "pubkey").await?;
/// ```
#[derive(Clone)]
pub struct Database {
    /// The connection pool
    pool: Pool,
}

impl Database {
    /// Connect to the PostgreSQL database.
    ///
    /// Creates a connection pool with sensible defaults:
    /// - Max 10 connections
    /// - 30 second connection timeout
    ///
    /// ## Arguments
    ///
    /// * `database_url` - PostgreSQL connection string
    ///
    /// ## Returns
    ///
    /// * `Ok(Database)` - Connected successfully
    /// * `Err(DatabaseError)` - Connection failed
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let db = Database::connect("postgres://postgres:password@localhost/vault").await?;
    /// ```
    pub async fn connect(database_url: &str) -> Result<Self, DatabaseError> {
        info!("Connecting to database...");

        // Parse the connection string using tokio_postgres::Config
        let tokio_config = database_url.parse::<TokioConfig>()
            .map_err(|e| DatabaseError::ConfigError(format!("Invalid database URL: {}", e)))?;

        // Convert to deadpool config
        let mut config = Config::new();
        
        if let Some(dbname) = tokio_config.get_dbname() {
            config.dbname = Some(dbname.to_string());
        }
        if let Some(user) = tokio_config.get_user() {
            config.user = Some(user.to_string());
        }
        if let Some(password) = tokio_config.get_password() {
            // Password is &[u8], convert to String
            config.password = Some(String::from_utf8_lossy(password).to_string());
        }
        if let Some(host) = tokio_config.get_hosts().first() {
            if let tokio_postgres::config::Host::Tcp(host_str) = host {
                config.host = Some(host_str.clone());
            }
        }
        if let Some(port) = tokio_config.get_ports().first() {
            config.port = Some(*port);
        }
        
        // Set pool size
        config.pool = Some(deadpool_postgres::PoolConfig {
            max_size: 10,
            ..Default::default()
        });

        // Create pool
        let pool = config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        // Test connection
        let client = pool.get().await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        // Simple query to verify connection
        client.query("SELECT 1", &[]).await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        info!("Database connection established");

        Ok(Self { pool })
    }

    /// Run database migrations.
    ///
    /// Migrations are SQL scripts that set up and update the database schema.
    /// They're run in order and tracked so each only runs once.
    ///
    /// ## Migration Files
    ///
    /// Located in `migrations/` directory:
    /// ```text
    /// migrations/
    /// ├── 001_initial_schema.sql
    /// └── ...
    /// ```
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// db.run_migrations().await?;
    /// ```
    pub async fn run_migrations(&self) -> Result<(), DatabaseError> {
        info!("Running database migrations...");

        let client = self.pool.get().await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        // Get current working directory for debugging
        let current_dir = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        debug!("Current working directory: {}", current_dir);

        // Read migration file (try multiple possible paths)
        let migration_paths = [
            "migrations/001_initial_schema.sql",
            "../migrations/001_initial_schema.sql",
            "backend/migrations/001_initial_schema.sql",
            "./backend/migrations/001_initial_schema.sql",
        ];

        let mut migration_sql = None;
        let mut found_path = None;
        for path in &migration_paths {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    migration_sql = Some(content);
                    found_path = Some(path.to_string());
                    info!("Found migration file at: {}", path);
                    break;
                }
                Err(e) => {
                    debug!("Tried path '{}': {}", path, e);
                }
            }
        }

        let migration_sql = migration_sql
            .ok_or_else(|| {
                error!("Could not find migration file. Current dir: {}. Tried paths: {:?}", current_dir, migration_paths);
                DatabaseError::MigrationError(
                    format!("Could not find migration file. Current directory: {}. Tried paths: {:?}", current_dir, migration_paths)
                )
            })?;

        info!("Found migration file at: {}, executing SQL...", found_path.unwrap_or_default());

        // Clean SQL: Remove comments but preserve structure
        // We need to be careful with $$ delimiters in functions
        let cleaned_sql: String = migration_sql
            .lines()
            .map(|line| {
                // Remove single-line comments (-- comments)
                // But preserve lines that are part of function definitions
                if let Some(comment_pos) = line.find("--") {
                    // Don't remove if it's inside a $$ block (simple heuristic)
                    if line.trim().starts_with("--") {
                        String::new() // Full comment line, remove it
                    } else {
                        line[..comment_pos].trim().to_string()
                    }
                } else {
                    line.trim().to_string()
                }
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        info!("Executing migration SQL ({} bytes)...", cleaned_sql.len());

        // Execute the entire SQL file as one batch
        // PostgreSQL's batch_execute handles multiple statements correctly,
        // including functions with $$ delimiters
        match client.batch_execute(&cleaned_sql).await {
            Ok(_) => {
                info!("Migrations completed successfully");
                Ok(())
            }
            Err(e) => {
                // Extract detailed error information
                let error_msg = e.to_string();
                let error_code = e.code();
                let error_detail = e.as_db_error()
                    .and_then(|db_err| db_err.detail())
                    .unwrap_or("No detail available");
                let error_hint = e.as_db_error()
                    .and_then(|db_err| db_err.hint())
                    .unwrap_or("No hint available");
                
                error!("Migration execution error:");
                error!("  Error: {}", error_msg);
                if let Some(code) = error_code {
                    error!("  Code: {}", code.code());
                }
                error!("  Detail: {}", error_detail);
                error!("  Hint: {}", error_hint);
                
                // Check if error is about objects already existing
                // PostgreSQL error codes:
                // 42P07 = duplicate_table
                // 42710 = duplicate_object (for functions, triggers, etc.)
                let is_duplicate_error = error_code
                    .map(|code| {
                        let code_str = code.code();
                        code_str == "42P07" || code_str == "42710"
                    })
                    .unwrap_or(false);
                
                if error_msg.contains("already exists") || 
                   error_msg.contains("duplicate") ||
                   (error_msg.contains("relation") && error_msg.contains("already exists")) ||
                   is_duplicate_error {
                    warn!("Some database objects may already exist (error code: {:?}). This is OK if migrations were run before.", 
                          error_code.map(|c| c.code()));
                    info!("Migrations completed (some objects may already exist)");
                    Ok(())
                } else {
                    // Show first 1000 chars of SQL for debugging
                    let sql_preview: String = cleaned_sql
                        .chars()
                        .take(1000)
                        .collect();
                    error!("SQL preview (first 1000 chars):\n{}", sql_preview);
                    
                    Err(DatabaseError::MigrationError(format!(
                        "Migration execution failed:\n  Error: {}\n  Code: {:?}\n  Detail: {}\n  Hint: {}\n  SQL size: {} bytes",
                        error_msg,
                        error_code.map(|c| c.code()),
                        error_detail,
                        error_hint,
                        cleaned_sql.len()
                    )))
                }
            }
        }
    }

    /// Get a reference to the connection pool.
    ///
    /// Use this when you need direct access to the pool
    /// for custom queries.
    pub fn pool(&self) -> &Pool {
        &self.pool
    }
}

// Re-export commonly used items
pub use models::*;
