//! # Events Module
//! 
//! This module defines all events that the Collateral Vault program emits.
//! Events are used for off-chain indexing and real-time notifications.
//! 
//! ## What are Events?
//! 
//! Events are log messages stored in the transaction data on Solana.
//! They don't affect program state but are useful for:
//! 
//! 1. **Off-chain indexing**: Services can listen for events and update databases
//! 2. **Real-time updates**: WebSocket connections can push events to frontends
//! 3. **Analytics**: Track TVL, deposit volumes, etc.
//! 4. **Audit trail**: Permanent record of all vault operations
//! 
//! ## How to Listen for Events:
//! 
//! ```javascript
//! // JavaScript/TypeScript example
//! program.addEventListener("DepositEvent", (event) => {
//!     console.log(`User ${event.user} deposited ${event.amount}`);
//!     updateDatabase(event);
//! });
//! ```
//! 
//! ## Event Flow:
//! ```text
//! User deposits USDT
//!        ↓
//! Vault Program emits DepositEvent
//!        ↓
//! Event stored in transaction logs
//!        ↓
//! Backend service detects event
//!        ↓
//! Updates PostgreSQL database
//!        ↓
//! WebSocket notifies frontend
//!        ↓
//! User sees updated balance
//! ```

use anchor_lang::prelude::*;

/// # DepositEvent
/// 
/// Emitted when a user deposits USDT into their vault.
/// 
/// ## Fields:
/// - `user`: The wallet address that made the deposit
/// - `vault`: The vault PDA address
/// - `amount`: Amount deposited (in USDT with 6 decimals)
/// - `new_balance`: Total balance after deposit
/// - `timestamp`: Unix timestamp of the deposit
/// 
/// ## Example Log:
/// ```text
/// DepositEvent {
///     user: "7xKt9Fj2...",
///     vault: "9Yht3Mk7...",
///     amount: 100_000_000,    // 100 USDT
///     new_balance: 500_000_000, // 500 USDT total
///     timestamp: 1699123456
/// }
/// ```
#[event]
pub struct DepositEvent {
    /// The public key of the user who deposited
    pub user: Pubkey,
    /// The vault PDA address
    pub vault: Pubkey,
    /// Amount deposited (USDT with 6 decimals)
    pub amount: u64,
    /// New total balance after deposit
    pub new_balance: u64,
    /// Unix timestamp of the deposit
    pub timestamp: i64,
}

/// # WithdrawEvent
/// 
/// Emitted when a user withdraws USDT from their vault.
/// 
/// ## Fields:
/// - `user`: The wallet address that made the withdrawal
/// - `vault`: The vault PDA address
/// - `amount`: Amount withdrawn (in USDT with 6 decimals)
/// - `remaining_balance`: Total balance after withdrawal
/// - `timestamp`: Unix timestamp of the withdrawal
#[event]
pub struct WithdrawEvent {
    /// The public key of the user who withdrew
    pub user: Pubkey,
    /// The vault PDA address
    pub vault: Pubkey,
    /// Amount withdrawn (USDT with 6 decimals)
    pub amount: u64,
    /// Remaining balance after withdrawal
    pub remaining_balance: u64,
    /// Unix timestamp of the withdrawal
    pub timestamp: i64,
}

/// # LockCollateralEvent
/// 
/// Emitted when collateral is locked for a trading position.
/// 
/// ## When is collateral locked?
/// 1. User opens a leveraged position
/// 2. System calculates required margin
/// 3. Margin amount is locked in vault
/// 4. This event is emitted
/// 
/// ## Example:
/// User opens 10x leveraged $1000 position:
/// - Required margin: $100
/// - $100 is locked from available balance
/// - LockCollateralEvent emitted with amount: 100_000_000
#[event]
pub struct LockCollateralEvent {
    /// The public key of the vault owner
    pub user: Pubkey,
    /// The vault PDA address
    pub vault: Pubkey,
    /// Amount locked (USDT with 6 decimals)
    pub amount: u64,
    /// New locked balance (total)
    pub new_locked_balance: u64,
    /// New available balance
    pub new_available_balance: u64,
    /// The program that requested the lock (position manager)
    pub locked_by: Pubkey,
    /// Unix timestamp
    pub timestamp: i64,
}

/// # UnlockCollateralEvent
/// 
/// Emitted when locked collateral is released.
/// 
/// ## When is collateral unlocked?
/// 1. User closes a trading position
/// 2. Position is settled (profit/loss calculated)
/// 3. Locked margin is released
/// 4. This event is emitted
/// 
/// ## Note:
/// The unlocked amount might differ from the originally locked amount
/// if there were profits/losses on the position.
#[event]
pub struct UnlockCollateralEvent {
    /// The public key of the vault owner
    pub user: Pubkey,
    /// The vault PDA address
    pub vault: Pubkey,
    /// Amount unlocked (USDT with 6 decimals)
    pub amount: u64,
    /// New locked balance (total)
    pub new_locked_balance: u64,
    /// New available balance
    pub new_available_balance: u64,
    /// The program that requested the unlock
    pub unlocked_by: Pubkey,
    /// Unix timestamp
    pub timestamp: i64,
}

/// # TransferCollateralEvent
/// 
/// Emitted when collateral is transferred between vaults.
/// 
/// ## Use Cases:
/// 1. **Trade Settlement**: Winner receives loser's margin
/// 2. **Liquidation**: Liquidator receives portion of liquidated margin
/// 3. **Fee Collection**: Protocol fees transferred to treasury
/// 
/// ## Example - Trade Settlement:
/// ```text
/// Alice (winner) vs Bob (loser)
/// Bob's loss: 50 USDT
/// 
/// Transfer: Bob's vault → Alice's vault
/// Amount: 50 USDT
/// ```
#[event]
pub struct TransferCollateralEvent {
    /// The source vault (sender)
    pub from_vault: Pubkey,
    /// The destination vault (receiver)
    pub to_vault: Pubkey,
    /// Amount transferred (USDT with 6 decimals)
    pub amount: u64,
    /// The program that initiated the transfer
    pub transferred_by: Pubkey,
    /// Reason for transfer (settlement, liquidation, etc.)
    pub reason: TransferReason,
    /// Unix timestamp
    pub timestamp: i64,
}

/// # TransferReason
/// 
/// Describes why a transfer between vaults occurred.
/// Used for analytics and audit trail.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransferReason {
    /// Trade settlement (winner receives loser's funds)
    Settlement,
    /// Position was liquidated
    Liquidation,
    /// Fee collection to protocol treasury
    FeeCollection,
    /// Insurance fund contribution
    InsuranceFund,
    /// Other/custom reason
    Other,
}

/// # VaultInitializedEvent
/// 
/// Emitted when a new vault is created for a user.
#[event]
pub struct VaultInitializedEvent {
    /// The owner of the new vault
    pub owner: Pubkey,
    /// The vault PDA address
    pub vault: Pubkey,
    /// The token account for this vault
    pub token_account: Pubkey,
    /// Unix timestamp of creation
    pub timestamp: i64,
}

/// # VaultPausedEvent
/// 
/// Emitted when the vault system is paused or unpaused.
#[event]
pub struct VaultPausedEvent {
    /// Whether the system is now paused (true) or unpaused (false)
    pub is_paused: bool,
    /// Admin who changed the state
    pub admin: Pubkey,
    /// Unix timestamp
    pub timestamp: i64,
}

