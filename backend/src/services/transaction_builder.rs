//! # Transaction Builder Service
//!
//! The TransactionBuilder creates Solana transactions for vault operations.
//! It handles all the complexity of building properly formatted transactions.
//!
//! ## Responsibilities
//!
//! - Build initialize_vault transactions
//! - Build deposit transactions
//! - Build withdrawal transactions
//! - Build lock/unlock collateral transactions
//! - Handle SPL Token account setup
//! - Set appropriate compute budgets
//!
//! ## Transaction Structure
//!
//! A Solana transaction consists of:
//!
//! ```text
//! Transaction
//! ├── Recent Blockhash (for expiration)
//! ├── Fee Payer (who pays transaction fees)
//! └── Instructions[]
//!     ├── Compute Budget (optional, for priority)
//!     └── Program Instruction (the actual operation)
//!         ├── Program ID
//!         ├── Accounts[]
//!         └── Data (serialized instruction args)
//! ```
//!
//! ## Unsigned vs Signed Transactions
//!
//! This builder creates **unsigned** transactions. The flow is:
//!
//! ```text
//! 1. Backend builds unsigned transaction
//!              ↓
//! 2. Send to frontend (base64 encoded)
//!              ↓
//! 3. User signs with their wallet
//!              ↓
//! 4. Frontend submits signed transaction to Solana
//! ```
//!
//! This ensures the user's private key never leaves their wallet.

use std::str::FromStr;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    transaction::Transaction,
    system_program,
};
use tracing::{info, debug};

use crate::config::AppConfig;
use crate::solana::SolanaClient;

/// Errors that can occur when building transactions.
#[derive(Debug, thiserror::Error)]
pub enum TransactionBuilderError {
    /// Invalid public key format.
    #[error("Invalid public key: {0}")]
    InvalidPubkey(String),

    /// Failed to get recent blockhash.
    #[error("Failed to get blockhash: {0}")]
    BlockhashError(String),

    /// Failed to serialize transaction.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Solana client error.
    #[error("Solana error: {0}")]
    #[allow(dead_code)]
    SolanaError(String),
}

/// The Transaction Builder service.
///
/// Creates properly formatted Solana transactions for
/// vault operations.
///
/// ## Usage
///
/// ```rust,ignore
/// let builder = TransactionBuilder::new(solana_client, config);
///
/// // Build a deposit transaction
/// let tx = builder.build_deposit("user_pubkey", 100_000_000).await?;
///
/// // tx is base64-encoded, send to frontend for signing
/// ```
#[derive(Clone)]
pub struct TransactionBuilder {
    /// Solana RPC client.
    solana: SolanaClient,

    /// Application configuration.
    #[allow(dead_code)]
    config: AppConfig,

    /// The vault program ID.
    program_id: Pubkey,

    /// USDT token mint.
    usdt_mint: Pubkey,
}

impl TransactionBuilder {
    /// Create a new TransactionBuilder.
    ///
    /// ## Arguments
    ///
    /// * `solana` - Solana RPC client
    /// * `config` - Application configuration
    pub fn new(solana: SolanaClient, config: AppConfig) -> Self {
        let program_id = Pubkey::from_str(&config.vault_program_id)
            .expect("Invalid vault program ID in config");
        
        let usdt_mint = Pubkey::from_str(&config.usdt_mint)
            .expect("Invalid USDT mint in config");

        Self {
            solana,
            config,
            program_id,
            usdt_mint,
        }
    }

    /// Build an initialize_vault transaction.
    ///
    /// Creates a new vault PDA and associated token account for the user.
    ///
    /// ## Arguments
    ///
    /// * `user_pubkey` - User's wallet public key (base58)
    ///
    /// ## Returns
    ///
    /// * `Ok((tx_base64, vault_pda))` - Unsigned transaction and vault address
    /// * `Err(...)` - Failed to build transaction
    ///
    /// ## Accounts Required
    ///
    /// 1. User (signer, payer)
    /// 2. Vault PDA (will be created)
    /// 3. USDT Mint
    /// 4. Vault Token Account (will be created)
    /// 5. System Program
    /// 6. Token Program
    /// 7. Associated Token Program
    pub async fn build_initialize_vault(
        &self,
        user_pubkey: &str,
    ) -> Result<(String, String), TransactionBuilderError> {
        info!("Building initialize_vault transaction for: {}", user_pubkey);

        let user = Pubkey::from_str(user_pubkey)
            .map_err(|_| TransactionBuilderError::InvalidPubkey(user_pubkey.to_string()))?;

        // Derive the vault PDA
        let (vault_pda, _bump) = Pubkey::find_program_address(
            &[b"vault", user.as_ref()],
            &self.program_id,
        );

        // Derive the vault authority PDA
        let (_vault_authority, _) = Pubkey::find_program_address(
            &[b"vault_authority"],
            &self.program_id,
        );

        // Get the vault's associated token account
        let vault_token_account = spl_associated_token_account::get_associated_token_address(
            &vault_pda,
            &self.usdt_mint,
        );

        // Build the instruction
        // Note: In production, you'd use Anchor's IDL to build this
        // For now, we'll create a placeholder instruction structure
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(user, true),           // user (signer, payer)
                AccountMeta::new(vault_pda, false),     // vault (PDA, will init)
                AccountMeta::new_readonly(self.usdt_mint, false), // usdt_mint
                AccountMeta::new(vault_token_account, false), // vault_token_account
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(
                    spl_associated_token_account::id(),
                    false,
                ),
            ],
            // Data would be the serialized Anchor instruction
            // For production: use anchor_client to build proper instruction data
            data: vec![0], // Placeholder: "initialize_vault" discriminator
        };

        // Get recent blockhash
        let _blockhash = self
            .solana
            .get_recent_blockhash()
            .await
            .map_err(|e| TransactionBuilderError::BlockhashError(e.to_string()))?;

        // Build transaction
        let message = Message::new(&[instruction], Some(&user));
        let transaction = Transaction::new_unsigned(message);

        // Serialize to base64
        let tx_bytes = bincode::serialize(&transaction)
            .map_err(|e| TransactionBuilderError::SerializationError(e.to_string()))?;
        let tx_base64 = BASE64.encode(&tx_bytes);

        debug!("Built initialize_vault tx, vault PDA: {}", vault_pda);

        Ok((tx_base64, vault_pda.to_string()))
    }

    /// Build a deposit transaction.
    ///
    /// Transfers USDT from user's wallet to their vault.
    ///
    /// ## Arguments
    ///
    /// * `user_pubkey` - User's wallet public key
    /// * `amount` - Amount to deposit (smallest units, 6 decimals)
    ///
    /// ## Returns
    ///
    /// Base64-encoded unsigned transaction
    pub async fn build_deposit(
        &self,
        user_pubkey: &str,
        amount: u64,
    ) -> Result<String, TransactionBuilderError> {
        info!("Building deposit transaction: {} USDT for {}", 
            amount as f64 / 1_000_000.0, user_pubkey);

        let user = Pubkey::from_str(user_pubkey)
            .map_err(|_| TransactionBuilderError::InvalidPubkey(user_pubkey.to_string()))?;

        // Derive PDAs
        let (vault_pda, _) = Pubkey::find_program_address(
            &[b"vault", user.as_ref()],
            &self.program_id,
        );

        let (vault_authority, _) = Pubkey::find_program_address(
            &[b"vault_authority"],
            &self.program_id,
        );

        // Get token accounts
        let user_token_account = spl_associated_token_account::get_associated_token_address(
            &user,
            &self.usdt_mint,
        );

        let vault_token_account = spl_associated_token_account::get_associated_token_address(
            &vault_pda,
            &self.usdt_mint,
        );

        // Build instruction
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(user, true),               // user (signer)
                AccountMeta::new(vault_pda, false),         // vault
                AccountMeta::new(user_token_account, false), // user_token_account
                AccountMeta::new(vault_token_account, false), // vault_token_account
                AccountMeta::new_readonly(vault_authority, false), // vault_authority
                AccountMeta::new_readonly(spl_token::id(), false), // token_program
            ],
            // Placeholder: "deposit" discriminator + amount
            data: Self::build_deposit_data(amount),
        };

        // Get recent blockhash
        let _blockhash = self
            .solana
            .get_recent_blockhash()
            .await
            .map_err(|e| TransactionBuilderError::BlockhashError(e.to_string()))?;

        // Build and serialize transaction
        let message = Message::new(&[instruction], Some(&user));
        let transaction = Transaction::new_unsigned(message);

        let tx_bytes = bincode::serialize(&transaction)
            .map_err(|e| TransactionBuilderError::SerializationError(e.to_string()))?;
        
        Ok(BASE64.encode(&tx_bytes))
    }

    /// Build a withdraw transaction.
    ///
    /// Transfers USDT from user's vault to their wallet.
    ///
    /// ## Arguments
    ///
    /// * `user_pubkey` - User's wallet public key
    /// * `amount` - Amount to withdraw (smallest units)
    pub async fn build_withdraw(
        &self,
        user_pubkey: &str,
        amount: u64,
    ) -> Result<String, TransactionBuilderError> {
        info!("Building withdraw transaction: {} USDT for {}", 
            amount as f64 / 1_000_000.0, user_pubkey);

        let user = Pubkey::from_str(user_pubkey)
            .map_err(|_| TransactionBuilderError::InvalidPubkey(user_pubkey.to_string()))?;

        // Derive PDAs
        let (vault_pda, _) = Pubkey::find_program_address(
            &[b"vault", user.as_ref()],
            &self.program_id,
        );

        let (vault_authority, _) = Pubkey::find_program_address(
            &[b"vault_authority"],
            &self.program_id,
        );

        // Get token accounts
        let user_token_account = spl_associated_token_account::get_associated_token_address(
            &user,
            &self.usdt_mint,
        );

        let vault_token_account = spl_associated_token_account::get_associated_token_address(
            &vault_pda,
            &self.usdt_mint,
        );

        // Build instruction
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(user, true),               // user (signer)
                AccountMeta::new(vault_pda, false),         // vault
                AccountMeta::new(user_token_account, false), // user_token_account
                AccountMeta::new(vault_token_account, false), // vault_token_account
                AccountMeta::new_readonly(vault_authority, false), // vault_authority
                AccountMeta::new_readonly(spl_token::id(), false), // token_program
            ],
            data: Self::build_withdraw_data(amount),
        };

        // Get recent blockhash
        let _blockhash = self
            .solana
            .get_recent_blockhash()
            .await
            .map_err(|e| TransactionBuilderError::BlockhashError(e.to_string()))?;

        // Build and serialize transaction
        let message = Message::new(&[instruction], Some(&user));
        let transaction = Transaction::new_unsigned(message);

        let tx_bytes = bincode::serialize(&transaction)
            .map_err(|e| TransactionBuilderError::SerializationError(e.to_string()))?;
        
        Ok(BASE64.encode(&tx_bytes))
    }

    /// Build a lock_collateral transaction.
    ///
    /// Locks collateral for a trading position. Must be signed by the position manager
    /// (authorized program). The authority is both the signer and fee payer.
    ///
    /// ## Arguments
    ///
    /// * `user_pubkey` - Vault owner's public key
    /// * `amount` - Amount to lock (in smallest units, 6 decimals)
    /// * `authority_pubkey` - Position manager's public key (will sign the transaction)
    ///
    /// ## Returns
    ///
    /// Base64-encoded unsigned transaction (must be signed by the authority)
    pub async fn build_lock_collateral(
        &self,
        user_pubkey: &str,
        amount: u64,
        authority_pubkey: &str,
    ) -> Result<String, TransactionBuilderError> {
        info!("Building lock_collateral transaction: {} USDT for {} (authority: {})", 
            amount as f64 / 1_000_000.0, user_pubkey, authority_pubkey);

        let user = Pubkey::from_str(user_pubkey)
            .map_err(|_| TransactionBuilderError::InvalidPubkey(user_pubkey.to_string()))?;

        let authority = Pubkey::from_str(authority_pubkey)
            .map_err(|_| TransactionBuilderError::InvalidPubkey(authority_pubkey.to_string()))?;

        let (vault_pda, _) = Pubkey::find_program_address(
            &[b"vault", user.as_ref()],
            &self.program_id,
        );

        let (vault_authority, _) = Pubkey::find_program_address(
            &[b"vault_authority"],
            &self.program_id,
        );

        // The authority (position manager) is the signer
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(authority, true),          // authority (signer - position manager)
                AccountMeta::new(vault_pda, false),         // vault
                AccountMeta::new_readonly(vault_authority, false), // vault_authority
            ],
            data: Self::build_lock_data(amount),
        };

        let _blockhash = self
            .solana
            .get_recent_blockhash()
            .await
            .map_err(|e| TransactionBuilderError::BlockhashError(e.to_string()))?;

        // Authority (position manager) is the fee payer
        let message = Message::new(&[instruction], Some(&authority));
        let transaction = Transaction::new_unsigned(message);

        let tx_bytes = bincode::serialize(&transaction)
            .map_err(|e| TransactionBuilderError::SerializationError(e.to_string()))?;
        
        Ok(BASE64.encode(&tx_bytes))
    }

    /// Build an unlock_collateral transaction.
    ///
    /// Unlocks collateral after a trading position is closed. Must be signed by the position manager
    /// (authorized program). The authority is both the signer and fee payer.
    ///
    /// ## Arguments
    ///
    /// * `user_pubkey` - Vault owner's public key
    /// * `amount` - Amount to unlock (in smallest units, 6 decimals)
    /// * `authority_pubkey` - Position manager's public key (will sign the transaction)
    ///
    /// ## Returns
    ///
    /// Base64-encoded unsigned transaction (must be signed by the authority)
    pub async fn build_unlock_collateral(
        &self,
        user_pubkey: &str,
        amount: u64,
        authority_pubkey: &str,
    ) -> Result<String, TransactionBuilderError> {
        info!("Building unlock_collateral transaction: {} USDT for {} (authority: {})", 
            amount as f64 / 1_000_000.0, user_pubkey, authority_pubkey);

        let user = Pubkey::from_str(user_pubkey)
            .map_err(|_| TransactionBuilderError::InvalidPubkey(user_pubkey.to_string()))?;

        let authority = Pubkey::from_str(authority_pubkey)
            .map_err(|_| TransactionBuilderError::InvalidPubkey(authority_pubkey.to_string()))?;

        let (vault_pda, _) = Pubkey::find_program_address(
            &[b"vault", user.as_ref()],
            &self.program_id,
        );

        let (vault_authority, _) = Pubkey::find_program_address(
            &[b"vault_authority"],
            &self.program_id,
        );

        // The authority (position manager) is the signer
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(authority, true),          // authority (signer - position manager)
                AccountMeta::new(vault_pda, false),         // vault
                AccountMeta::new_readonly(vault_authority, false), // vault_authority
            ],
            data: Self::build_unlock_data(amount),
        };

        let _blockhash = self
            .solana
            .get_recent_blockhash()
            .await
            .map_err(|e| TransactionBuilderError::BlockhashError(e.to_string()))?;

        // Authority (position manager) is the fee payer
        let message = Message::new(&[instruction], Some(&authority));
        let transaction = Transaction::new_unsigned(message);

        let tx_bytes = bincode::serialize(&transaction)
            .map_err(|e| TransactionBuilderError::SerializationError(e.to_string()))?;
        
        Ok(BASE64.encode(&tx_bytes))
    }

    /// Build a transfer_collateral transaction.
    ///
    /// Transfers collateral from one vault to another.
    /// Only callable by authorized programs (liquidation engine, settlement relayer).
    ///
    /// ## Arguments
    ///
    /// * `from_pubkey` - Source vault owner
    /// * `to_pubkey` - Destination vault owner
    /// * `amount` - Amount to transfer (in smallest units, 6 decimals)
    /// * `reason` - Transfer reason (0=settlement, 1=liquidation, 2=fee)
    /// * `authority_pubkey` - Public key of the authorized program (liquidation engine) that will sign
    ///
    /// ## Returns
    ///
    /// Base64-encoded unsigned transaction (must be signed by the authority)
    pub async fn build_transfer_collateral(
        &self,
        from_pubkey: &str,
        to_pubkey: &str,
        amount: u64,
        reason: u8,
        authority_pubkey: &str,
    ) -> Result<String, TransactionBuilderError> {
        info!("Building transfer_collateral transaction: {} USDT from {} to {} (authority: {})", 
            amount as f64 / 1_000_000.0, from_pubkey, to_pubkey, authority_pubkey);

        let from_user = Pubkey::from_str(from_pubkey)
            .map_err(|_| TransactionBuilderError::InvalidPubkey(from_pubkey.to_string()))?;
        let to_user = Pubkey::from_str(to_pubkey)
            .map_err(|_| TransactionBuilderError::InvalidPubkey(to_pubkey.to_string()))?;

        let authority = Pubkey::from_str(authority_pubkey)
            .map_err(|_| TransactionBuilderError::InvalidPubkey(authority_pubkey.to_string()))?;

        // Derive PDAs
        let (from_vault_pda, _) = Pubkey::find_program_address(
            &[b"vault", from_user.as_ref()],
            &self.program_id,
        );

        let (to_vault_pda, _) = Pubkey::find_program_address(
            &[b"vault", to_user.as_ref()],
            &self.program_id,
        );

        let (vault_authority, _) = Pubkey::find_program_address(
            &[b"vault_authority"],
            &self.program_id,
        );

        // Get token accounts
        let from_token_account = spl_associated_token_account::get_associated_token_address(
            &from_vault_pda,
            &self.usdt_mint,
        );

        let to_token_account = spl_associated_token_account::get_associated_token_address(
            &to_vault_pda,
            &self.usdt_mint,
        );

        // Build instruction
        // The authority (liquidation engine) is the signer
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(authority, true),              // authority (signer - liquidation engine)
                AccountMeta::new(from_vault_pda, false),        // from_vault
                AccountMeta::new(from_token_account, false),     // from_token_account
                AccountMeta::new(to_vault_pda, false),          // to_vault
                AccountMeta::new(to_token_account, false),      // to_token_account
                AccountMeta::new_readonly(vault_authority, false), // vault_authority
                AccountMeta::new_readonly(spl_token::id(), false), // token_program
            ],
            data: Self::build_transfer_data(amount, reason),
        };

        let _blockhash = self
            .solana
            .get_recent_blockhash()
            .await
            .map_err(|e| TransactionBuilderError::BlockhashError(e.to_string()))?;

        // Authority (liquidation engine) is the fee payer
        let message = Message::new(&[instruction], Some(&authority));
        let transaction = Transaction::new_unsigned(message);

        let tx_bytes = bincode::serialize(&transaction)
            .map_err(|e| TransactionBuilderError::SerializationError(e.to_string()))?;
        
        Ok(BASE64.encode(&tx_bytes))
    }

    // ==========================================
    // HELPER METHODS
    // ==========================================

    /// Build instruction data for deposit.
    /// 
    /// Uses the correct Anchor discriminator from the IDL.
    /// Discriminator: [242, 35, 198, 137, 82, 225, 242, 182] (0xf223c68952e1f2b6)
    fn build_deposit_data(amount: u64) -> Vec<u8> {
        // Anchor discriminator for "deposit" instruction
        // This is the first 8 bytes of sha256("global:deposit")
        let discriminator = vec![242, 35, 198, 137, 82, 225, 242, 182];
        let mut data = discriminator;
        // Append amount as little-endian u64 (8 bytes)
        data.extend_from_slice(&amount.to_le_bytes());
        data
    }

    /// Build instruction data for withdraw.
    /// Discriminator: [183, 18, 70, 156, 148, 109, 161, 34] (0xb712469c946da122)
    fn build_withdraw_data(amount: u64) -> Vec<u8> {
        let discriminator = vec![183, 18, 70, 156, 148, 109, 161, 34];
        let mut data = discriminator;
        data.extend_from_slice(&amount.to_le_bytes());
        data
    }

    /// Build instruction data for lock_collateral.
    /// Discriminator: [161, 216, 135, 122, 12, 104, 211, 101] (0xa1d8877a0c68d365)
    fn build_lock_data(amount: u64) -> Vec<u8> {
        let discriminator = vec![161, 216, 135, 122, 12, 104, 211, 101];
        let mut data = discriminator;
        data.extend_from_slice(&amount.to_le_bytes());
        data
    }

    /// Build instruction data for unlock_collateral.
    /// Discriminator: [167, 213, 221, 147, 129, 209, 132, 190] (0xa7d5dd9381d184be)
    fn build_unlock_data(amount: u64) -> Vec<u8> {
        let discriminator = vec![167, 213, 221, 147, 129, 209, 132, 190];
        let mut data = discriminator;
        data.extend_from_slice(&amount.to_le_bytes());
        data
    }

    /// Build instruction data for transfer_collateral.
    /// Discriminator: [157, 163, 63, 27, 242, 72, 251, 97] (0x9da33f1bf248fb61)
    fn build_transfer_data(amount: u64, reason: u8) -> Vec<u8> {
        let discriminator = vec![157, 163, 63, 27, 242, 72, 251, 97];
        let mut data = discriminator;
        data.extend_from_slice(&amount.to_le_bytes());
        data.push(reason); // Transfer reason: 0=settlement, 1=liquidation, 2=fee
        data
    }
}

