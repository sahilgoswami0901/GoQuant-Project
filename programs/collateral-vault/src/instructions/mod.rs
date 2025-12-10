//! # Instructions Module
//! 
//! This module contains all the instructions (functions) that can be called
//! on the Collateral Vault program. Each instruction performs a specific action.
//! 
//! ## Available Instructions:
//! 
//! | Instruction | Who Can Call | Description |
//! |-------------|--------------|-------------|
//! | `initialize_vault` | Any user | Create a new vault for the user |
//! | `initialize_vault_authority` | Admin | Set up the authority config (once) |
//! | `deposit` | Vault owner | Deposit USDT into vault |
//! | `withdraw` | Vault owner | Withdraw USDT from vault |
//! | `lock_collateral` | Authorized programs | Lock funds for positions |
//! | `unlock_collateral` | Authorized programs | Release locked funds |
//! | `transfer_collateral` | Authorized programs | Transfer between vaults |
//! 
//! ## Instruction Flow:
//! 
//! ```text
//! 1. User creates vault:     initialize_vault
//!                                   ↓
//! 2. User adds funds:        deposit
//!                                   ↓
//! 3. User opens position:    lock_collateral (via trading program)
//!                                   ↓
//! 4. User closes position:   unlock_collateral (via trading program)
//!                                   ↓
//! 5. User withdraws:         withdraw
//! ```

pub mod initialize_vault;
pub mod initialize_vault_authority;
pub mod deposit;
pub mod withdraw;
pub mod lock_collateral;
pub mod unlock_collateral;
pub mod transfer_collateral;

pub use initialize_vault::*;
pub use initialize_vault_authority::*;
pub use deposit::*;
pub use withdraw::*;
pub use lock_collateral::*;
pub use unlock_collateral::*;
pub use transfer_collateral::*;

