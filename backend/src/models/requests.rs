//! # API Request Models
//!
//! Structures for incoming API request bodies.
//! Each struct represents the expected JSON body for an endpoint.

use serde::{Deserialize, Serialize};

/// Request to initialize a new vault.
///
/// ## Example JSON
///
/// ```json
/// {
///     "userPubkey": "7xKt9Fj2abc123..."
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeVaultRequest {
    /// The user's wallet public key (base58 encoded).
    pub user_pubkey: String,

    /// Optional: Path to user's keypair file for automatic signing.
    /// **DEVNET/TESTING ONLY** - Allows backend to sign and submit automatically.
    /// Example: "~/.config/solana/id.json"
    /// If provided, backend will sign and submit the transaction automatically.
    pub user_keypair_path: Option<String>,
}

/// Request to deposit USDT into a vault.
///
/// ## Example JSON
///
/// ```json
/// {
///     "userPubkey": "7xKt9Fj2abc123...",
///     "amount": 100000000,
///     "signature": "5Ht3Rjabc..."
/// }
/// ```
///
/// ## Notes
///
/// - `amount` is in smallest units (6 decimals)
/// - 1 USDT = 1,000,000
/// - Example: 100 USDT = 100,000,000
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositRequest {
    /// User's wallet public key.
    pub user_pubkey: String,

    /// Amount to deposit (in smallest units, 6 decimals).
    /// Example: 100 USDT = 100_000_000
    pub amount: u64,

    /// Optional: Pre-signed transaction signature.
    /// If provided, backend will submit this transaction.
    /// If not provided, backend will build and return unsigned tx.
    pub signature: Option<String>,

    /// Optional: Path to user's keypair file for automatic signing.
    /// **DEVNET/TESTING ONLY** - Allows backend to sign and submit automatically.
    /// Example: "~/.config/solana/id.json"
    /// If provided, backend will sign and submit the transaction automatically.
    pub user_keypair_path: Option<String>,
}

/// Request to withdraw USDT from a vault.
///
/// ## Example JSON
///
/// ```json
/// {
///     "userPubkey": "7xKt9Fj2abc123...",
///     "amount": 50000000
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawRequest {
    /// User's wallet public key.
    pub user_pubkey: String,

    /// Amount to withdraw (in smallest units).
    pub amount: u64,

    /// Optional: Path to user's keypair file for automatic signing.
    /// **DEVNET/TESTING ONLY** - Allows backend to sign and submit automatically.
    /// Example: "~/.config/solana/id.json"
    /// If provided, backend will sign and submit the transaction automatically.
    pub user_keypair_path: Option<String>,
}

/// Request to lock collateral for a position.
///
/// ## Example JSON
///
/// ```json
/// {
///     "userPubkey": "7xKt9Fj2abc123...",
///     "amount": 10000000,
///     "positionId": "pos_123abc...",
///     "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
/// }
/// ```
///
/// ## Authorization
///
/// This endpoint requires internal authorization.
/// Only authorized trading systems can call it.
/// The transaction must be signed by the position manager's keypair.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockCollateralRequest {
    /// User's wallet public key.
    pub user_pubkey: String,

    /// Amount to lock (in smallest units).
    pub amount: u64,

    /// Position ID this lock is for.
    pub position_id: String,

    /// Path to position manager's keypair file for automatic signing.
    /// **REQUIRED** - Transaction must be signed by authorized position manager.
    /// Example: "~/.config/solana/position-manager.json"
    pub position_manager_keypair_path: String,
}

/// Request to unlock collateral after position close.
///
/// ## Example JSON
///
/// ```json
/// {
///     "userPubkey": "7xKt9Fj2abc123...",
///     "amount": 10000000,
///     "positionId": "pos_123abc...",
///     "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
/// }
/// ```
///
/// ## Authorization
///
/// The transaction must be signed by the position manager's keypair.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockCollateralRequest {
    /// User's wallet public key.
    pub user_pubkey: String,

    /// Amount to unlock (in smallest units).
    pub amount: u64,

    /// Position ID being closed.
    pub position_id: String,

    /// Path to position manager's keypair file for automatic signing.
    /// **REQUIRED** - Transaction must be signed by authorized position manager.
    /// Example: "~/.config/solana/position-manager.json"
    pub position_manager_keypair_path: String,
}

/// Request to transfer collateral between vaults.
///
/// ## Example JSON
///
/// ```json
/// {
///     "fromPubkey": "7xKt9Fj2abc123...",
///     "toPubkey": "9Yht3Mkxyz789...",
///     "amount": 50000000,
///     "reason": "settlement",
///     "liquidationEngineKeypairPath": "~/.config/solana/liquidation-engine.json"
/// }
/// ```
///
/// ## Reasons
///
/// - `settlement`: Trade settlement (winner receives from loser)
/// - `liquidation`: Position was liquidated
/// - `fee`: Protocol fee collection
///
/// ## Authorization
///
/// The transaction must be signed by the liquidation engine's keypair.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferCollateralRequest {
    /// Source vault owner's public key.
    pub from_pubkey: String,

    /// Destination vault owner's public key.
    pub to_pubkey: String,

    /// Amount to transfer (in smallest units).
    pub amount: u64,

    /// Reason for transfer.
    pub reason: String,

    /// Path to liquidation engine's keypair file for automatic signing.
    /// **REQUIRED** - Transaction must be signed by authorized liquidation engine.
    /// Example: "~/.config/solana/liquidation-engine.json"
    pub liquidation_engine_keypair_path: String,
}

/// Query parameters for transaction history.
///
/// ## Example URL
///
/// ```text
/// GET /vault/transactions/7xKt9Fj2...?limit=20&offset=0&type=deposit
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionQuery {
    /// Maximum number of transactions to return.
    /// Default: 20, Max: 100
    #[serde(default = "default_limit")]
    pub limit: i64,

    /// Number of transactions to skip (for pagination).
    /// Default: 0
    #[serde(default)]
    pub offset: i64,

    /// Filter by transaction type (optional).
    /// Values: deposit, withdrawal, lock, unlock, transfer_in, transfer_out
    #[serde(rename = "type")]
    pub tx_type: Option<String>,
}

fn default_limit() -> i64 {
    20
}

/// Request to mint test USDT (devnet only).
///
/// ## Example JSON
///
/// ```json
/// {
///     "userPubkey": "7xKt9Fj2abc123...",
///     "amount": 1000000000
/// }
/// ```
///
/// ## Notes
///
/// - **DEVNET ONLY** - This endpoint only works on devnet
/// - `amount` is in smallest units (6 decimals)
/// - 1 USDT = 1,000,000
/// - Example: 1000 USDT = 1,000,000,000
/// - Requires backend to have mint authority for the USDT mint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MintUsdtRequest {
    /// User's wallet public key.
    pub user_pubkey: String,

    /// Amount to mint (in smallest units, 6 decimals).
    /// Example: 1000 USDT = 1_000_000_000
    pub amount: u64,
}

/// Query parameters for balance history.
///
/// ## Example URL
///
/// ```text
/// GET /vault/history/7xKt9Fj2...?from=2024-01-01&to=2024-01-31
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceHistoryQuery {
    /// Start date (ISO 8601 format).
    pub from: String,

    /// End date (ISO 8601 format).
    pub to: String,
}

