//! # Database Models
//!
//! This module defines the data structures that map to database tables.
//! Each struct represents a row in a table.
//!
//! ## Table Overview
//!
//! | Table | Description |
//! |-------|-------------|
//! | `vaults` | Cached vault account data |
//! | `transactions` | All vault transactions (deposit, withdraw, etc.) |
//! | `balance_snapshots` | Periodic balance recordings |
//! | `reconciliation_logs` | Audit trail for balance checks |
//!
//! ## Relationship Diagram
//!
//! ```text
//! ┌─────────────┐       ┌──────────────────┐
//! │   vaults    │──────<│   transactions   │
//! │             │       │                  │
//! │ owner (PK)  │       │ vault_owner (FK) │
//! │ balance     │       │ type             │
//! │ ...         │       │ amount           │
//! └─────────────┘       └──────────────────┘
//!        │
//!        │
//!        ▼
//! ┌──────────────────┐
//! │balance_snapshots │
//! │                  │
//! │ vault_owner (FK) │
//! │ balance          │
//! │ timestamp        │
//! └──────────────────┘
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a vault record in the database.
///
/// This is a cached copy of the on-chain vault data.
/// It's updated whenever we detect changes on the blockchain.
///
/// ## Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | owner | String | User's wallet public key |
/// | vault_address | String | PDA address of the vault |
/// | token_account | String | Associated token account |
/// | total_balance | i64 | Total USDT in vault |
/// | locked_balance | i64 | Locked for positions |
/// | available_balance | i64 | Free to withdraw |
/// | total_deposited | i64 | Lifetime deposits |
/// | total_withdrawn | i64 | Lifetime withdrawals |
///
/// ## Note on Types
///
/// We use `i64` instead of `u64` because PostgreSQL doesn't have
/// unsigned integers. Values are always positive in practice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultRecord {
    /// The owner's wallet public key (base58 encoded).
    /// This is the primary key.
    pub owner: String,

    /// The vault's PDA address (base58 encoded).
    pub vault_address: String,

    /// The vault's token account address.
    pub token_account: String,

    /// Total USDT balance in the vault.
    /// In smallest units (6 decimals), so 1 USDT = 1,000,000.
    pub total_balance: i64,

    /// Amount locked for open trading positions.
    /// Cannot be withdrawn until positions are closed.
    pub locked_balance: i64,

    /// Amount available for withdrawal.
    /// Equals total_balance - locked_balance.
    pub available_balance: i64,

    /// Lifetime total of all deposits.
    /// Only increases, never decreases.
    pub total_deposited: i64,

    /// Lifetime total of all withdrawals.
    /// Only increases, never decreases.
    pub total_withdrawn: i64,

    /// When the vault was first created (on-chain).
    pub created_at: DateTime<Utc>,

    /// When this record was last updated in the database.
    pub updated_at: DateTime<Utc>,

    /// Current status of the vault.
    /// "active", "paused", or "closed"
    pub status: String,
}

/// Transaction types for vault operations.
///
/// Each variant represents a different operation that can happen
/// to a vault's balance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionType {
    /// User deposited USDT into vault
    Deposit,
    /// User withdrew USDT from vault
    Withdrawal,
    /// Collateral locked for trading position
    Lock,
    /// Collateral unlocked after position closed
    Unlock,
    /// Received from another vault (settlement win)
    TransferIn,
    /// Sent to another vault (settlement loss)
    TransferOut,
    /// Fees paid to protocol
    Fee,
}

impl ToString for TransactionType {
    fn to_string(&self) -> String {
        match self {
            TransactionType::Deposit => "deposit".to_string(),
            TransactionType::Withdrawal => "withdrawal".to_string(),
            TransactionType::Lock => "lock".to_string(),
            TransactionType::Unlock => "unlock".to_string(),
            TransactionType::TransferIn => "transfer_in".to_string(),
            TransactionType::TransferOut => "transfer_out".to_string(),
            TransactionType::Fee => "fee".to_string(),
        }
    }
}

/// Transaction status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    /// Transaction is being processed
    Pending,
    /// Transaction confirmed on blockchain
    Confirmed,
    /// Transaction failed
    Failed,
}

/// Represents a transaction record in the database.
///
/// Every vault operation (deposit, withdrawal, lock, etc.) creates
/// a transaction record for auditing and history.
///
/// ## Example
///
/// When a user deposits 100 USDT:
/// ```text
/// TransactionRecord {
///     id: "550e8400-e29b-41d4-a716-446655440000",
///     vault_owner: "7xKt9Fj2...",
///     transaction_type: Deposit,
///     amount: 100_000_000,  // 100 USDT with 6 decimals
///     signature: "5Ht3Rj...",
///     status: Confirmed,
///     ...
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    /// Unique transaction ID (UUID v4).
    pub id: Uuid,

    /// The vault owner's public key.
    pub vault_owner: String,

    /// Type of transaction.
    pub transaction_type: String,

    /// Amount involved (in smallest units, 6 decimals).
    pub amount: i64,

    /// Solana transaction signature (base58 encoded).
    /// Used to verify the transaction on-chain.
    pub signature: Option<String>,

    /// Current status of the transaction.
    pub status: String,

    /// Balance before this transaction.
    pub balance_before: i64,

    /// Balance after this transaction.
    pub balance_after: i64,

    /// For transfers: the counterparty vault owner.
    /// NULL for deposits/withdrawals.
    pub counterparty: Option<String>,

    /// Optional note or reason for the transaction.
    pub note: Option<String>,

    /// When the transaction was initiated.
    pub created_at: DateTime<Utc>,

    /// When the transaction was last updated.
    pub updated_at: DateTime<Utc>,

    /// When the transaction was confirmed on-chain.
    pub confirmed_at: Option<DateTime<Utc>>,
}

/// Balance snapshot for analytics and auditing.
///
/// We take periodic snapshots of vault balances to:
/// - Track balance history over time
/// - Generate charts and reports
/// - Detect anomalies
///
/// ## Snapshot Frequency
///
/// - Every 15 minutes for high-activity vaults
/// - Every hour for normal vaults
/// - Daily summary snapshots for all vaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    /// Unique snapshot ID.
    pub id: Uuid,

    /// The vault owner's public key.
    pub vault_owner: String,

    /// Total balance at snapshot time.
    pub total_balance: i64,

    /// Locked balance at snapshot time.
    pub locked_balance: i64,

    /// Available balance at snapshot time.
    pub available_balance: i64,

    /// When the snapshot was taken.
    pub timestamp: DateTime<Utc>,

    /// Type of snapshot: "periodic", "event", "daily_summary"
    pub snapshot_type: String,
}

/// Reconciliation log entry.
///
/// Records the results of balance reconciliation between
/// on-chain data and our database cache.
///
/// ## Why Reconciliation?
///
/// Our database is a cache of on-chain data. They can get out of sync:
/// - Backend was down while transactions happened
/// - Network issues caused missed events
/// - Manual on-chain operations bypassed the backend
///
/// Reconciliation detects and fixes these discrepancies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationLog {
    /// Unique log ID.
    pub id: Uuid,

    /// The vault being reconciled (NULL for system-wide).
    pub vault_owner: Option<String>,

    /// Balance we had in database.
    pub expected_balance: i64,

    /// Balance found on-chain.
    pub actual_balance: i64,

    /// Difference (actual - expected).
    /// Positive = we were under-counting.
    /// Negative = we were over-counting.
    pub difference: i64,

    /// Whether the discrepancy was automatically fixed.
    pub auto_fixed: bool,

    /// Notes about the reconciliation.
    pub notes: Option<String>,

    /// When reconciliation was performed.
    pub created_at: DateTime<Utc>,
}

/// Alert for monitoring purposes.
///
/// The system generates alerts for various conditions:
/// - Low balance
/// - Large transactions
/// - Failed transactions
/// - Reconciliation discrepancies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert ID.
    pub id: Uuid,

    /// Alert severity: "info", "warning", "critical"
    pub severity: String,

    /// Alert type: "low_balance", "large_tx", "failed_tx", etc.
    pub alert_type: String,

    /// The vault this alert relates to (if applicable).
    pub vault_owner: Option<String>,

    /// Human-readable alert message.
    pub message: String,

    /// Additional data as JSON.
    pub data: Option<serde_json::Value>,

    /// Whether the alert has been acknowledged.
    pub acknowledged: bool,

    /// When the alert was created.
    pub created_at: DateTime<Utc>,

    /// When the alert was acknowledged (if applicable).
    pub acknowledged_at: Option<DateTime<Utc>>,
}

/// Total Value Locked (TVL) snapshot.
///
/// Tracks the total amount of USDT locked across all vaults.
/// Used for protocol analytics and reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvlSnapshot {
    /// Unique snapshot ID.
    pub id: Uuid,

    /// Total USDT across all vaults.
    pub total_value_locked: i64,

    /// Number of active vaults.
    pub active_vaults: i64,

    /// Total locked for positions.
    pub total_locked: i64,

    /// Total available for withdrawal.
    pub total_available: i64,

    /// When the snapshot was taken.
    pub timestamp: DateTime<Utc>,
}

