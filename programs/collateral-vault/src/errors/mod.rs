//! # Error Handling Module
//! 
//! This module defines all custom errors that can occur in the Collateral Vault program.
//! Each error has a unique code and descriptive message to help debug issues.
//! 
//! ## How Errors Work in Anchor:
//! 
//! When something goes wrong, we return an error like this:
//! ```rust,ignore
//! if amount == 0 {
//!     return Err(VaultError::InvalidAmount.into());
//! }
//! ```
//! 
//! The error is then propagated to the client with its code and message.
//! 
//! ## Error Codes:
//! Anchor assigns error codes starting from 6000 (0x1770).
//! Each error gets an incremental code: 6000, 6001, 6002, etc.

use anchor_lang::prelude::*;

/// # VaultError
/// 
/// All possible errors that can occur when interacting with the vault program.
/// Each variant has a descriptive message explaining what went wrong.
/// 
/// ## Error Categories:
/// 
/// | Category | Error Codes | Description |
/// |----------|-------------|-------------|
/// | Input Validation | 6000-6009 | Invalid user input |
/// | Balance Errors | 6010-6019 | Insufficient funds |
/// | Authorization | 6020-6029 | Permission denied |
/// | State Errors | 6030-6039 | Invalid program state |
/// | Overflow/Math | 6040-6049 | Arithmetic errors |
#[error_code]
pub enum VaultError {
    // ============================================
    // INPUT VALIDATION ERRORS (6000-6009)
    // ============================================

    /// The amount provided is invalid (zero or negative).
    /// 
    /// ## When this occurs:
    /// - User tries to deposit 0 USDT
    /// - User tries to withdraw 0 USDT
    /// 
    /// ## How to fix:
    /// Ensure amount > 0 before calling the instruction.
    #[msg("Amount must be greater than zero")]
    InvalidAmount, // 6000

    /// The amount is below the minimum required deposit.
    /// 
    /// ## When this occurs:
    /// - User tries to deposit less than minimum (e.g., < 1 USDT)
    /// 
    /// ## How to fix:
    /// Deposit at least the minimum amount.
    #[msg("Amount is below minimum deposit requirement")]
    BelowMinimumDeposit, // 6001

    /// The provided token mint does not match expected USDT mint.
    /// 
    /// ## When this occurs:
    /// - User tries to deposit a different token (not USDT)
    /// 
    /// ## How to fix:
    /// Only use USDT token accounts.
    #[msg("Invalid token mint - only USDT is accepted")]
    InvalidTokenMint, // 6002

    // ============================================
    // BALANCE ERRORS (6010-6019)
    // ============================================

    /// User doesn't have enough available (unlocked) balance.
    /// 
    /// ## When this occurs:
    /// - Withdrawal amount > available_balance
    /// - Lock amount > available_balance
    /// 
    /// ## Example:
    /// ```text
    /// total_balance: 1000 USDT
    /// locked_balance: 800 USDT
    /// available_balance: 200 USDT
    /// 
    /// User tries to withdraw 500 USDT → ERROR
    /// ```
    /// 
    /// ## How to fix:
    /// Close some positions to unlock collateral, or reduce withdrawal amount.
    #[msg("Insufficient available balance for this operation")]
    InsufficientBalance = 6010, // 6010

    /// User has open positions and cannot withdraw all funds.
    /// 
    /// ## When this occurs:
    /// - User tries to withdraw but locked_balance > 0
    /// 
    /// ## How to fix:
    /// Close all trading positions first.
    #[msg("Cannot withdraw - you have open positions. Close positions first.")]
    HasOpenPositions, // 6011

    /// Trying to unlock more collateral than is locked.
    /// 
    /// ## When this occurs:
    /// - unlock_amount > locked_balance
    /// 
    /// ## This usually indicates a bug:
    /// The position manager might be tracking positions incorrectly.
    #[msg("Cannot unlock more collateral than is currently locked")]
    InsufficientLockedBalance, // 6012

    /// The vault token account balance doesn't match recorded balance.
    /// 
    /// ## When this occurs:
    /// - Discrepancy between actual token account and vault.total_balance
    /// 
    /// ## This is a critical error:
    /// Indicates potential security issue or bug.
    #[msg("Vault balance mismatch - actual token balance differs from recorded balance")]
    BalanceMismatch, // 6013

    // ============================================
    // AUTHORIZATION ERRORS (6020-6029)
    // ============================================

    /// Caller is not the owner of this vault.
    /// 
    /// ## When this occurs:
    /// - Someone other than the vault owner tries to withdraw
    /// 
    /// ## Security:
    /// This is a critical check - never allow unauthorized withdrawals.
    #[msg("You are not authorized to perform this action on this vault")]
    Unauthorized = 6020, // 6020

    /// The calling program is not in the authorized programs list.
    /// 
    /// ## When this occurs:
    /// - An unknown program tries to lock/unlock collateral
    /// 
    /// ## How to fix:
    /// Admin needs to add the program to VaultAuthority.authorized_programs
    #[msg("Calling program is not authorized to lock/unlock collateral")]
    UnauthorizedProgram, // 6021

    /// Caller is not the admin of VaultAuthority.
    /// 
    /// ## When this occurs:
    /// - Someone other than admin tries to modify authorized programs
    #[msg("Only admin can perform this action")]
    NotAdmin, // 6022

    /// Maximum number of authorized programs reached.
    /// 
    /// ## When this occurs:
    /// - Trying to add an authorized program when list is full
    /// 
    /// ## How to fix:
    /// Remove an existing authorized program first.
    #[msg("Maximum number of authorized programs reached (10)")]
    MaxAuthorizedProgramsReached, // 6023

    // ============================================
    // STATE ERRORS (6030-6039)
    // ============================================

    /// Vault is already initialized.
    /// 
    /// ## When this occurs:
    /// - Trying to initialize a vault that already exists
    #[msg("Vault already exists for this user")]
    VaultAlreadyExists = 6030, // 6030

    /// Vault has not been initialized.
    /// 
    /// ## When this occurs:
    /// - Trying to deposit/withdraw before initializing vault
    #[msg("Vault does not exist - initialize first")]
    VaultNotFound, // 6031

    /// The vault system is currently paused.
    /// 
    /// ## When this occurs:
    /// - Admin paused the system for maintenance or emergency
    /// 
    /// ## How to fix:
    /// Wait for admin to unpause the system.
    #[msg("Vault system is currently paused")]
    VaultPaused, // 6032

    // ============================================
    // OVERFLOW/MATH ERRORS (6040-6049)
    // ============================================

    /// Arithmetic overflow occurred.
    /// 
    /// ## When this occurs:
    /// - balance + amount > u64::MAX
    /// 
    /// ## Technical details:
    /// u64::MAX = 18,446,744,073,709,551,615
    /// With 6 decimals: ~18.4 quintillion USDT (impossible in practice)
    /// 
    /// ## This usually indicates a bug:
    /// Normal operations should never overflow.
    #[msg("Arithmetic overflow - this should never happen")]
    Overflow = 6040, // 6040

    /// Arithmetic underflow occurred.
    /// 
    /// ## When this occurs:
    /// - balance - amount < 0 (would wrap around)
    /// 
    /// ## This usually indicates a bug:
    /// Should be caught by balance checks before this.
    #[msg("Arithmetic underflow - this should never happen")]
    Underflow, // 6041
}

