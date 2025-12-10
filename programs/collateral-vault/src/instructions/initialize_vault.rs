//! # Initialize Vault Instruction
//! 
//! This instruction creates a new collateral vault for a user.
//! Each user can only have ONE vault (enforced by PDA derivation).
//! 
//! ## What This Instruction Does:
//! 
//! 1. Creates a PDA account for the vault state
//! 2. Creates an associated token account for USDT
//! 3. Initializes all balance fields to zero
//! 4. Sets the user as the vault owner
//! 
//! ## Account Diagram:
//! 
//! ```text
//! BEFORE:                          AFTER:
//! 
//! User Wallet                      User Wallet
//! └── Has SOL for fees             └── Has SOL for fees (minus rent)
//!                                  
//!                                  Vault PDA (created)
//!                                  ├── owner: User
//!                                  ├── total_balance: 0
//!                                  └── ...
//!                                  
//!                                  Vault Token Account (created)
//!                                  └── balance: 0 USDT
//! ```
//! 
//! ## Rent-Exempt:
//! 
//! On Solana, accounts must hold a minimum SOL balance (rent) to exist.
//! If balance drops below rent, the account is deleted!
//! 
//! "Rent-exempt" means the account holds enough SOL to never pay rent.
//! Anchor handles this automatically with the `init` constraint.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::state::CollateralVault;
use crate::events::VaultInitializedEvent;

/// # initialize_vault
/// 
/// Creates a new collateral vault for the calling user.
/// 
/// ## Arguments
/// 
/// * `ctx` - The context containing all accounts needed for this instruction
/// 
/// ## Returns
/// 
/// * `Ok(())` - Vault successfully created
/// * `Err(...)` - Various errors (see error module)
/// 
/// ## Example Usage (from client):
/// 
/// ```typescript
/// await program.methods
///     .initializeVault()
///     .accounts({
///         user: wallet.publicKey,
///         vault: vaultPda,
///         usdtMint: USDT_MINT_ADDRESS,
///         vaultTokenAccount: vaultTokenAccount,
///         systemProgram: SystemProgram.programId,
///         tokenProgram: TOKEN_PROGRAM_ID,
///         associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
///     })
///     .rpc();
/// ```
/// 
/// ## Security Notes:
/// 
/// - Each user can only have ONE vault (enforced by PDA seeds)
/// - The user becomes the permanent owner of this vault
/// - Only the owner can withdraw from this vault
pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
    // Get the current timestamp from Solana's clock
    // Clock is a "sysvar" - a special account maintained by Solana
    let clock = Clock::get()?;

    // Capture the vault key BEFORE mutable borrow (to satisfy borrow checker)
    let vault_key = ctx.accounts.vault.key();
    // Get mutable reference to the vault account
    // Anchor has already created and allocated the account for us
    let vault = &mut ctx.accounts.vault;

    // Set all the initial values
    // ===========================
    
    // The user who created this vault becomes the permanent owner
    vault.owner = ctx.accounts.user.key();
    
    // Store reference to the token account that holds actual USDT
    vault.token_account = ctx.accounts.vault_token_account.key();
    
    // Initialize all balances to zero
    vault.total_balance = 0;
    vault.locked_balance = 0;
    vault.available_balance = 0;
    vault.total_deposited = 0;
    vault.total_withdrawn = 0;
    
    // Record creation timestamp
    vault.created_at = clock.unix_timestamp;
    
    // Store the bump seed for future PDA derivation
    // The bump is passed via ctx.bumps (Anchor calculates it)
    vault.bump = ctx.bumps.vault;

    // Emit event for off-chain indexing
    // ==================================
    // This event will be captured by backend services
    // and used to update databases, send notifications, etc.
    emit!(VaultInitializedEvent {
        owner: vault.owner,
        vault: vault_key,
        token_account: vault.token_account,
        timestamp: clock.unix_timestamp,
    });

    // Log a message (visible in transaction explorer)
    msg!(
        "Vault initialized for user: {} with token account: {}",
        vault.owner,
        vault.token_account
    );

    Ok(())
}

/// # InitializeVault Accounts
/// 
/// This struct defines ALL accounts required for the initialize_vault instruction.
/// Anchor validates these accounts before the instruction runs.
/// 
/// ## Account Validation:
/// 
/// Each account has "constraints" that Anchor checks:
/// - `mut`: Account will be modified
/// - `init`: Account will be created
/// - `seeds`: PDA derivation seeds
/// - `bump`: PDA bump seed
/// - `payer`: Who pays for account creation
/// - `space`: Size of account in bytes
/// 
/// If ANY constraint fails, the instruction is rejected before running.
#[derive(Accounts)]
pub struct InitializeVault<'info> {
    // ========================================
    // USER ACCOUNT (The person creating vault)
    // ========================================
    
    /// The user creating the vault.
    /// 
    /// ## Constraints:
    /// - `mut`: User's SOL balance will decrease (paying rent)
    /// - `Signer`: User must sign this transaction
    /// 
    /// ## What is a Signer?
    /// A Signer is an account whose private key was used to sign the transaction.
    /// This proves the user authorized this action.
    #[account(mut)]
    pub user: Signer<'info>,

    // ========================================
    // VAULT PDA (Created by this instruction)
    // ========================================
    
    /// The vault account that stores balance tracking and ownership info.
    /// 
    /// ## Constraints Explained:
    /// 
    /// ### `init`
    /// - Creates a new account
    /// - Fails if account already exists (prevents double initialization)
    /// 
    /// ### `seeds = [b"vault", user.key().as_ref()]`
    /// - PDA derivation seeds
    /// - "vault" = constant prefix (identifies account type)
    /// - user.key() = user's public key (makes it unique per user)
    /// 
    /// ### `bump`
    /// - Anchor finds the correct bump automatically
    /// - Stored in ctx.bumps.vault
    /// 
    /// ### `payer = user`
    /// - User pays the SOL rent for this account
    /// 
    /// ### `space = CollateralVault::LEN`
    /// - Allocates exactly enough bytes for the account
    #[account(
        init,
        seeds = [CollateralVault::SEED_PREFIX, user.key().as_ref()],
        bump,
        payer = user,
        space = CollateralVault::LEN
    )]
    pub vault: Account<'info, CollateralVault>,

    // ========================================
    // USDT MINT (Reference to USDT token)
    // ========================================
    
    /// The USDT token mint account.
    /// 
    /// ## What is a Mint?
    /// A Mint is the "definition" of a token. It contains:
    /// - Total supply
    /// - Decimal places (USDT has 6)
    /// - Mint authority (who can create more)
    /// 
    /// We don't modify it, just reference it to create the token account.
    pub usdt_mint: Account<'info, Mint>,

    // ========================================
    // VAULT TOKEN ACCOUNT (Holds actual USDT)
    // ========================================
    
    /// The Associated Token Account that will hold USDT for this vault.
    /// 
    /// ## What is an Associated Token Account (ATA)?
    /// 
    /// ATAs are special token accounts with deterministic addresses:
    /// ```text
    /// ATA Address = derive(wallet_address, token_mint)
    /// ```
    /// 
    /// This means you can calculate anyone's USDT account address
    /// if you know their wallet and the USDT mint.
    /// 
    /// ## Constraints:
    /// 
    /// ### `init`
    /// - Creates the token account
    /// 
    /// ### `payer = user`
    /// - User pays rent
    /// 
    /// ### `associated_token::mint = usdt_mint`
    /// - This token account is for USDT tokens
    /// 
    /// ### `associated_token::authority = vault`
    /// - The vault PDA controls this token account
    /// - This is KEY: the vault (not user) owns the tokens!
    #[account(
        init,
        payer = user,
        associated_token::mint = usdt_mint,
        associated_token::authority = vault
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    // ========================================
    // SYSTEM PROGRAMS (Required by Solana)
    // ========================================
    
    /// The Solana System Program.
    /// 
    /// Required for creating new accounts.
    /// Address: 11111111111111111111111111111111
    pub system_program: Program<'info, System>,

    /// The SPL Token Program.
    /// 
    /// Required for token account operations.
    /// Address: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
    pub token_program: Program<'info, Token>,

    /// The Associated Token Program.
    /// 
    /// Required for creating ATAs.
    /// Address: ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
    pub associated_token_program: Program<'info, AssociatedToken>,
}

