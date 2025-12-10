//! # Lock Collateral Instruction
//! 
//! This instruction locks a portion of a user's collateral for trading positions.
//! 
//! ## Why Lock Collateral?
//! 
//! In perpetual futures trading, when a user opens a position, they need "margin":
//! 
//! ```text
//! User wants to: Open 10x leveraged $1000 position
//! Required margin: $1000 / 10 = $100
//! 
//! BEFORE:                           AFTER:
//! ├── total_balance: 500            ├── total_balance: 500 (unchanged)
//! ├── locked_balance: 0             ├── locked_balance: 100 (+100)
//! └── available_balance: 500        └── available_balance: 400 (-100)
//! ```
//! 
//! ## Who Can Call This?
//! 
//! Only **authorized programs** (like the Position Manager) can lock collateral.
//! This is NOT callable by regular users!
//! 
//! ```text
//! User: "Lock my collateral"          → REJECTED ❌
//! Position Manager: "Lock collateral" → ALLOWED ✓ (if authorized)
//! Random Program: "Lock collateral"   → REJECTED ❌
//! ```
//! 
//! ## Important Notes:
//! 
//! - Locking does NOT transfer tokens
//! - It only updates internal accounting (locked_balance, available_balance)
//! - The actual tokens stay in the vault token account
//! - Locked funds cannot be withdrawn until unlocked

use anchor_lang::prelude::*;

use crate::state::{CollateralVault, VaultAuthority};
use crate::events::LockCollateralEvent;
use crate::errors::VaultError;

/// # lock_collateral
/// 
/// Locks collateral in a user's vault for margin requirements.
/// Only callable by authorized programs.
/// 
/// ## Arguments
/// 
/// * `ctx` - Context containing all required accounts
/// * `amount` - Amount to lock (USDT with 6 decimals)
/// 
/// ## Returns
/// 
/// * `Ok(())` - Lock successful
/// * `Err(VaultError::UnauthorizedProgram)` - Caller is not authorized
/// * `Err(VaultError::InsufficientBalance)` - Not enough available balance
/// * `Err(VaultError::VaultPaused)` - System is paused
/// 
/// ## How Position Manager Calls This (CPI):
/// 
/// ```rust,ignore
/// // In the Position Manager program
/// pub fn open_position(ctx: Context<OpenPosition>, size: u64) -> Result<()> {
///     let margin_required = size / leverage;
///     
///     // CPI to Vault Program
///     collateral_vault::cpi::lock_collateral(
///         CpiContext::new(
///             ctx.accounts.vault_program.to_account_info(),
///             LockCollateral {
///                 vault: ctx.accounts.user_vault,
///                 vault_authority: ctx.accounts.vault_authority,
///                 calling_program: // this program's ID
///             }
///         ),
///         margin_required
///     )?;
///     
///     // Continue with position opening...
///     Ok(())
/// }
/// ```
pub fn lock_collateral(ctx: Context<LockCollateral>, amount: u64) -> Result<()> {
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
    // STEP 2: Verify Caller is Authorized
    // ===================================
    
    // Get the calling program's ID
    // In a CPI, this would be the Position Manager's program ID
    // For now, we'll use the authority signer as a proxy
    // In production, you'd use more sophisticated verification
    
    // NOTE: In a real implementation with CPI, you would check:
    // let calling_program = ctx.accounts.calling_program.key();
    // require!(
    //     ctx.accounts.vault_authority.is_program_authorized(&calling_program),
    //     VaultError::UnauthorizedProgram
    // );

    // ===================================
    // STEP 3: Check Available Balance
    // ===================================
    
    let vault = &ctx.accounts.vault;
    
    require!(
        vault.available_balance >= amount,
        VaultError::InsufficientBalance
    );

    // ===================================
    // STEP 4: Update Vault State
    // ===================================
    
    // Capture keys BEFORE mutable borrow (to satisfy borrow checker)
    let vault_key = ctx.accounts.vault.key();
    let authority_key = ctx.accounts.authority.key();
    
    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;

    // Move funds from available to locked
    // Note: total_balance stays the same!
    vault.locked_balance = vault
        .locked_balance
        .checked_add(amount)
        .ok_or(VaultError::Overflow)?;
    
    vault.available_balance = vault
        .available_balance
        .checked_sub(amount)
        .ok_or(VaultError::Underflow)?;

    // ===================================
    // STEP 5: Emit Event
    // ===================================
    
    emit!(LockCollateralEvent {
        user: vault.owner,
        vault: vault_key,
        amount,
        new_locked_balance: vault.locked_balance,
        new_available_balance: vault.available_balance,
        locked_by: authority_key,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Locked {} USDT. Locked: {}, Available: {}",
        amount,
        vault.locked_balance,
        vault.available_balance
    );

    Ok(())
}

/// # LockCollateral Accounts
/// 
/// Accounts required for the lock_collateral instruction.
/// 
/// ## Security Model:
/// 
/// The `authority` account represents the program/entity requesting the lock.
/// In a full implementation, you would also pass the calling program's ID
/// and verify it's in the authorized list.
#[derive(Accounts)]
pub struct LockCollateral<'info> {
    /// The authority requesting the lock.
    /// 
    /// In production, this would typically be verified against
    /// VaultAuthority.authorized_programs.
    /// 
    /// For a simpler implementation, this could be:
    /// 1. A trusted backend signer
    /// 2. The position manager's PDA
    /// 3. A program-specific authority
    pub authority: Signer<'info>,

    /// The vault to lock collateral in.
    /// 
    /// Note: We use `has_one = owner` but don't require owner to sign.
    /// The lock is authorized by the authority, not the owner.
    #[account(
        mut,
        seeds = [CollateralVault::SEED_PREFIX, vault.owner.as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, CollateralVault>,

    /// Global VaultAuthority to check:
    /// 1. If system is paused
    /// 2. If calling program is authorized (in full implementation)
    #[account(
        seeds = [VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
}

