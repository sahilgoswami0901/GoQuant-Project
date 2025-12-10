//! # Vault Monitor Service
//!
//! The VaultMonitor continuously monitors all vaults for:
//! - Unusual activity
//! - Security threats
//! - System health
//! - Analytics data
//!
//! ## Responsibilities
//!
//! - Detect unauthorized access attempts
//! - Alert on large transactions
//! - Track Total Value Locked (TVL)
//! - Monitor system performance
//! - Generate alerts for anomalies
//!
//! ## Monitoring Flow
//!
//! ```text
//! VaultMonitor (background task)
//!              │
//!              ├── Every 30s: Check balance thresholds
//!              │
//!              ├── Every 5m: Update TVL metrics
//!              │
//!              ├── Every 15m: Create snapshots
//!              │
//!              └── Real-time: Process blockchain events
//! ```

use std::time::Duration;
use chrono::Utc;
use tokio::time::interval;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::db::{Database, Alert, TvlSnapshot};
use crate::db::queries;
use crate::solana::SolanaClient;

/// The Vault Monitor service.
///
/// Runs in the background, continuously monitoring the
/// health and security of the vault system.
///
/// ## Usage
///
/// ```rust,ignore
/// let monitor = VaultMonitor::new(db, solana, config);
///
/// // Start monitoring (runs forever)
/// tokio::spawn(async move {
///     monitor.start().await;
/// });
/// ```
#[derive(Clone)]
pub struct VaultMonitor {
    /// Database connection.
    db: Database,

    /// Solana RPC client.
    solana: SolanaClient,

    /// Application configuration.
    config: AppConfig,
}

impl VaultMonitor {
    /// Create a new VaultMonitor instance.
    ///
    /// ## Arguments
    ///
    /// * `db` - Database connection
    /// * `solana` - Solana RPC client
    /// * `config` - Application configuration
    pub fn new(db: Database, solana: SolanaClient, config: AppConfig) -> Self {
        Self { db, solana, config }
    }

    /// Start the monitoring loop.
    ///
    /// This runs continuously, performing various checks at
    /// different intervals.
    ///
    /// ## Checks Performed
    ///
    /// | Check | Interval | Description |
    /// |-------|----------|-------------|
    /// | Balance Check | Configurable (default: 30s) | Check for low balances |
    /// | TVL Update | 5 minutes | Update TVL metrics |
    /// | Health Check | 2 minutes | Verify system health (Solana RPC connectivity) |
    pub async fn start(&self) {
        info!("Starting Vault Monitor service");

        // Different intervals for different checks
        let mut balance_ticker = interval(Duration::from_secs(
            self.config.balance_check_interval,
        ));
        let mut tvl_ticker = interval(Duration::from_secs(300)); // 5 minutes
        let mut health_ticker = interval(Duration::from_secs(120)); // 2 minutes (reduced frequency)

        loop {
            tokio::select! {
                // Balance check tick
                _ = balance_ticker.tick() => {
                    if let Err(e) = self.check_low_balances().await {
                        error!("Balance check failed: {}", e);
                    }
                }
                
                // TVL update tick
                _ = tvl_ticker.tick() => {
                    if let Err(e) = self.update_tvl_metrics().await {
                        error!("TVL update failed: {}", e);
                    }
                }
                
                // Health check tick
                _ = health_ticker.tick() => {
                    if let Err(e) = self.perform_health_check().await {
                        // Only log as warning - health check failures are expected with public RPCs
                        warn!("Health check operation failed: {}", e);
                    }
                }
            }
        }
    }

    /// Check all vaults for low balances.
    ///
    /// Creates alerts for vaults where available balance
    /// is below the configured threshold.
    async fn check_low_balances(&self) -> Result<(), String> {
        debug!("Checking for low balance vaults");

        let threshold = self.config.low_balance_threshold as i64 * 1_000_000;
        
        // Get all active vaults with low balance
        let vaults = queries::get_all_active_vaults(self.db.pool(), 1000, 0)
            .await
            .map_err(|e| e.to_string())?;

        let mut low_balance_count = 0;

        for vault in vaults {
            if vault.available_balance < threshold {
                low_balance_count += 1;
                
                // Create alert
                self.create_alert(
                    "warning",
                    "low_balance",
                    Some(&vault.owner),
                    &format!(
                        "Vault {} has low balance: {} USDT available",
                        vault.owner,
                        vault.available_balance as f64 / 1_000_000.0
                    ),
                ).await?;
            }
        }

        if low_balance_count > 0 {
            info!("Found {} vaults with low balance", low_balance_count);
        }

        Ok(())
    }

    /// Update Total Value Locked metrics.
    ///
    /// Calculates current TVL and stores a snapshot.
    async fn update_tvl_metrics(&self) -> Result<(), String> {
        debug!("Updating TVL metrics");

        let (total_tvl, total_locked, total_available, vault_count) = queries::get_current_tvl(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;

        // Log the metrics
        info!(
            "TVL: {} USDT (Locked: {}, Available: {}) | Active Vaults: {}",
            total_tvl as f64 / 1_000_000.0,
            total_locked as f64 / 1_000_000.0,
            total_available as f64 / 1_000_000.0,
            vault_count
        );

        // Create snapshot
        let snapshot = TvlSnapshot {
            id: Uuid::new_v4(),
            total_value_locked: total_tvl,
            active_vaults: vault_count,
            total_locked,
            total_available,
            timestamp: Utc::now(),
        };

        queries::create_tvl_snapshot(self.db.pool(), &snapshot)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Perform system health check.
    ///
    /// Verifies that all system components are working correctly.
    async fn perform_health_check(&self) -> Result<(), String> {
        debug!("Performing health check");

        // Check database connection
        let db_healthy = self.check_database_health().await;
        
        // Check Solana RPC connection
        let solana_healthy = self.check_solana_health().await;

        if !db_healthy {
            self.create_alert(
                "critical",
                "database_unhealthy",
                None,
                "Database connection is unhealthy",
            ).await?;
        }

        if !solana_healthy {
            self.create_alert(
                "critical",
                "solana_unhealthy",
                None,
                "Solana RPC connection is unhealthy",
            ).await?;
        }

        if db_healthy && solana_healthy {
            debug!("Health check passed: all systems operational");
        }

        Ok(())
    }

    /// Check if database is healthy.
    async fn check_database_health(&self) -> bool {
        // Simple query to check connection
        match queries::get_vault_count(self.db.pool()).await {
            Ok(_) => true,
            Err(e) => {
                error!("Database health check failed: {}", e);
                false
            }
        }
    }

    /// Check if Solana RPC is healthy.
    async fn check_solana_health(&self) -> bool {
        match self.solana.get_health().await {
            Ok(healthy) => healthy,
            Err(e) => {
                // Use debug level - get_health already logs warnings for failures
                debug!("Solana health check returned error: {}", e);
                false
            }
        }
    }

    /// Create an alert.
    ///
    /// Alerts are stored in the database for the admin dashboard
    /// and can trigger notifications (email, Slack, etc.).
    ///
    /// ## Arguments
    ///
    /// * `severity` - "info", "warning", or "critical"
    /// * `alert_type` - Type identifier (e.g., "low_balance")
    /// * `vault_owner` - Related vault (if applicable)
    /// * `message` - Human-readable message
    async fn create_alert(
        &self,
        severity: &str,
        alert_type: &str,
        vault_owner: Option<&str>,
        message: &str,
    ) -> Result<(), String> {
        let _alert = Alert {
            id: Uuid::new_v4(),
            severity: severity.to_string(),
            alert_type: alert_type.to_string(),
            vault_owner: vault_owner.map(|s| s.to_string()),
            message: message.to_string(),
            data: None,
            acknowledged: false,
            created_at: Utc::now(),
            acknowledged_at: None,
        };

        // Log the alert
        match severity {
            "critical" => error!("ALERT [{}]: {}", alert_type, message),
            "warning" => warn!("ALERT [{}]: {}", alert_type, message),
            _ => info!("ALERT [{}]: {}", alert_type, message),
        }

        // TODO: Store in database
        // queries::create_alert(self.db.pool(), &alert).await?;

        // TODO: Send notifications (email, Slack, etc.)

        Ok(())
    }

    /// Monitor a large transaction.
    ///
    /// Called when a transaction exceeds a certain threshold.
    /// Creates an alert for review.
    #[allow(dead_code)]
    pub async fn monitor_large_transaction(
        &self,
        vault_owner: &str,
        amount: u64,
        tx_type: &str,
    ) -> Result<(), String> {
        // Define threshold (e.g., 10,000 USDT)
        let large_tx_threshold = 10_000_000_000u64; // 10,000 USDT

        if amount >= large_tx_threshold {
            self.create_alert(
                "info",
                "large_transaction",
                Some(vault_owner),
                &format!(
                    "Large {} detected: {} USDT for vault {}",
                    tx_type,
                    amount as f64 / 1_000_000.0,
                    vault_owner
                ),
            ).await?;
        }

        Ok(())
    }

    /// Get current system status.
    ///
    /// Returns a summary of system health for the API.
    #[allow(dead_code)]
    pub async fn get_system_status(&self) -> SystemStatus {
        let db_healthy = self.check_database_health().await;
        let solana_healthy = self.check_solana_health().await;

        let (tvl, _, _, vault_count) = queries::get_current_tvl(self.db.pool())
            .await
            .unwrap_or((0, 0, 0, 0));

        SystemStatus {
            healthy: db_healthy && solana_healthy,
            database_connected: db_healthy,
            solana_connected: solana_healthy,
            total_value_locked: tvl,
            active_vaults: vault_count,
            last_check: Utc::now(),
        }
    }
}

/// System status information.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SystemStatus {
    /// Whether all systems are healthy.
    pub healthy: bool,

    /// Database connection status.
    pub database_connected: bool,

    /// Solana RPC connection status.
    pub solana_connected: bool,

    /// Total Value Locked in USDT (smallest units).
    pub total_value_locked: i64,

    /// Number of active vaults.
    pub active_vaults: i64,

    /// When this status was generated.
    pub last_check: chrono::DateTime<Utc>,
}

