//! # State Module
//! 
//! This module contains all the account structures (state) that will be stored
//! on the Solana blockchain. Think of these as database tables, but stored on-chain.
//! 
//! ## Key Concepts for Beginners:
//! 
//! - **Account**: On Solana, everything is an account. Accounts store data and SOL.
//! - **PDA (Program Derived Address)**: A special account address that only your program can control.
//! - **Serialization**: Converting Rust structs to bytes for blockchain storage.

pub mod vault;
pub mod vault_authority;

pub use vault::*;
pub use vault_authority::*;

