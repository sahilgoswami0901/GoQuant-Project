//! # Services Module
//!
//! This module contains the core business logic services for the
//! Collateral Vault backend. Each service handles a specific domain.
//!
//! ## Services Overview
//!
//! | Service | Responsibility |
//! |---------|---------------|
//! | `VaultManager` | Vault lifecycle, deposits, withdrawals |
//! | `BalanceTracker` | Real-time balance monitoring, reconciliation |
//! | `TransactionBuilder` | Building Solana transactions |
//! | `VaultMonitor` | Continuous monitoring, alerts, TVL |
//!
//! ## Service Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        SERVICES LAYER                            │
//! │                                                                  │
//! │  ┌──────────────────────────────────────────────────────────┐   │
//! │  │                    VaultManager                           │   │
//! │  │  • initialize_vault()  • deposit()  • withdraw()          │   │
//! │  │  • lock_collateral()   • unlock_collateral()              │   │
//! │  └──────────────────────────────────────────────────────────┘   │
//! │                              │                                   │
//! │         ┌────────────────────┼────────────────────┐             │
//! │         ▼                    ▼                    ▼             │
//! │  ┌────────────┐      ┌────────────┐       ┌────────────┐       │
//! │  │Transaction │      │  Balance   │       │   Vault    │       │
//! │  │  Builder   │      │  Tracker   │       │  Monitor   │       │
//! │  │            │      │            │       │            │       │
//! │  │ Build txs  │      │ Reconcile  │       │ Alerts     │       │
//! │  │ Sign txs   │      │ Track TVL  │       │ TVL        │       │
//! │  └────────────┘      └────────────┘       └────────────┘       │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod vault_manager;
pub mod balance_tracker;
pub mod transaction_builder;
pub mod vault_monitor;
pub mod token_minter;
pub mod transaction_submitter;

pub use vault_manager::VaultManager;
pub use balance_tracker::BalanceTracker;
pub use transaction_builder::TransactionBuilder;
pub use vault_monitor::VaultMonitor;
pub use token_minter::TokenMinter;
pub use transaction_submitter::TransactionSubmitter;

