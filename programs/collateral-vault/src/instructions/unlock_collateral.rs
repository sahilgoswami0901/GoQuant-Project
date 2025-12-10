//! # Unlock Collateral Instruction
//! 
//! This instruction releases previously locked collateral.
//! 
//! ## When is Collateral Unlocked?
//! 
//! 1. **Position Closed**: User closes their trading position
//! 2. **Profit/Loss Settlement**: After P&L is calculated and applied
//! 3. **Partial Reduction**: User reduces position size
//! 
//! ## Example Flow:
//! 
//! ```text
//! 1. User opens position, 100 USDT locked
//!    └── locked_balance: 100, available: 400
//! 
//! 2. Position makes 20 USDT profit
//!    └── Settlement transfers 20 USDT to vault
//! 
//! 3. Position closed, collateral unlocked
//!    └── locked_balance: 0, available: 520 (original 100 + 20 profit)
//! ```
//! 
//! ## Important Notes:
//! 
//! - Only authorized programs can unlock
//! - Cannot unlock more than is currently locked
//! - Unlocking makes funds available for withdrawal

use anchor_lang::prelude::*;

use crate::state::{CollateralVault, VaultAuthority};
use crate::events::UnlockCollateralEvent;
use crate::errors::VaultError;

/// # unlock_collateral
/// 
/// Unlocks previously locked collateral in a user's vault.
/// Only callable by authorized programs.
/// 
/// ## Arguments
/// 
/// * `ctx` - Context containing all required accounts
/// * `amount` - Amount to unlock (USDT with 6 decimals)
/// 
/// ## Returns
/// 
/// * `Ok(())` - Unlock successful
/// * `Err(VaultError::InsufficientLockedBalance)` - Trying to unlock more than locked
/// * `Err(VaultError::VaultPaused)` - System is paused
/// 
/// ## Example - Position Close Flow:
/// 
/// ```text
/// User closes profitable position:
/// 
/// 1. Position Manager calculates: +50 USDT profit
/// 2. Settlement transfers 50 USDT from counterparty
/// 3. unlock_collateral(100) called (original margin)
/// 4. User now has 150 USDT available (100 margin + 50 profit)
/// ```
pub fn unlock_collateral(ctx: Context<UnlockCollateral>, amount: u64) -> Result<()> {
    // ===================================
    // STEP 1: Validate Input
    // ===================================
    
    require!(amount > 0, VaultError::InvalidAmount);
    
    // Check system is not paused
    require!(
        !ctx.accounts.vault_authority.is_paused,
        VaultError::VaultPaused
    );

    // ===================================
    // STEP 2: Check Locked Balance
    // ===================================
    
    let vault = &ctx.accounts.vault;
    
    // Cannot unlock more than is locked
    // This would indicate a bug in the calling program
    require!(
        vault.locked_balance >= amount,
        VaultError::InsufficientLockedBalance
    );

    // ===================================
    // STEP 3: Update Vault State
    // ===================================
    
    // Capture keys BEFORE mutable borrow (to satisfy borrow checker)
    let vault_key = ctx.accounts.vault.key();
    let authority_key = ctx.accounts.authority.key();
    
    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;

    // Move funds from locked to available
    // Note: total_balance stays the same!
    vault.locked_balance = vault
        .locked_balance
        .checked_sub(amount)
        .ok_or(VaultError::Underflow)?;
    
    vault.available_balance = vault
        .available_balance
        .checked_add(amount)
        .ok_or(VaultError::Overflow)?;

    // ===================================
    // STEP 4: Emit Event
    // ===================================
    
    emit!(UnlockCollateralEvent {
        user: vault.owner,
        vault: vault_key,
        amount,
        new_locked_balance: vault.locked_balance,
        new_available_balance: vault.available_balance,
        unlocked_by: authority_key,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Unlocked {} USDT. Locked: {}, Available: {}",
        amount,
        vault.locked_balance,
        vault.available_balance
    );

    Ok(())
}

/// # UnlockCollateral Accounts
/// 
/// Accounts required for the unlock_collateral instruction.
/// Similar to LockCollateral accounts.
#[derive(Accounts)]
pub struct UnlockCollateral<'info> {
    /// The authority requesting the unlock.
    /// Must be an authorized program (in production).
    pub authority: Signer<'info>,

    /// The vault to unlock collateral in.
    #[account(
        mut,
        seeds = [CollateralVault::SEED_PREFIX, vault.owner.as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, CollateralVault>,

    /// Global VaultAuthority.
    #[account(
        seeds = [VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
}

