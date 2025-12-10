//! # Token Minter Service
//!
//! This service handles minting test USDT tokens on devnet.
//! **SECURITY: Only works on devnet!**
//!
//! ## Usage
//!
//! ```rust,ignore
//! let minter = TokenMinter::new(solana_client, config)?;
//! let signature = minter.mint_usdt("user_pubkey", 1000_000_000).await?;
//! ```

use std::str::FromStr;
use std::fs;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;
use spl_token::instruction::mint_to;
use spl_associated_token_account::get_associated_token_address;
use tracing::info;

use crate::config::AppConfig;

/// Errors that can occur when minting tokens.
#[derive(Debug, thiserror::Error)]
pub enum TokenMinterError {
    /// Not on devnet - minting only allowed on devnet.
    #[error("Minting is only allowed on devnet. Current RPC: {0}")]
    NotDevnet(String),

    /// Invalid public key format.
    #[error("Invalid public key: {0}")]
    InvalidPubkey(String),

    /// Failed to load keypair.
    #[error("Failed to load keypair: {0}")]
    KeypairError(String),

    /// Failed to get recent blockhash.
    #[error("Failed to get blockhash: {0}")]
    BlockhashError(String),

    /// RPC error.
    #[error("RPC error: {0}")]
    RpcError(String),

    /// Token account creation failed.
    #[error("Failed to create token account: {0}")]
    TokenAccountError(String),

    /// Minting failed.
    #[error("Minting failed: {0}")]
    MintError(String),
}

/// Token Minter service for devnet testing.
///
/// This service allows minting test USDT tokens to user accounts.
/// **IMPORTANT: Only works on devnet!**
pub struct TokenMinter {
    /// Application configuration.
    config: AppConfig,

    /// Backend keypair (mint authority).
    keypair: Keypair,

    /// USDT mint address.
    usdt_mint: Pubkey,
}

impl TokenMinter {
    /// Create a new TokenMinter.
    ///
    /// ## Arguments
    ///
    /// * `config` - Application configuration
    ///
    /// ## Returns
    ///
    /// * `Ok(TokenMinter)` - Minter created successfully
    /// * `Err(...)` - Failed to create minter
    ///
    /// ## Security
    ///
    /// This will check that we're on devnet and fail if not.
    pub fn new(config: AppConfig) -> Result<Self, TokenMinterError> {
        // SECURITY: Only allow on devnet!
        if !config.solana_rpc_url.contains("devnet") 
            && !config.solana_rpc_url.contains("localhost")
            && !config.solana_rpc_url.contains("127.0.0.1") {
            return Err(TokenMinterError::NotDevnet(config.solana_rpc_url.clone()));
        }

        // Load backend keypair (mint authority)
        let keypair_path = shellexpand::full(&config.keypair_path)
            .map_err(|e| TokenMinterError::KeypairError(format!("Invalid path: {}", e)))?;
        
        let keypair_bytes = fs::read_to_string(keypair_path.as_ref())
            .map_err(|e| TokenMinterError::KeypairError(format!("Failed to read keypair file: {}", e)))?;
        
        let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_bytes)
            .map_err(|e| TokenMinterError::KeypairError(format!("Failed to parse keypair: {}", e)))?;
        
        let keypair = Keypair::from_bytes(&keypair_bytes)
            .map_err(|e| TokenMinterError::KeypairError(format!("Invalid keypair format: {}", e)))?;

        let usdt_mint = Pubkey::from_str(&config.usdt_mint)
            .map_err(|e| TokenMinterError::InvalidPubkey(format!("Invalid USDT mint: {}", e)))?;

        info!("TokenMinter initialized (devnet only)");
        info!("  Mint authority: {}", keypair.pubkey());
        info!("  USDT mint: {}", usdt_mint);

        Ok(Self {
            config,
            keypair,
            usdt_mint,
        })
    }

    /// Mint USDT tokens to a user's token account.
    ///
    /// ## Arguments
    ///
    /// * `user_pubkey` - User's wallet public key (base58)
    /// * `amount` - Amount to mint (smallest units, 6 decimals)
    ///
    /// ## Returns
    ///
    /// Transaction signature if successful.
    ///
    /// ## Process
    ///
    /// 1. Get or create user's associated token account
    /// 2. Build mint instruction
    /// 3. Sign and submit transaction
    pub async fn mint_usdt(
        &self,
        user_pubkey: &str,
        amount: u64,
    ) -> Result<String, TokenMinterError> {
        info!("Minting {} USDT to {}", amount as f64 / 1_000_000.0, user_pubkey);

        // Validate amount
        if amount == 0 {
            return Err(TokenMinterError::MintError("Amount must be greater than 0".to_string()));
        }

        // Parse user pubkey
        let user = Pubkey::from_str(user_pubkey)
            .map_err(|_| TokenMinterError::InvalidPubkey(user_pubkey.to_string()))?;

        // Get user's associated token account
        let user_token_account = get_associated_token_address(&user, &self.usdt_mint);

        // Use actix_web::web::block for blocking operations
        use actix_web::web;

        // Serialize keypair for moving into closure
        let keypair_bytes = self.keypair.to_bytes();
        let rpc_url = self.config.solana_rpc_url.clone();
        let user_token_account_clone = user_token_account;

        // Check if token account exists, create if not
        let account_exists = web::block(move || {
            let rpc_client = RpcClient::new(rpc_url.clone());
            rpc_client.get_account(&user_token_account_clone).is_ok()
        }).await
        .map_err(|e| TokenMinterError::RpcError(format!("Failed to check account: {}", e)))?;

        if !account_exists {
            info!("Creating token account for user: {}", user_token_account);
            self.create_token_account(&user, &user_token_account).await?;
        }

        // Get recent blockhash and mint
        let rpc_url_clone = self.config.solana_rpc_url.clone();
        let usdt_mint_clone = self.usdt_mint;
        let keypair_bytes_clone = keypair_bytes.clone();
        let user_token_account_clone = user_token_account;

        let signature = web::block(move || {
            let rpc_client = RpcClient::new(rpc_url_clone);
            let keypair = Keypair::from_bytes(&keypair_bytes_clone)
                .map_err(|e| TokenMinterError::KeypairError(format!("Failed to recreate keypair: {}", e)))?;

            // Get recent blockhash
            let recent_blockhash = rpc_client.get_latest_blockhash()
                .map_err(|e| TokenMinterError::BlockhashError(e.to_string()))?;

            // Build mint instruction
            let mint_instruction = mint_to(
                &spl_token::id(),
                &usdt_mint_clone,
                &user_token_account_clone,
                &keypair.pubkey(),
                &[],
                amount,
            )
            .map_err(|e| TokenMinterError::MintError(format!("Failed to build mint instruction: {}", e)))?;

            // Build transaction
            let mut transaction = Transaction::new_with_payer(
                &[mint_instruction],
                Some(&keypair.pubkey()),
            );
            transaction.sign(&[&keypair], recent_blockhash);

            // Submit transaction
            let signature = rpc_client.send_and_confirm_transaction(&transaction)
                .map_err(|e| TokenMinterError::RpcError(format!("Failed to submit transaction: {}", e)))?;

            Ok::<String, TokenMinterError>(signature.to_string())
        }).await
        .map_err(|e| TokenMinterError::RpcError(format!("Blocking task failed: {}", e)))?
        .map_err(|e| TokenMinterError::RpcError(e.to_string()))?;

        info!("✓ Minted {} USDT to {}", amount as f64 / 1_000_000.0, user_pubkey);
        info!("  Transaction: {}", signature);

        Ok(signature)
    }

    /// Create an associated token account for a user.
    ///
    /// This is a helper method that creates the token account
    /// if it doesn't exist.
    async fn create_token_account(
        &self,
        user: &Pubkey,
        _token_account: &Pubkey,
    ) -> Result<(), TokenMinterError> {
        use actix_web::web;
        use spl_associated_token_account::instruction::create_associated_token_account;

        let rpc_url = self.config.solana_rpc_url.clone();
        let keypair_bytes = self.keypair.to_bytes();
        let usdt_mint = self.usdt_mint;
        let user_clone = *user;

        web::block(move || {
            let rpc_client = RpcClient::new(rpc_url);
            let keypair = Keypair::from_bytes(&keypair_bytes)
                .map_err(|e| TokenMinterError::KeypairError(format!("Failed to recreate keypair: {}", e)))?;

            // Build create ATA instruction
            let create_ata_instruction = create_associated_token_account(
                &keypair.pubkey(),  // payer
                &user_clone,        // owner
                &usdt_mint,         // mint
                &spl_token::id(),   // token program
            );

            // Get recent blockhash
            let recent_blockhash = rpc_client.get_latest_blockhash()
                .map_err(|e| TokenMinterError::BlockhashError(e.to_string()))?;

            // Build transaction
            let mut transaction = Transaction::new_with_payer(
                &[create_ata_instruction],
                Some(&keypair.pubkey()),
            );
            transaction.sign(&[&keypair], recent_blockhash);

            // Submit transaction
            rpc_client.send_and_confirm_transaction(&transaction)
                .map_err(|e| TokenMinterError::TokenAccountError(format!("Failed to create token account: {}", e)))?;

            Ok::<(), TokenMinterError>(())
        }).await
        .map_err(|e| TokenMinterError::TokenAccountError(format!("Blocking task failed: {}", e)))?
        .map_err(|e| TokenMinterError::TokenAccountError(e.to_string()))?;

        info!("✓ Created token account");
        Ok(())
    }
}
