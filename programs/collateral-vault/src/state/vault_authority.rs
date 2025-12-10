//! # Vault Authority Account Structure
//! 
//! This file defines the VaultAuthority account which manages permissions
//! for which programs can interact with vaults (lock/unlock collateral).
//! 
//! ## Real-World Analogy:
//! Think of this as a security access list at a bank:
//! - Only specific authorized personnel (trading programs) can access the vault
//! - The list is maintained by an admin
//! - Unauthorized access attempts are rejected

use anchor_lang::prelude::*;

/// # VaultAuthority
/// 
/// A global configuration account that stores which programs are authorized
/// to perform privileged operations on vaults (like locking/unlocking collateral).
/// 
/// ## Why is this needed?
/// 
/// In a perpetual futures exchange, multiple programs interact:
/// 1. **Vault Program** (this one) - Manages collateral
/// 2. **Position Manager** - Opens/closes trading positions
/// 3. **Liquidation Engine** - Liquidates underwater positions
/// 4. **Settlement Relayer** - Settles trades between users
/// 
/// Only these authorized programs should be able to lock/unlock collateral.
/// Random programs or users should NOT be able to lock someone's funds.
/// 
/// ## Security Model:
/// ```text
/// User Request: "Lock my collateral"
///       ↓
/// Vault Program: "Who's asking?"
///       ↓
/// Check VaultAuthority.authorized_programs
///       ↓
/// If caller is in list → Allow
/// If caller is NOT in list → Reject with error
/// ```
/// 
/// ## Example Authorized Programs:
/// - Position Manager Program (opens leveraged trades)
/// - Liquidation Engine (closes risky positions)
/// - Settlement Relayer (distributes profits/losses)
#[account]
pub struct VaultAuthority {
    /// The admin who can modify the authorized programs list.
    /// Typically the protocol's multisig or governance address.
    /// 
    /// Size: 32 bytes
    pub admin: Pubkey,

    /// List of program IDs that are authorized to lock/unlock collateral.
    /// 
    /// ## Why Vec<Pubkey>?
    /// Different programs in the protocol need different permissions:
    /// - Trading program needs to lock collateral for positions
    /// - Liquidation program needs to transfer collateral
    /// 
    /// ## Maximum Size:
    /// We allocate space for up to 10 authorized programs.
    /// Each Pubkey is 32 bytes, so: 10 * 32 = 320 bytes max
    /// 
    /// Size: 4 bytes (Vec length) + (32 bytes × number of programs)
    pub authorized_programs: Vec<Pubkey>,

    /// The bump seed for PDA derivation.
    /// 
    /// Size: 1 byte
    pub bump: u8,

    /// Whether the vault system is paused (emergency stop).
    /// When true, no deposits/withdrawals/locks are allowed.
    /// 
    /// Size: 1 byte
    pub is_paused: bool,

    /// Unix timestamp of last update to authorized programs.
    /// Used for auditing and security monitoring.
    /// 
    /// Size: 8 bytes
    pub last_updated: i64,
}

impl VaultAuthority {
    /// Maximum number of authorized programs allowed.
    /// This prevents unbounded growth of the account size.
    pub const MAX_AUTHORIZED_PROGRAMS: usize = 10;

    /// The total space (in bytes) required to store this account.
    /// 
    /// ## Calculation:
    /// - 8 bytes: Anchor discriminator
    /// - 32 bytes: admin (Pubkey)
    /// - 4 bytes: Vec length prefix
    /// - 320 bytes: authorized_programs (10 × 32 bytes max)
    /// - 1 byte: bump (u8)
    /// - 1 byte: is_paused (bool)
    /// - 8 bytes: last_updated (i64)
    /// 
    /// Total: 8 + 32 + 4 + 320 + 1 + 1 + 8 = 374 bytes
    pub const LEN: usize = 8 + 32 + 4 + (32 * Self::MAX_AUTHORIZED_PROGRAMS) + 1 + 1 + 8;

    /// Seed prefix for deriving the VaultAuthority PDA.
    /// There is only ONE VaultAuthority account per program deployment.
    pub const SEED_PREFIX: &'static [u8] = b"vault_authority";

    /// Check if a program is authorized to perform privileged operations.
    /// 
    /// ## Arguments
    /// * `program_id` - The program ID to check
    /// 
    /// ## Returns
    /// * `true` if the program is authorized
    /// * `false` if the program is not in the authorized list
    /// 
    /// ## Example
    /// ```rust,ignore
    /// if !vault_authority.is_program_authorized(&calling_program_id) {
    ///     return Err(ErrorCode::UnauthorizedProgram.into());
    /// }
    /// ```
    pub fn is_program_authorized(&self, program_id: &Pubkey) -> bool {
        self.authorized_programs.contains(program_id)
    }

    /// Add a program to the authorized list.
    /// 
    /// ## Arguments
    /// * `program_id` - The program ID to authorize
    /// 
    /// ## Returns
    /// * `Ok(())` if successfully added
    /// * `Err` if the list is full or program already exists
    pub fn add_authorized_program(&mut self, program_id: Pubkey) -> Result<()> {
        // Check if already authorized
        if self.authorized_programs.contains(&program_id) {
            return Ok(()); // Already authorized, no-op
        }

        // Check if we have room
        if self.authorized_programs.len() >= Self::MAX_AUTHORIZED_PROGRAMS {
            return Err(error!(super::super::errors::VaultError::MaxAuthorizedProgramsReached));
        }

        self.authorized_programs.push(program_id);
        Ok(())
    }

    /// Remove a program from the authorized list.
    /// 
    /// ## Arguments
    /// * `program_id` - The program ID to remove
    /// 
    /// ## Returns
    /// * `Ok(())` if successfully removed or not found
    pub fn remove_authorized_program(&mut self, program_id: &Pubkey) {
        self.authorized_programs.retain(|p| p != program_id);
    }
}

