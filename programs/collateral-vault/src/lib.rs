// Suppress warnings from Anchor/Solana version mismatches
#![allow(unexpected_cfgs)]
#![allow(ambiguous_glob_reexports)]

//! # Collateral Vault Management System
//! 
//! A Solana smart contract (Anchor program) for managing user collateral
//! in a decentralized perpetual futures exchange.
//! 
//! ## Overview
//! 
//! This program provides secure, non-custodial management of user funds:
//! 
//! - **Deposits**: Users deposit USDT collateral into PDA-controlled vaults
//! - **Withdrawals**: Users withdraw available (unlocked) funds
//! - **Lock/Unlock**: Trading programs lock collateral for positions
//! - **Transfers**: Settlement of trades between vaults
//! 
//! ## Architecture
//! 
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    COLLATERAL VAULT PROGRAM                      │
//! │                                                                  │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
//! │  │ User Vault A │  │ User Vault B │  │ User Vault C │  ...     │
//! │  │  (PDA)       │  │  (PDA)       │  │  (PDA)       │          │
//! │  └──────────────┘  └──────────────┘  └──────────────┘          │
//! │                                                                  │
//! │  ┌──────────────────────────────────────────────────────────┐   │
//! │  │                    Vault Authority                        │   │
//! │  │  • Admin address                                          │   │
//! │  │  • Authorized programs list                               │   │
//! │  │  • Pause/unpause control                                  │   │
//! │  └──────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              │ CPI
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      SPL TOKEN PROGRAM                           │
//! │              (Handles actual USDT transfers)                     │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//! 
//! ## Security Model
//! 
//! 1. **PDA Control**: Vaults are PDAs - no private keys, only program controls
//! 2. **Owner Verification**: Only vault owner can withdraw
//! 3. **Authorized Programs**: Only whitelisted programs can lock/unlock
//! 4. **Atomic Operations**: All balance updates are atomic (no partial states)
//! 5. **Overflow Protection**: All arithmetic uses checked operations
//! 
//! ## Instructions Summary
//! 
//! | Instruction | Who Can Call | Description |
//! |-------------|--------------|-------------|
//! | `initialize_vault` | Any user | Create personal vault |
//! | `initialize_vault_authority` | Admin (once) | Set up permissions |
//! | `deposit` | Vault owner | Add USDT to vault |
//! | `withdraw` | Vault owner | Remove USDT from vault |
//! | `lock_collateral` | Authorized programs | Reserve for trading |
//! | `unlock_collateral` | Authorized programs | Release after trade |
//! | `transfer_collateral` | Authorized programs | Settlement/liquidation |
//! 
//! ## Example Usage
//! 
//! ```typescript
//! // 1. Initialize vault
//! await program.methods.initializeVault().rpc();
//! 
//! // 2. Deposit 100 USDT
//! await program.methods.deposit(new BN(100_000_000)).rpc();
//! 
//! // 3. [Trading program locks collateral via CPI]
//! 
//! // 4. [Trading program unlocks after position closes]
//! 
//! // 5. Withdraw profits
//! await program.methods.withdraw(new BN(150_000_000)).rpc();
//! ```

use anchor_lang::prelude::*;

// Module declarations
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

// Re-export for easier access
pub use errors::*;
pub use events::*;
pub use instructions::*;
pub use state::*;

// Declare the program ID
// This is a placeholder - replace with actual program ID after deployment
declare_id!("5nf16sQkkaRMybxk7kD92847W8E3VaonKC5sHsqBXCFD");

/// # Collateral Vault Program
/// 
/// The main program module containing all instruction handlers.
/// 
/// ## Anchor Macros Explained:
/// 
/// - `#[program]`: Marks this module as the Anchor program entry point
/// - Each function becomes a callable instruction
/// - First parameter is always `Context<T>` where T defines required accounts
#[program]
pub mod collateral_vault {
    use super::*; 

    // ========================================
    // VAULT INITIALIZATION
    // ========================================

    /// Initialize a new collateral vault for the calling user.
    /// 
    /// Creates:
    /// - A PDA account to store vault state
    /// - An associated token account for USDT
    /// 
    /// ## Accounts Required:
    /// - `user`: The user creating the vault (signer, payer)
    /// - `vault`: The vault PDA to create
    /// - `usdt_mint`: USDT token mint
    /// - `vault_token_account`: Associated token account for vault
    /// - System/Token/AssociatedToken programs
    /// 
    /// ## Errors:
    /// - Account already exists (PDA collision = vault already initialized)
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        instructions::initialize_vault(ctx)
    }

    /// Initialize the global VaultAuthority configuration.
    /// 
    /// This should be called ONCE after program deployment.
    /// Sets up the admin and authorized programs list.
    /// 
    /// ## Accounts Required:
    /// - `admin`: The initial admin (signer, payer)
    /// - `vault_authority`: The VaultAuthority PDA to create
    /// - System program
    pub fn initialize_vault_authority(ctx: Context<InitializeVaultAuthority>) -> Result<()> {
        instructions::initialize_vault_authority(ctx)
    }

    /// Add a program to the authorized list.
    /// Only the admin can call this.
    /// 
    /// ## Arguments:
    /// - `program_id`: The program ID to authorize
    pub fn add_authorized_program(
        ctx: Context<UpdateVaultAuthority>,
        program_id: Pubkey,
    ) -> Result<()> {
        instructions::add_authorized_program(ctx, program_id)
    }

    /// Remove a program from the authorized list.
    /// Only the admin can call this.
    /// 
    /// ## Arguments:
    /// - `program_id`: The program ID to remove
    pub fn remove_authorized_program(
        ctx: Context<UpdateVaultAuthority>,
        program_id: Pubkey,
    ) -> Result<()> {
        instructions::remove_authorized_program(ctx, program_id)
    }

    /// Pause or unpause the vault system.
    /// Only the admin can call this.
    /// 
    /// ## Arguments:
    /// - `is_paused`: `true` to pause, `false` to unpause
    pub fn set_paused(ctx: Context<UpdateVaultAuthority>, is_paused: bool) -> Result<()> {
        instructions::set_paused(ctx, is_paused)
    }

    // ========================================
    // USER OPERATIONS
    // ========================================

    /// Deposit USDT into the caller's vault.
    /// 
    /// Transfers tokens from user's wallet to vault's token account.
    /// 
    /// ## Arguments:
    /// - `amount`: Amount to deposit (USDT with 6 decimals)
    /// 
    /// ## Accounts Required:
    /// - `user`: Vault owner (signer)
    /// - `vault`: User's vault PDA
    /// - `user_token_account`: User's USDT account (source)
    /// - `vault_token_account`: Vault's USDT account (destination)
    /// - `vault_authority`: Global config
    /// - Token program
    /// 
    /// ## Errors:
    /// - `InvalidAmount`: amount is 0
    /// - `VaultPaused`: system is paused
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit(ctx, amount)
    }

    /// Withdraw USDT from the caller's vault.
    /// 
    /// Transfers tokens from vault to user's wallet.
    /// Can only withdraw available (unlocked) funds.
    /// 
    /// ## Arguments:
    /// - `amount`: Amount to withdraw (USDT with 6 decimals)
    /// 
    /// ## Errors:
    /// - `InsufficientBalance`: not enough available balance
    /// - `VaultPaused`: system is paused
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw(ctx, amount)
    }

    // ========================================
    // AUTHORIZED PROGRAM OPERATIONS
    // ========================================

    /// Lock collateral for a trading position.
    /// Only callable by authorized programs.
    /// 
    /// Moves funds from available to locked balance.
    /// Does NOT transfer tokens.
    /// 
    /// ## Arguments:
    /// - `amount`: Amount to lock (USDT with 6 decimals)
    /// 
    /// ## Errors:
    /// - `InsufficientBalance`: not enough available balance
    /// - `VaultPaused`: system is paused
    pub fn lock_collateral(ctx: Context<LockCollateral>, amount: u64) -> Result<()> {
        instructions::lock_collateral(ctx, amount)
    }

    /// Unlock previously locked collateral.
    /// Only callable by authorized programs.
    /// 
    /// Moves funds from locked to available balance.
    /// Does NOT transfer tokens.
    /// 
    /// ## Arguments:
    /// - `amount`: Amount to unlock (USDT with 6 decimals)
    /// 
    /// ## Errors:
    /// - `InsufficientLockedBalance`: not enough locked
    /// - `VaultPaused`: system is paused
    pub fn unlock_collateral(ctx: Context<UnlockCollateral>, amount: u64) -> Result<()> {
        instructions::unlock_collateral(ctx, amount)
    }

    /// Transfer collateral between vaults.
    /// Only callable by authorized programs.
    /// 
    /// Used for:
    /// - Trade settlement (winner receives from loser)
    /// - Liquidations
    /// - Fee collection
    /// 
    /// ## Arguments:
    /// - `amount`: Amount to transfer
    /// - `reason`: Why the transfer is happening (for audit)
    /// 
    /// ## Errors:
    /// - `InsufficientBalance`: source doesn't have enough
    /// - `VaultPaused`: system is paused
    pub fn transfer_collateral(
        ctx: Context<TransferCollateral>,
        amount: u64,
        reason: TransferReason,
    ) -> Result<()> {
        instructions::transfer_collateral(ctx, amount, reason)
    }
}

