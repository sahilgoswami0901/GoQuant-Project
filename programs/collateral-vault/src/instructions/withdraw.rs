//! # Withdraw Instruction
//! 
//! This instruction allows users to withdraw USDT from their vault.
//! 
//! ## Key Differences from Deposit:
//! 
//! 1. **Direction**: Money flows FROM vault TO user (opposite of deposit)
//! 2. **Authority**: The vault PDA signs the transfer (not the user)
//! 3. **Validation**: Must check available balance (unlocked funds only)
//! 
//! ## What Happens During a Withdrawal:
//! 
//! ```text
//! BEFORE:                              AFTER:
//! 
//! User Wallet                          User Wallet
//! └── USDT: 900                        └── USDT: 1000 (+100)
//!                                      
//! Vault PDA                            Vault PDA
//! ├── total_balance: 600               ├── total_balance: 500 (-100)
//! ├── locked_balance: 100              ├── locked_balance: 100
//! └── available_balance: 500           └── available_balance: 400 (-100)
//!                                      
//! Vault Token Account                  Vault Token Account
//! └── USDT balance: 600                └── USDT balance: 500 (-100)
//! ```
//! 
//! ## PDA Signing:
//! 
//! Since the vault PDA controls the token account, WE need to sign the transfer.
//! This is done using "PDA seeds" as the signer:
//! 
//! ```text
//! Normal Transaction:
//!     User signs with private key → Wallet authorizes
//! 
//! PDA Transaction:
//!     Program provides seeds → Solana verifies PDA → PDA authorizes
//! ```
//! 
//! ## Security:
//! 
//! - Only vault owner can withdraw
//! - Cannot withdraw more than available balance
//! - Locked collateral is protected (cannot be withdrawn)
//! - System must not be paused

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::state::{CollateralVault, VaultAuthority};
use crate::events::WithdrawEvent;
use crate::errors::VaultError;

/// # withdraw
/// 
/// Withdraws USDT from the vault to the user's wallet.
/// 
/// ## Arguments
/// 
/// * `ctx` - Context containing all required accounts
/// * `amount` - Amount of USDT to withdraw (in smallest units, 6 decimals)
/// 
/// ## Returns
/// 
/// * `Ok(())` - Withdrawal successful
/// * `Err(VaultError::InvalidAmount)` - Amount is zero
/// * `Err(VaultError::InsufficientBalance)` - Not enough available balance
/// * `Err(VaultError::VaultPaused)` - System is paused
/// 
/// ## Example (TypeScript client):
/// 
/// ```typescript
/// // Withdraw 50 USDT
/// const amount = new BN(50 * 1_000_000); // 50 USDT with 6 decimals
/// 
/// await program.methods
///     .withdraw(amount)
///     .accounts({
///         user: wallet.publicKey,
///         vault: vaultPda,
///         userTokenAccount: userUsdtAccount,
///         vaultTokenAccount: vaultUsdtAccount,
///         vaultAuthority: vaultAuthorityPda,
///         tokenProgram: TOKEN_PROGRAM_ID,
///     })
///     .rpc();
/// ```
pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    // ===================================
    // STEP 1: Validate Input
    // ===================================
    
    // Amount must be greater than zero
    require!(amount > 0, VaultError::InvalidAmount);

    // System must not be paused
    require!(
        !ctx.accounts.vault_authority.is_paused,
        VaultError::VaultPaused
    );

    // ===================================
    // STEP 2: Check Available Balance
    // ===================================
    
    let vault = &ctx.accounts.vault;
    
    // User can only withdraw from AVAILABLE balance (not locked)
    // This is the critical security check for perpetual trading!
    // 
    // Example:
    //   total_balance: 1000
    //   locked_balance: 700 (for open positions)
    //   available_balance: 300
    //   
    //   User tries to withdraw 500 → REJECTED (only 300 available)
    require!(
        vault.available_balance >= amount,
        VaultError::InsufficientBalance
    );

    // ===================================
    // STEP 3: Perform Token Transfer (CPI with PDA signing)
    // ===================================
    
    // Prepare the PDA signer seeds
    // These seeds + bump prove to Solana that this PDA authorizes the transfer
    let user_key = ctx.accounts.user.key();
    let seeds = &[
        CollateralVault::SEED_PREFIX,
        user_key.as_ref(),
        &[vault.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // Create CPI context WITH PDA signer
    // CpiContext::new_with_signer() is used when a PDA needs to sign
    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            // Source: Vault's token account (PDA controlled)
            from: ctx.accounts.vault_token_account.to_account_info(),
            // Destination: User's token account
            to: ctx.accounts.user_token_account.to_account_info(),
            // Authority: The vault PDA (we provide seeds to prove authority)
            authority: ctx.accounts.vault.to_account_info(),
        },
        // The seeds that prove we can sign for this PDA
        signer_seeds,
    );

    // Execute the transfer
    token::transfer(cpi_context, amount)?;

    // ===================================
    // STEP 4: Update Vault State
    // ===================================
    
    // Capture keys BEFORE mutable borrow (to satisfy borrow checker)
    let vault_key = ctx.accounts.vault.key();
    let user_key = ctx.accounts.user.key();
    
    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;

    // Update balances using checked arithmetic
    vault.total_balance = vault
        .total_balance
        .checked_sub(amount)
        .ok_or(VaultError::Underflow)?;
    
    vault.available_balance = vault
        .available_balance
        .checked_sub(amount)
        .ok_or(VaultError::Underflow)?;
    
    vault.total_withdrawn = vault
        .total_withdrawn
        .checked_add(amount)
        .ok_or(VaultError::Overflow)?;

    // ===================================
    // STEP 5: Emit Event
    // ===================================
    
    emit!(WithdrawEvent {
        user: user_key,
        vault: vault_key,
        amount,
        remaining_balance: vault.total_balance,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Withdrew {} USDT. Remaining balance: {}",
        amount,
        vault.total_balance
    );

    Ok(())
}

/// # Withdraw Accounts
/// 
/// All accounts required for the withdraw instruction.
/// Nearly identical to Deposit, but the transfer goes in reverse.
#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// The user withdrawing USDT.
    /// Must be the vault owner.
    #[account(mut)]
    pub user: Signer<'info>,

    /// The user's vault account.
    /// 
    /// ## Security Constraint:
    /// `constraint = vault.owner == user.key()` ensures only the owner
    /// can withdraw. This is THE most critical security check!
    #[account(
        mut,
        seeds = [CollateralVault::SEED_PREFIX, user.key().as_ref()],
        bump = vault.bump,
        constraint = vault.owner == user.key() @ VaultError::Unauthorized
    )]
    pub vault: Account<'info, CollateralVault>,

    /// User's USDT token account (destination for withdrawal).
    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ VaultError::Unauthorized
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Vault's USDT token account (source of withdrawal).
    #[account(
        mut,
        constraint = vault_token_account.key() == vault.token_account @ VaultError::Unauthorized
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    /// Global VaultAuthority (to check if paused).
    #[account(
        seeds = [VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    /// SPL Token Program.
    pub token_program: Program<'info, Token>,
}

