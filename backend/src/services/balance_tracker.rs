//! # Balance Tracker Service
//!
//! The BalanceTracker monitors vault balances in real-time and ensures
//! consistency between on-chain state and the database cache.
//!
//! ## Responsibilities
//!
//! - Monitor vault balances in real-time
//! - Detect discrepancies between database and blockchain
//! - Auto-reconcile when possible
//! - Generate alerts for significant issues
//! - Track Total Value Locked (TVL)
//!
//! ## Reconciliation Flow
//!
//! ```text
//! Every N seconds:
//! 1. Get all vaults from database
//!               ↓
//! 2. For each vault, query blockchain
//!               ↓
//! 3. Compare database balance vs on-chain balance
//!               ↓
//! 4. If different:
//!    a. Log the discrepancy
//!    b. Update database to match blockchain (source of truth)
//!    c. Generate alert if difference is significant
//! ```
//!
//! ## Why Reconciliation?
//!
//! The database is a **cache** of blockchain state. It can become stale:
//! - Backend was down during transactions
//! - Network issues caused missed events
//! - Direct on-chain transactions bypassed the backend
//!
//! The blockchain is always the **source of truth**.

use std::time::Duration;
use chrono::Utc;
use tokio::time::interval;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::db::{Database, BalanceSnapshot, ReconciliationLog, TvlSnapshot};
use crate::db::queries;
use crate::solana::SolanaClient;

/// The Balance Tracker service.
///
/// Monitors vault balances and ensures database cache
/// is synchronized with blockchain state.
///
/// ## Usage
///
/// ```rust,ignore
/// let tracker = BalanceTracker::new(db, solana, config);
///
/// // Start background reconciliation
/// tracker.start_reconciliation_loop().await;
///
/// // Or manually reconcile a specific vault
/// tracker.reconcile_vault("7xKt9Fj2...").await?;
/// ```
#[derive(Clone)]
pub struct BalanceTracker {
    /// Database connection.
    db: Database,

    /// Solana RPC client.
    solana: SolanaClient,

    /// Application configuration.
    config: AppConfig,
}

impl BalanceTracker {
    /// Create a new BalanceTracker instance.
    ///
    /// ## Arguments
    ///
    /// * `db` - Database connection
    /// * `solana` - Solana RPC client
    /// * `config` - Application configuration
    pub fn new(db: Database, solana: SolanaClient, config: AppConfig) -> Self {
        Self { db, solana, config }
    }

    /// Start the background reconciliation loop.
    ///
    /// This runs continuously, periodically checking all vaults
    /// for discrepancies with the blockchain.
    ///
    /// ## Configuration
    ///
    /// The interval is controlled by `config.reconciliation_interval`
    /// (default: 300 seconds = 5 minutes).
    ///
    /// ## Note
    ///
    /// This should be spawned as a background task:
    ///
    /// ```rust,ignore
    /// let tracker = BalanceTracker::new(...);
    /// tokio::spawn(async move {
    ///     tracker.start_reconciliation_loop().await;
    /// });
    /// ```
    pub async fn start_reconciliation_loop(&self) {
        info!(
            "Starting balance reconciliation loop (interval: {}s)",
            self.config.reconciliation_interval
        );

        let mut ticker = interval(Duration::from_secs(self.config.reconciliation_interval));

        loop {
            ticker.tick().await;
            
            info!("Running scheduled reconciliation...");
            
            if let Err(e) = self.reconcile_all_vaults().await {
                error!("Reconciliation failed: {}", e);
            }

            // Also update TVL snapshot
            if let Err(e) = self.update_tvl_snapshot().await {
                error!("TVL snapshot failed: {}", e);
            }
        }
    }

    /// Reconcile all active vaults.
    ///
    /// Fetches all vaults from the database and compares
    /// each one against the blockchain state.
    ///
    /// ## Process
    ///
    /// 1. Get all active vaults from database (paginated)
    /// 2. For each vault, call `reconcile_vault()`
    /// 3. Log summary of discrepancies found
    pub async fn reconcile_all_vaults(&self) -> Result<(), String> {
        info!("Starting full reconciliation of all vaults");

        let mut offset = 0;
        let limit = 100; // Process in batches
        let mut total_checked = 0;
        let mut total_discrepancies = 0;

        loop {
            // Get batch of vaults
            let vaults = queries::get_all_active_vaults(self.db.pool(), limit, offset)
                .await
                .map_err(|e| e.to_string())?;

            if vaults.is_empty() {
                break;
            }

            // Check each vault
            for vault in &vaults {
                match self.reconcile_vault(&vault.owner).await {
                    Ok(had_discrepancy) => {
                        if had_discrepancy {
                            total_discrepancies += 1;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to reconcile vault {}: {}", vault.owner, e);
                    }
                }
                total_checked += 1;
            }

            offset += limit;
            
            // Small delay between batches to avoid overwhelming RPC
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        info!(
            "Reconciliation complete: checked {} vaults, found {} discrepancies",
            total_checked, total_discrepancies
        );

        Ok(())
    }

    /// Reconcile a single vault.
    ///
    /// Compares the database cached balance with the actual
    /// on-chain balance and updates if different.
    ///
    /// ## Arguments
    ///
    /// * `owner` - Vault owner's public key
    ///
    /// ## Returns
    ///
    /// * `Ok(true)` - Discrepancy found and fixed
    /// * `Ok(false)` - Balances match, no action needed
    /// * `Err(...)` - Error during reconciliation
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let had_discrepancy = tracker.reconcile_vault("7xKt9Fj2...").await?;
    /// if had_discrepancy {
    ///     println!("Balance was corrected");
    /// }
    /// ```
    pub async fn reconcile_vault(&self, owner: &str) -> Result<bool, String> {
        debug!("Reconciling vault: {}", owner);

        // Get cached balance from database
        let db_vault = queries::get_vault_by_owner(self.db.pool(), owner)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Vault not found in database: {}", owner))?;

        // Get actual balance from blockchain
        let chain_data = self
            .solana
            .get_vault_account(owner)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Vault not found on-chain: {}", owner))?;

        // Compare balances
        let db_balance = db_vault.total_balance;
        let chain_balance = chain_data.total_balance as i64;
        let difference = chain_balance - db_balance;

        if difference != 0 {
            warn!(
                "Balance discrepancy for {}: DB={}, Chain={}, Diff={}",
                owner, db_balance, chain_balance, difference
            );

            // Update database to match blockchain (source of truth)
            queries::update_vault_balance(
                self.db.pool(),
                owner,
                chain_data.total_balance as i64,
                chain_data.locked_balance as i64,
                chain_data.available_balance as i64,
            )
            .await
            .map_err(|e| e.to_string())?;

            // Log the reconciliation
            let log = ReconciliationLog {
                id: Uuid::new_v4(),
                vault_owner: Some(owner.to_string()),
                expected_balance: db_balance,
                actual_balance: chain_balance,
                difference,
                auto_fixed: true,
                notes: Some(format!(
                    "Auto-corrected from {} to {}",
                    db_balance, chain_balance
                )),
                created_at: Utc::now(),
            };

            queries::create_reconciliation_log(self.db.pool(), &log)
                .await
                .map_err(|e| e.to_string())?;

            info!("Vault {} balance corrected: {} -> {}", owner, db_balance, chain_balance);
            
            return Ok(true);
        }

        debug!("Vault {} balance matches: {}", owner, db_balance);
        Ok(false)
    }

    /// Create a balance snapshot for a vault.
    ///
    /// Snapshots are used for:
    /// - Historical balance charts
    /// - Audit trail
    /// - Anomaly detection
    ///
    /// ## Arguments
    ///
    /// * `owner` - Vault owner's public key
    /// * `snapshot_type` - Type: "periodic", "event", "daily_summary"
    pub async fn create_snapshot(
        &self,
        owner: &str,
        snapshot_type: &str,
    ) -> Result<(), String> {
        let vault = queries::get_vault_by_owner(self.db.pool(), owner)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Vault not found: {}", owner))?;

        let snapshot = BalanceSnapshot {
            id: Uuid::new_v4(),
            vault_owner: owner.to_string(),
            total_balance: vault.total_balance,
            locked_balance: vault.locked_balance,
            available_balance: vault.available_balance,
            timestamp: Utc::now(),
            snapshot_type: snapshot_type.to_string(),
        };

        queries::create_balance_snapshot(self.db.pool(), &snapshot)
            .await
            .map_err(|e| e.to_string())?;

        debug!("Created {} snapshot for vault {}", snapshot_type, owner);
        Ok(())
    }

    /// Update Total Value Locked (TVL) snapshot.
    ///
    /// Calculates current TVL across all vaults and stores
    /// a snapshot for analytics.
    pub async fn update_tvl_snapshot(&self) -> Result<(), String> {
        debug!("Updating TVL snapshot");

        // Get aggregate stats with breakdown
        let (total_tvl, total_locked, total_available, vault_count) = queries::get_current_tvl(self.db.pool())
            .await
            .map_err(|e| e.to_string())?;

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

        info!(
            "TVL snapshot: {} USDT (Locked: {}, Available: {}) across {} vaults",
            total_tvl as f64 / 1_000_000.0,
            total_locked as f64 / 1_000_000.0,
            total_available as f64 / 1_000_000.0,
            vault_count
        );

        Ok(())
    }

    /// Get current TVL with breakdown.
    ///
    /// Returns the total USDT locked across all vaults with locked/available breakdown.
    pub async fn get_tvl(&self) -> Result<(i64, i64, i64, i64), String> {
        queries::get_current_tvl(self.db.pool())
            .await
            .map_err(|e| e.to_string())
    }

    /// Check if a vault has low balance.
    ///
    /// Compares available balance against configured threshold.
    /// Used for alerting.
    ///
    /// ## Arguments
    ///
    /// * `owner` - Vault owner's public key
    ///
    /// ## Returns
    ///
    /// * `Ok(true)` - Balance is below threshold
    /// * `Ok(false)` - Balance is healthy
    pub async fn is_low_balance(&self, owner: &str) -> Result<bool, String> {
        let vault = queries::get_vault_by_owner(self.db.pool(), owner)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Vault not found: {}", owner))?;

        // Threshold is in USDT, vault balance is in smallest units
        let threshold_units = self.config.low_balance_threshold * 1_000_000;
        
        Ok(vault.available_balance < threshold_units as i64)
    }
}

