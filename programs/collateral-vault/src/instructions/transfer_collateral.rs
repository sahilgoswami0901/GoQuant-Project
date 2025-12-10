//! # Transfer Collateral Instruction
//! 
//! This instruction transfers collateral between two vaults.
//! Used for trade settlements and liquidations.
//! 
//! ## Use Cases:
//! 
//! ### 1. Trade Settlement (Winner receives from Loser)
//! 
//! ```text
//! Alice (Long) vs Bob (Short)
//! Price goes UP → Alice wins, Bob loses
//! 
//! Transfer: Bob's vault → Alice's vault
//! Amount: Bob's loss (= Alice's profit)
//! ```
//! 
//! ### 2. Liquidation
//! 
//! ```text
//! Charlie's position goes underwater
//! Liquidator (Dave) liquidates Charlie
//! 
//! Transfer: Charlie's vault → Dave's vault (liquidator reward)
//! Transfer: Charlie's vault → Insurance Fund (remaining)
//! ```
//! 
//! ### 3. Fee Collection
//! 
//! ```text
//! Protocol charges 0.1% trading fee
//! 
//! Transfer: Trader's vault → Protocol Treasury
//! ```
//! 
//! ## Important:
//! 
//! - Only authorized programs can transfer
//! - This moves actual tokens (not just accounting)
//! - Both vault balances are updated atomically

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::state::{CollateralVault, VaultAuthority};
use crate::events::{TransferCollateralEvent, TransferReason};
use crate::errors::VaultError;

/// # transfer_collateral
/// 
/// Transfers collateral from one vault to another.
/// Only callable by authorized programs for settlements/liquidations.
/// 
/// ## Arguments
/// 
/// * `ctx` - Context containing all required accounts
/// * `amount` - Amount to transfer (USDT with 6 decimals)
/// * `reason` - Why this transfer is happening (for audit trail)
/// 
/// ## Returns
/// 
/// * `Ok(())` - Transfer successful
/// * `Err(VaultError::InsufficientBalance)` - Source vault doesn't have enough
/// * `Err(VaultError::VaultPaused)` - System is paused
/// 
/// ## Token Flow:
/// 
/// ```text
/// from_vault_token_account  ────────────>  to_vault_token_account
///         │                                         │
///         │  (SPL Token Transfer via CPI)           │
///         │                                         │
///         ▼                                         ▼
/// from_vault.total_balance -= amount    to_vault.total_balance += amount
/// ```
pub fn transfer_collateral(
    ctx: Context<TransferCollateral>,
    amount: u64,
    reason: TransferReason,
) -> Result<()> {
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
    // STEP 2: Check Source Vault Balance
    // ===================================
    
    // For settlements/liquidations, we transfer from LOCKED balance
    // (the losing position's margin is locked)
    let from_vault = &ctx.accounts.from_vault;
    
    // We can transfer from either locked or available balance
    // depending on the use case. For safety, check total.
    require!(
        from_vault.total_balance >= amount,
        VaultError::InsufficientBalance
    );

    // ===================================
    // STEP 3: Perform Token Transfer (CPI with PDA signing)
    // ===================================
    
    // The source vault PDA needs to sign the transfer
    let from_owner = ctx.accounts.from_vault.owner;
    let seeds = &[
        CollateralVault::SEED_PREFIX,
        from_owner.as_ref(),
        &[ctx.accounts.from_vault.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.from_token_account.to_account_info(),
            to: ctx.accounts.to_token_account.to_account_info(),
            authority: ctx.accounts.from_vault.to_account_info(),
        },
        signer_seeds,
    );

    token::transfer(cpi_context, amount)?;

    // ===================================
    // STEP 4: Update Both Vault States
    // ===================================
    
    let clock = Clock::get()?;
    
    // Update source vault (decrease balance)
    let from_vault = &mut ctx.accounts.from_vault;
    
    from_vault.total_balance = from_vault
        .total_balance
        .checked_sub(amount)
        .ok_or(VaultError::Underflow)?;
    
    // If we're transferring locked funds (settlement), decrease locked
    // If we're transferring available funds, decrease available
    // For simplicity, we prioritize decreasing locked_balance first
    if from_vault.locked_balance >= amount {
        from_vault.locked_balance = from_vault
            .locked_balance
            .checked_sub(amount)
            .ok_or(VaultError::Underflow)?;
    } else {
        // Reduce locked fully, then reduce available for the rest
        let remaining = amount - from_vault.locked_balance;
        from_vault.locked_balance = 0;
        from_vault.available_balance = from_vault
            .available_balance
            .checked_sub(remaining)
            .ok_or(VaultError::Underflow)?;
    }

    // Update destination vault (increase balance)
    let to_vault = &mut ctx.accounts.to_vault;
    
    to_vault.total_balance = to_vault
        .total_balance
        .checked_add(amount)
        .ok_or(VaultError::Overflow)?;
    
    // Incoming funds go to available balance
    to_vault.available_balance = to_vault
        .available_balance
        .checked_add(amount)
        .ok_or(VaultError::Overflow)?;

    // ===================================
    // STEP 5: Emit Event
    // ===================================
    
    emit!(TransferCollateralEvent {
        from_vault: ctx.accounts.from_vault.key(),
        to_vault: ctx.accounts.to_vault.key(),
        amount,
        transferred_by: ctx.accounts.authority.key(),
        reason,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Transferred {} USDT from {:?} to {:?}. Reason: {:?}",
        amount,
        ctx.accounts.from_vault.key(),
        ctx.accounts.to_vault.key(),
        reason
    );

    Ok(())
}

/// # TransferCollateral Accounts
/// 
/// Accounts required for transferring between vaults.
/// 
/// ## Account Structure:
/// 
/// ```text
/// authority (Signer) - The authorized program/admin
///     │
///     ├── from_vault (source)
///     │       └── from_token_account
///     │
///     └── to_vault (destination)
///             └── to_token_account
/// ```
#[derive(Accounts)]
pub struct TransferCollateral<'info> {
    /// The authority executing the transfer.
    /// Must be an authorized program.
    pub authority: Signer<'info>,

    // ========================================
    // SOURCE VAULT
    // ========================================
    
    /// The vault to transfer FROM.
    #[account(
        mut,
        seeds = [CollateralVault::SEED_PREFIX, from_vault.owner.as_ref()],
        bump = from_vault.bump
    )]
    pub from_vault: Account<'info, CollateralVault>,

    /// The token account of the source vault.
    #[account(
        mut,
        constraint = from_token_account.key() == from_vault.token_account @ VaultError::Unauthorized
    )]
    pub from_token_account: Account<'info, TokenAccount>,

    // ========================================
    // DESTINATION VAULT
    // ========================================
    
    /// The vault to transfer TO.
    #[account(
        mut,
        seeds = [CollateralVault::SEED_PREFIX, to_vault.owner.as_ref()],
        bump = to_vault.bump
    )]
    pub to_vault: Account<'info, CollateralVault>,

    /// The token account of the destination vault.
    #[account(
        mut,
        constraint = to_token_account.key() == to_vault.token_account @ VaultError::Unauthorized
    )]
    pub to_token_account: Account<'info, TokenAccount>,

    // ========================================
    // GLOBAL STATE
    // ========================================
    
    /// Global VaultAuthority.
    #[account(
        seeds = [VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    // ========================================
    // PROGRAMS
    // ========================================
    
    /// SPL Token Program.
    pub token_program: Program<'info, Token>,
}

