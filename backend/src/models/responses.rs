//! # API Response Models
//!
//! Structures for outgoing API response bodies.
//! All responses are wrapped in a standard format.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Standard API response wrapper.
///
/// All API responses follow this format:
///
/// ## Success Response
///
/// ```json
/// {
///     "success": true,
///     "data": { ... },
///     "error": null
/// }
/// ```
///
/// ## Error Response
///
/// ```json
/// {
///     "success": false,
///     "data": null,
///     "error": {
///         "code": "INSUFFICIENT_BALANCE",
///         "message": "Not enough available balance"
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T> {
    /// Whether the request was successful.
    pub success: bool,

    /// Response data (null on error).
    pub data: Option<T>,

    /// Error information (null on success).
    pub error: Option<ApiError>,
}

impl<T> ApiResponse<T> {
    /// Create a successful response with data.
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(code: &str, message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
            }),
        }
    }
}

/// API error information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// Error code (e.g., "INSUFFICIENT_BALANCE").
    pub code: String,

    /// Human-readable error message.
    pub message: String,
}

/// Vault balance response.
///
/// Returned by `GET /vault/balance/:user`
///
/// ## Example Response
///
/// ```json
/// {
///     "success": true,
///     "data": {
///         "owner": "7xKt9Fj2abc123...",
///         "vaultAddress": "9Yht3Mkxyz789...",
///         "totalBalance": 1000000000,
///         "lockedBalance": 200000000,
///         "availableBalance": 800000000,
///         "totalDeposited": 1500000000,
///         "totalWithdrawn": 500000000,
///         "formattedTotal": "1000.00 USDT",
///         "formattedAvailable": "800.00 USDT"
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultBalanceResponse {
    /// Vault owner's public key.
    pub owner: String,

    /// Vault PDA address.
    pub vault_address: String,

    /// Token account address.
    pub token_account: String,

    /// Total balance in smallest units.
    pub total_balance: i64,

    /// Locked balance in smallest units.
    pub locked_balance: i64,

    /// Available balance in smallest units.
    pub available_balance: i64,

    /// Lifetime total deposits.
    pub total_deposited: i64,

    /// Lifetime total withdrawals.
    pub total_withdrawn: i64,

    /// Human-readable total balance (e.g., "1000.00 USDT").
    pub formatted_total: String,

    /// Human-readable available balance.
    pub formatted_available: String,

    /// When the vault was created.
    pub created_at: DateTime<Utc>,

    /// Last update timestamp.
    pub last_updated: DateTime<Utc>,
}

impl VaultBalanceResponse {
    /// Format a balance value as human-readable USDT.
    /// 
    /// 1,000,000 smallest units = 1 USDT
    pub fn format_usdt(amount: i64) -> String {
        let usdt = amount as f64 / 1_000_000.0;
        format!("{:.2} USDT", usdt)
    }
}

/// Transaction record response.
///
/// Returned in transaction history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResponse {
    /// Transaction ID.
    pub id: Uuid,

    /// Transaction type.
    pub transaction_type: String,

    /// Amount in smallest units.
    pub amount: i64,

    /// Human-readable amount.
    pub formatted_amount: String,

    /// Solana transaction signature (if confirmed).
    pub signature: Option<String>,

    /// Transaction status.
    pub status: String,

    /// Balance before transaction.
    pub balance_before: i64,

    /// Balance after transaction.
    pub balance_after: i64,

    /// Counterparty (for transfers).
    pub counterparty: Option<String>,

    /// Transaction note.
    pub note: Option<String>,

    /// When transaction was created.
    pub created_at: DateTime<Utc>,

    /// When transaction was confirmed.
    pub confirmed_at: Option<DateTime<Utc>>,
}

/// Transaction list response with pagination.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionListResponse {
    /// List of transactions.
    pub transactions: Vec<TransactionResponse>,

    /// Total number of transactions (for pagination).
    pub total: i64,

    /// Current offset.
    pub offset: i64,

    /// Number of items returned.
    pub limit: i64,
}

/// Deposit/Withdraw operation response.
///
/// Returned after initiating a deposit or withdrawal.
///
/// ## Example Response
///
/// ```json
/// {
///     "success": true,
///     "data": {
///         "transactionId": "550e8400-e29b-41d4-a716-446655440000",
///         "status": "pending",
///         "unsignedTransaction": "base64encodedtx...",
///         "message": "Please sign and submit the transaction"
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResponse {
    /// Internal transaction ID (for tracking).
    pub transaction_id: Uuid,

    /// Current status: pending, confirmed, failed.
    pub status: String,

    /// Base64-encoded unsigned transaction (if applicable).
    /// Client should sign this and submit to Solana.
    pub unsigned_transaction: Option<String>,

    /// Solana transaction signature (if already submitted).
    pub signature: Option<String>,

    /// Human-readable status message.
    pub message: String,
}

/// Total Value Locked response.
///
/// Returned by `GET /vault/tvl`
///
/// ## Example Response
///
/// ```json
/// {
///     "success": true,
///     "data": {
///         "totalValueLocked": 50000000000,
///         "formattedTvl": "50,000.00 USDT",
///         "activeVaults": 1234,
///         "totalLocked": 10000000000,
///         "totalAvailable": 40000000000,
///         "timestamp": "2024-01-15T12:00:00Z"
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TvlResponse {
    /// Total USDT across all vaults.
    pub total_value_locked: i64,

    /// Human-readable TVL.
    pub formatted_tvl: String,

    /// Number of active vaults.
    pub active_vaults: i64,

    /// Total locked for positions.
    pub total_locked: i64,

    /// Total available for withdrawal.
    pub total_available: i64,

    /// When this data was calculated.
    pub timestamp: DateTime<Utc>,
}

/// Balance snapshot for history charts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSnapshotResponse {
    /// Total balance at this point.
    pub total_balance: i64,

    /// Locked balance at this point.
    pub locked_balance: i64,

    /// Available balance at this point.
    pub available_balance: i64,

    /// Timestamp of snapshot.
    pub timestamp: DateTime<Utc>,
}

/// Balance history response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceHistoryResponse {
    /// Vault owner.
    pub owner: String,

    /// List of snapshots.
    pub snapshots: Vec<BalanceSnapshotResponse>,

    /// Query start time.
    pub from: DateTime<Utc>,

    /// Query end time.
    pub to: DateTime<Utc>,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    /// Service status: "healthy" or "unhealthy".
    pub status: String,

    /// Database connection status.
    pub database: bool,

    /// Solana RPC connection status.
    pub solana_rpc: bool,

    /// Service version.
    pub version: String,

    /// Current timestamp.
    pub timestamp: DateTime<Utc>,
}

