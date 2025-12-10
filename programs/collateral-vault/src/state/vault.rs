//! # Collateral Vault Account Structure
//! 
//! This file defines the main vault account that stores user collateral information.
//! Each user gets their own vault, which is a PDA (Program Derived Address).
//! 
//! ## Real-World Analogy:
//! Think of this as a user's personal safe deposit box record card that tracks:
//! - Who owns the box (owner)
//! - How much money is inside (total_balance)
//! - How much is being used for trading (locked_balance)
//! - How much can be withdrawn (available_balance)

use anchor_lang::prelude::*;

/// # CollateralVault
/// 
/// The main account structure that represents a user's collateral vault.
/// This is stored as a PDA (Program Derived Address) on the Solana blockchain.
/// 
/// ## Fields Explained:
/// 
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | owner | Pubkey | The wallet address of the vault owner |
/// | token_account | Pubkey | The SPL token account holding actual USDT |
/// | total_balance | u64 | Total USDT in the vault |
/// | locked_balance | u64 | USDT locked for open positions |
/// | available_balance | u64 | USDT available for withdrawal (total - locked) |
/// | total_deposited | u64 | Lifetime total deposited (for analytics) |
/// | total_withdrawn | u64 | Lifetime total withdrawn (for analytics) |
/// | created_at | i64 | Unix timestamp when vault was created |
/// | bump | u8 | PDA bump seed (used for address derivation) |
/// 
/// ## Example Usage:
/// ```rust,ignore
/// // Creating a new vault (conceptually)
/// let vault = CollateralVault {
///     owner: user_wallet_pubkey,
///     token_account: vault_usdt_token_account,
///     total_balance: 0,
///     locked_balance: 0,
///     available_balance: 0,
///     total_deposited: 0,
///     total_withdrawn: 0,
///     created_at: current_timestamp,
///     bump: pda_bump,
/// };
/// ```
/// 
/// ## Security Notes:
/// - Only the `owner` can withdraw funds
/// - `locked_balance` can only be modified by authorized programs
/// - `available_balance` must always equal `total_balance - locked_balance`
#[account]
#[derive(Default)]
pub struct CollateralVault {
    /// The public key of the user who owns this vault.
    /// Only this address can initiate withdrawals.
    /// 
    /// Size: 32 bytes
    pub owner: Pubkey,

    /// The public key of the SPL Token account that holds the actual USDT.
    /// This is separate from the vault PDA itself.
    /// 
    /// Think of it like this:
    /// - Vault PDA = Record keeping (tracks balances, ownership)
    /// - Token Account = Actual money storage (holds USDT tokens)
    /// 
    /// Size: 32 bytes
    pub token_account: Pubkey,

    /// The total amount of USDT currently in the vault.
    /// This equals the actual token balance in `token_account`.
    /// 
    /// Unit: USDT with 6 decimals (1 USDT = 1_000_000)
    /// Example: 100 USDT = 100_000_000
    /// 
    /// Size: 8 bytes
    pub total_balance: u64,

    /// The amount of USDT currently locked for open trading positions.
    /// Locked funds cannot be withdrawn until positions are closed.
    /// 
    /// Example: User has 1000 USDT, opens a position requiring 200 USDT margin
    /// - total_balance: 1000 USDT
    /// - locked_balance: 200 USDT
    /// - available_balance: 800 USDT
    /// 
    /// Size: 8 bytes
    pub locked_balance: u64,

    /// The amount of USDT available for withdrawal.
    /// Calculated as: total_balance - locked_balance
    /// 
    /// This is the "free" collateral that can be:
    /// 1. Withdrawn to user's wallet
    /// 2. Used to open new positions
    /// 
    /// Size: 8 bytes
    pub available_balance: u64,

    /// Lifetime total of all deposits made to this vault.
    /// Only increases, never decreases. Used for analytics.
    /// 
    /// Size: 8 bytes
    pub total_deposited: u64,

    /// Lifetime total of all withdrawals from this vault.
    /// Only increases, never decreases. Used for analytics.
    /// 
    /// Size: 8 bytes
    pub total_withdrawn: u64,

    /// Unix timestamp (seconds since Jan 1, 1970) when the vault was created.
    /// 
    /// Size: 8 bytes
    pub created_at: i64,

    /// The "bump" seed used in PDA derivation.
    /// 
    /// ## What is a bump?
    /// When creating a PDA, Solana tries different "bump" values (255, 254, 253...)
    /// until it finds an address that is NOT on the elliptic curve (has no private key).
    /// We store this bump to avoid recalculating it every time.
    /// 
    /// Size: 1 byte
    pub bump: u8,
}

impl CollateralVault {
    /// The total space (in bytes) required to store this account on-chain.
    /// 
    /// ## Calculation:
    /// - 8 bytes: Anchor discriminator (identifies account type)
    /// - 32 bytes: owner (Pubkey)
    /// - 32 bytes: token_account (Pubkey)
    /// - 8 bytes: total_balance (u64)
    /// - 8 bytes: locked_balance (u64)
    /// - 8 bytes: available_balance (u64)
    /// - 8 bytes: total_deposited (u64)
    /// - 8 bytes: total_withdrawn (u64)
    /// - 8 bytes: created_at (i64)
    /// - 1 byte: bump (u8)
    /// 
    /// Total: 8 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1 = 121 bytes
    /// 
    /// ## Why do we need this?
    /// Solana requires knowing the exact size when creating accounts
    /// to calculate the required rent (SOL needed to keep account alive).
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1;

    /// Seeds used to derive the PDA address for a user's vault.
    /// 
    /// ## How PDA derivation works:
    /// ```text
    /// Seeds: ["vault", user_public_key] + bump
    ///                    ↓
    ///         Hash Function (SHA256)
    ///                    ↓
    ///         PDA Address: "7xKt..." (unique per user)
    /// ```
    /// 
    /// This ensures each user gets a unique, deterministic vault address.
    pub const SEED_PREFIX: &'static [u8] = b"vault";
}

/// # TransactionType
/// 
/// Enum representing the different types of transactions that can occur in a vault.
/// Used for logging and tracking transaction history.
/// 
/// ## Variants:
/// - `Deposit`: User adds funds to vault
/// - `Withdrawal`: User removes funds from vault  
/// - `Lock`: Funds locked for trading position
/// - `Unlock`: Funds released after position closed
/// - `TransferIn`: Funds received from another vault (settlements)
/// - `TransferOut`: Funds sent to another vault (liquidations)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum TransactionType {
    /// User deposited USDT into the vault
    Deposit,
    /// User withdrew USDT from the vault
    Withdrawal,
    /// Collateral locked for a trading position
    Lock,
    /// Collateral unlocked after position closed
    Unlock,
    /// Received funds from another vault (e.g., winning a trade)
    TransferIn,
    /// Sent funds to another vault (e.g., losing a trade, liquidation)
    TransferOut,
}

