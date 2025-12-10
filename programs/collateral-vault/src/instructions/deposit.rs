//! # Deposit Instruction
//! 
//! This instruction allows users to deposit USDT into their vault.
//! 
//! ## What Happens During a Deposit:
//! 
//! ```text
//! BEFORE:                              AFTER:
//! 
//! User Wallet                          User Wallet
//! └── USDT: 1000                       └── USDT: 900 (-100)
//!                                      
//! Vault PDA                            Vault PDA
//! ├── total_balance: 500               ├── total_balance: 600 (+100)
//! ├── locked_balance: 100              ├── locked_balance: 100
//! └── available_balance: 400           └── available_balance: 500 (+100)
//!                                      
//! Vault Token Account                  Vault Token Account
//! └── USDT balance: 500                └── USDT balance: 600 (+100)
//! ```
//! 
//! ## Cross-Program Invocation (CPI):
//! 
//! Your program doesn't hold tokens directly - it calls the SPL Token Program:
//! 
//! ```text
//! Your Vault Program                 SPL Token Program
//!        │                                  │
//!        │   CPI: "Transfer 100 USDT"       │
//!        │─────────────────────────────────>│
//!        │                                  │
//!        │   from: user's token account     │
//!        │   to: vault's token account      │
//!        │   authority: user (signed tx)    │
//!        │                                  │
//!        │<─────────────────────────────────│
//!        │   Response: "Success!"           │
//! ```
//! 
//! ## Security:
//! 
//! - User must sign the transaction (proves they authorize the transfer)
//! - User's token account must have sufficient balance
//! - Amount must be greater than zero

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::state::{CollateralVault, VaultAuthority};
use crate::events::DepositEvent;
use crate::errors::VaultError;

/// # deposit
/// 
/// Deposits USDT from user's wallet into their vault.
/// 
/// ## Arguments
/// 
/// * `ctx` - Context containing all required accounts
/// * `amount` - Amount of USDT to deposit (in smallest units, 6 decimals)
///              Example: 100 USDT = 100_000_000
/// 
/// ## Returns
/// 
/// * `Ok(())` - Deposit successful
/// * `Err(VaultError::InvalidAmount)` - Amount is zero
/// * `Err(VaultError::VaultPaused)` - System is paused
/// * `Err(VaultError::Overflow)` - Balance would overflow (impossible in practice)
/// 
/// ## Example (TypeScript client):
/// 
/// ```typescript
/// // Deposit 100 USDT
/// const amount = new BN(100 * 1_000_000); // 100 USDT with 6 decimals
/// 
/// await program.methods
///     .deposit(amount)
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
pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    // ===================================
    // STEP 1: Validate Input
    // ===================================
    
    // Check that amount is greater than zero
    // require!() is a macro that returns an error if condition is false
    require!(amount > 0, VaultError::InvalidAmount);

    // Check that the system is not paused
    require!(
        !ctx.accounts.vault_authority.is_paused,
        VaultError::VaultPaused
    );

    // ===================================
    // STEP 2: Perform Token Transfer (CPI)
    // ===================================
    
    // Create the CPI context
    // CpiContext tells the token program what to do
    let cpi_context = CpiContext::new(
        // The program we're calling (SPL Token Program)
        ctx.accounts.token_program.to_account_info(),
        
        // The accounts for the transfer instruction
        Transfer {
            // Source: User's USDT token account
            from: ctx.accounts.user_token_account.to_account_info(),
            
            // Destination: Vault's USDT token account  
            to: ctx.accounts.vault_token_account.to_account_info(),
            
            // Authority: User (they signed the transaction)
            authority: ctx.accounts.user.to_account_info(),
        },
    );

    // Execute the transfer!
    // This calls the SPL Token Program's transfer instruction
    // The ? operator propagates any errors (like insufficient balance)
    token::transfer(cpi_context, amount)?;

    // ===================================
    // STEP 3: Update Vault State
    // ===================================
    
    // Capture keys BEFORE mutable borrow (to satisfy borrow checker)
    let vault_key = ctx.accounts.vault.key();
    let user_key = ctx.accounts.user.key();
    
    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;

    // Update balances using checked arithmetic to prevent overflow
    // checked_add returns None if overflow would occur
    vault.total_balance = vault
        .total_balance
        .checked_add(amount)
        .ok_or(VaultError::Overflow)?;
    
    vault.available_balance = vault
        .available_balance
        .checked_add(amount)
        .ok_or(VaultError::Overflow)?;
    
    vault.total_deposited = vault
        .total_deposited
        .checked_add(amount)
        .ok_or(VaultError::Overflow)?;

    // ===================================
    // STEP 4: Emit Event for Off-chain Indexing
    // ===================================
    
    emit!(DepositEvent {
        user: user_key,
        vault: vault_key,
        amount,
        new_balance: vault.total_balance,
        timestamp: clock.unix_timestamp,
    });

    // Log for debugging (visible in transaction explorer)
    msg!(
        "Deposited {} USDT. New balance: {}",
        amount,
        vault.total_balance
    );

    Ok(())
}

/// # Deposit Accounts
/// 
/// All accounts required for the deposit instruction.
/// 
/// ## Account Relationships:
/// 
/// ```text
/// user (Signer)
///   │
///   ├── owns ──> user_token_account (has USDT to deposit)
///   │
///   └── owns ──> vault (PDA derived from user's pubkey)
///                   │
///                   └── controls ──> vault_token_account (receives USDT)
/// ```
#[derive(Accounts)]
pub struct Deposit<'info> {
    // ========================================
    // USER (The depositor)
    // ========================================
    
    /// The user depositing USDT.
    /// 
    /// ## Why `mut`?
    /// The user's account is not modified, but we need `mut` because
    /// they're acting as the authority for the token transfer.
    /// 
    /// ## Why `Signer`?
    /// The user must sign to authorize the token transfer.
    /// Without their signature, we can't take their USDT!
    #[account(mut)]
    pub user: Signer<'info>,

    // ========================================
    // VAULT STATE (Balance tracking)
    // ========================================
    
    /// The user's vault account (PDA).
    /// 
    /// ## Constraints:
    /// 
    /// ### `mut`
    /// We need to update balance fields.
    /// 
    /// ### `seeds = [b"vault", user.key().as_ref()]`
    /// Derives the correct vault PDA for this user.
    /// 
    /// ### `bump = vault.bump`
    /// Uses the stored bump (faster than recalculating).
    /// 
    /// ### `constraint = vault.owner == user.key()`
    /// SECURITY: Ensures only the owner can deposit to this vault.
    /// (This also implicitly happens due to PDA derivation, but explicit is better!)
    #[account(
        mut,
        seeds = [CollateralVault::SEED_PREFIX, user.key().as_ref()],
        bump = vault.bump,
        constraint = vault.owner == user.key() @ VaultError::Unauthorized
    )]
    pub vault: Account<'info, CollateralVault>,

    // ========================================
    // TOKEN ACCOUNTS
    // ========================================
    
    /// User's USDT token account (source of deposit).
    /// 
    /// ## Constraints:
    /// 
    /// ### `mut`
    /// Balance will decrease by deposit amount.
    /// 
    /// ### `constraint = user_token_account.owner == user.key()`
    /// SECURITY: Must be the user's own token account.
    /// Prevents depositing from someone else's account!
    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ VaultError::Unauthorized
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Vault's USDT token account (destination).
    /// 
    /// ## Constraints:
    /// 
    /// ### `mut`
    /// Balance will increase by deposit amount.
    /// 
    /// ### `constraint = vault_token_account.key() == vault.token_account`
    /// SECURITY: Must be the vault's official token account.
    /// Prevents depositing to a wrong/attacker's account.
    #[account(
        mut,
        constraint = vault_token_account.key() == vault.token_account @ VaultError::Unauthorized
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    // ========================================
    // GLOBAL STATE
    // ========================================
    
    /// The global VaultAuthority (to check if system is paused).
    #[account(
        seeds = [VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    // ========================================
    // PROGRAMS
    // ========================================
    
    /// SPL Token Program (needed for transfer CPI).
    pub token_program: Program<'info, Token>,
}

