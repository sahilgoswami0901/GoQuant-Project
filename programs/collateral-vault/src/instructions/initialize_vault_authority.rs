//! # Initialize Vault Authority Instruction
//! 
//! This instruction creates the global VaultAuthority configuration account.
//! This should be called ONCE when deploying the program.
//! 
//! ## What is VaultAuthority?
//! 
//! VaultAuthority is a singleton account (only one exists) that:
//! 1. Stores the admin address (who can modify settings)
//! 2. Lists which programs can lock/unlock collateral
//! 3. Controls system pause/unpause state
//! 
//! ## Why is this needed?
//! 
//! In a perpetual futures exchange, multiple programs interact:
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    YOUR VAULT PROGRAM                    │
//! │                                                          │
//! │  Authorized Programs (from VaultAuthority):              │
//! │  ├── Position Manager: Can lock/unlock collateral        │
//! │  ├── Liquidation Engine: Can transfer between vaults     │
//! │  └── Settlement Relayer: Can process settlements         │
//! │                                                          │
//! │  Unauthorized Program:                                   │
//! │  └── Random Attacker: REJECTED ❌                        │
//! └─────────────────────────────────────────────────────────┘
//! ```
//! 
//! ## Deployment Sequence:
//! 
//! 1. Deploy the vault program
//! 2. Call `initialize_vault_authority` (sets up admin + permissions)
//! 3. Call `add_authorized_program` for each program that needs access
//! 4. Users can now initialize vaults and deposit

use anchor_lang::prelude::*;

use crate::state::VaultAuthority;

/// # initialize_vault_authority
/// 
/// Creates the global VaultAuthority configuration account.
/// This can only be called once (creating a second will fail).
/// 
/// ## Arguments
/// 
/// * `ctx` - The context containing all accounts
/// 
/// ## Security:
/// 
/// - The signer becomes the permanent admin
/// - Only the admin can add/remove authorized programs
/// - Only the admin can pause/unpause the system
pub fn initialize_vault_authority(ctx: Context<InitializeVaultAuthority>) -> Result<()> {
    let clock = Clock::get()?;
    let authority = &mut ctx.accounts.vault_authority;

    // Set the initial admin to whoever calls this instruction
    authority.admin = ctx.accounts.admin.key();
    
    // Start with an empty list of authorized programs
    authority.authorized_programs = Vec::new();
    
    // Store the bump for future PDA derivation
    authority.bump = ctx.bumps.vault_authority;
    
    // System is active by default (not paused)
    authority.is_paused = false;
    
    // Record initialization time
    authority.last_updated = clock.unix_timestamp;

    msg!(
        "VaultAuthority initialized with admin: {}",
        authority.admin
    );

    Ok(())
}

/// # add_authorized_program
/// 
/// Adds a program to the authorized list.
/// Only the admin can call this.
/// 
/// ## Arguments
/// 
/// * `ctx` - The context
/// * `program_id` - The program to authorize
pub fn add_authorized_program(
    ctx: Context<UpdateVaultAuthority>,
    program_id: Pubkey,
) -> Result<()> {
    let clock = Clock::get()?;
    let authority = &mut ctx.accounts.vault_authority;
    
    authority.add_authorized_program(program_id)?;
    authority.last_updated = clock.unix_timestamp;

    msg!("Added authorized program: {}", program_id);

    Ok(())
}

/// # remove_authorized_program
/// 
/// Removes a program from the authorized list.
/// Only the admin can call this.
/// 
/// ## Arguments
/// 
/// * `ctx` - The context
/// * `program_id` - The program to remove
pub fn remove_authorized_program(
    ctx: Context<UpdateVaultAuthority>,
    program_id: Pubkey,
) -> Result<()> {
    let clock = Clock::get()?;
    let authority = &mut ctx.accounts.vault_authority;
    
    authority.remove_authorized_program(&program_id);
    authority.last_updated = clock.unix_timestamp;

    msg!("Removed authorized program: {}", program_id);

    Ok(())
}

/// # set_paused
/// 
/// Pauses or unpauses the vault system.
/// When paused, no deposits/withdrawals/locks are allowed.
/// 
/// ## Arguments
/// 
/// * `ctx` - The context
/// * `is_paused` - true to pause, false to unpause
pub fn set_paused(ctx: Context<UpdateVaultAuthority>, is_paused: bool) -> Result<()> {
    let clock = Clock::get()?;
    let authority = &mut ctx.accounts.vault_authority;
    
    authority.is_paused = is_paused;
    authority.last_updated = clock.unix_timestamp;

    // Emit event
    emit!(crate::events::VaultPausedEvent {
        is_paused,
        admin: ctx.accounts.admin.key(),
        timestamp: clock.unix_timestamp,
    });

    msg!("Vault system paused: {}", is_paused);

    Ok(())
}

/// # InitializeVaultAuthority Accounts
/// 
/// Accounts required to create the VaultAuthority singleton.
#[derive(Accounts)]
pub struct InitializeVaultAuthority<'info> {
    /// The admin who will control the VaultAuthority.
    /// This should be a multisig or governance address in production.
    #[account(mut)]
    pub admin: Signer<'info>,

    /// The VaultAuthority PDA.
    /// 
    /// ## Seeds:
    /// Just ["vault_authority"] - no user key because it's global.
    /// This ensures only ONE can exist.
    #[account(
        init,
        seeds = [VaultAuthority::SEED_PREFIX],
        bump,
        payer = admin,
        space = VaultAuthority::LEN
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    /// System program for account creation.
    pub system_program: Program<'info, System>,
}

/// # UpdateVaultAuthority Accounts
/// 
/// Accounts required to modify the VaultAuthority (add/remove programs, pause).
#[derive(Accounts)]
pub struct UpdateVaultAuthority<'info> {
    /// The admin making the change.
    /// Must match the admin stored in VaultAuthority.
    #[account(
        constraint = admin.key() == vault_authority.admin @ crate::errors::VaultError::NotAdmin
    )]
    pub admin: Signer<'info>,

    /// The VaultAuthority to update.
    #[account(
        mut,
        seeds = [VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
}

