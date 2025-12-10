//! # Solana Client Module
//!
//! This module provides a client for interacting with the Solana blockchain.
//! It wraps the Solana SDK and provides high-level methods for vault operations.
//!
//! ## Responsibilities
//!
//! - Connect to Solana RPC
//! - Fetch vault account data
//! - Submit transactions
//! - Subscribe to account changes
//! - Monitor transaction confirmations
//!
//! ## Connection Types
//!
//! | Type | Use Case |
//! |------|----------|
//! | HTTP RPC | Queries, transaction submission |
//! | WebSocket | Real-time event subscriptions |
//!
//! ## Account Data Flow
//!
//! ```text
//! 1. Backend requests vault data
//!              ↓
//! 2. SolanaClient.get_vault_account()
//!              ↓
//! 3. Solana RPC returns raw account data
//!              ↓
//! 4. Deserialize Anchor account structure
//!              ↓
//! 5. Return typed VaultAccountData
//! ```

use std::str::FromStr;
use std::time::Duration;
use chrono::{DateTime, Utc, TimeZone};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    pubkey::Pubkey,
    signature::Signature,
};
use tracing::{info, debug, warn};
use tokio::time::timeout;

use crate::config::AppConfig;

/// Vault account data fetched from the blockchain.
///
/// This represents the deserialized CollateralVault account
/// from the Anchor program.
#[derive(Debug, Clone)]
pub struct VaultAccountData {
    /// Vault PDA address.
    pub vault_address: String,

    /// Associated token account.
    pub token_account: String,

    /// Total balance (smallest units).
    pub total_balance: u64,

    /// Locked balance.
    pub locked_balance: u64,

    /// Available balance.
    pub available_balance: u64,

    /// Lifetime deposits.
    pub total_deposited: u64,

    /// Lifetime withdrawals.
    pub total_withdrawn: u64,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Solana RPC client wrapper.
///
/// Provides high-level methods for interacting with Solana
/// and the Collateral Vault program.
///
/// ## Usage
///
/// ```rust,ignore
/// let config = AppConfig::from_env()?;
/// let client = SolanaClient::new(&config)?;
///
/// // Get vault data
/// let vault = client.get_vault_account("7xKt9Fj2...").await?;
/// println!("Balance: {}", vault.total_balance);
///
/// // Check health
/// let healthy = client.get_health().await?;
/// ```
#[derive(Clone)]
pub struct SolanaClient {
    /// The RPC endpoint URL.
    rpc_url: String,

    /// The vault program ID.
    program_id: Pubkey,

    /// USDT token mint.
    usdt_mint: Pubkey,
}

impl SolanaClient {
    /// Create a new SolanaClient.
    ///
    /// ## Arguments
    ///
    /// * `config` - Application configuration containing RPC URL and program ID
    ///
    /// ## Returns
    ///
    /// * `Ok(SolanaClient)` - Client created successfully
    /// * `Err(...)` - Invalid configuration
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let config = AppConfig::from_env()?;
    /// let client = SolanaClient::new(&config)?;
    /// ```
    pub fn new(config: &AppConfig) -> Result<Self, String> {
        let program_id = Pubkey::from_str(&config.vault_program_id)
            .map_err(|e| format!("Invalid program ID: {}", e))?;

        let usdt_mint = Pubkey::from_str(&config.usdt_mint)
            .map_err(|e| format!("Invalid USDT mint: {}", e))?;

        info!("Solana client initialized:");
        info!("  RPC: {}", config.solana_rpc_url);
        info!("  Program: {}", program_id);
        info!("  USDT Mint: {}", usdt_mint);

        Ok(Self {
            rpc_url: config.solana_rpc_url.clone(),
            program_id,
            usdt_mint,
        })
    }

    /// Get an RPC client instance.
    ///
    /// Creates a new RpcClient for each call. In production,
    /// you might want to pool connections.
    fn get_rpc_client(&self) -> RpcClient {
        RpcClient::new_with_commitment(
            self.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        )
    }

    /// Execute an RPC operation with retry logic.
    ///
    /// Retries up to 4 attempts (initial attempt + 3 retries) with exponential backoff
    /// on network errors. Uses a 10-second timeout per attempt.
    /// This helps reduce error rates when connecting to api.devnet.solana.com.
    ///
    /// ## Arguments
    ///
    /// * `operation` - Async closure that performs the RPC operation
    ///
    /// ## Returns
    ///
    /// * `Ok(T)` - Operation succeeded (on any attempt)
    /// * `Err(String)` - Operation failed after all retries (4 total attempts)
    async fn retry_rpc_operation<F, Fut, T>(&self, mut operation: F) -> Result<T, String>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        const MAX_RETRIES: u32 = 3;
        const INITIAL_DELAY_MS: u64 = 200;
        const TIMEOUT_SECS: u64 = 10;

        for attempt in 0..=MAX_RETRIES {
            match timeout(Duration::from_secs(TIMEOUT_SECS), operation()).await {
                Ok(Ok(result)) => {
                    if attempt > 0 {
                        info!("RPC operation succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    // RPC call completed but returned an error
                    if attempt < MAX_RETRIES {
                        let delay_ms = INITIAL_DELAY_MS * (1 << attempt); // Exponential backoff: 200ms, 400ms, 800ms, 1600ms, 3200ms
                        debug!("RPC operation failed (attempt {}): {}. Retrying in {}ms...", 
                            attempt + 1, e, delay_ms);
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    } else {
                        warn!("RPC operation failed after {} attempts: {}", MAX_RETRIES + 1, e);
                        return Err(e);
                    }
                }
                Err(_) => {
                    // Timeout
                    if attempt < MAX_RETRIES {
                        let delay_ms = INITIAL_DELAY_MS * (1 << attempt);
                        debug!("RPC operation timed out (attempt {}). Retrying in {}ms...", 
                            attempt + 1, delay_ms);
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    } else {
                        warn!("RPC operation timed out after {} attempts ({}s timeout)", 
                            MAX_RETRIES + 1, TIMEOUT_SECS);
                        return Err(format!("RPC operation timed out after {} attempts", MAX_RETRIES + 1));
                    }
                }
            }
        }

        Err("RPC operation failed after all retries".to_string())
    }

    /// Check if the Solana RPC is healthy.
    ///
    /// Makes a simple request to verify connectivity with timeout and retry logic.
    ///
    /// ## Returns
    ///
    /// * `Ok(true)` - RPC is responding
    /// * `Ok(false)` - RPC is not healthy (after retries)
    /// * `Err(...)` - Connection error
    pub async fn get_health(&self) -> Result<bool, String> {
        const MAX_RETRIES: u32 = 2;
        const TIMEOUT_SECS: u64 = 5;
        
        // Retry logic with exponential backoff
        for attempt in 0..=MAX_RETRIES {
            match timeout(
                Duration::from_secs(TIMEOUT_SECS),
                self.get_slot()
            ).await {
                Ok(Ok(_)) => {
                    // Success - reset any previous warnings
                    if attempt > 0 {
                        info!("Solana RPC health check recovered after {} retries", attempt);
                    }
                    return Ok(true);
                }
                Ok(Err(e)) => {
                    // RPC call succeeded but returned an error
                    if attempt < MAX_RETRIES {
                        let delay_ms = 500 * (attempt + 1) as u64;
                        debug!("Solana RPC health check attempt {} failed: {}. Retrying in {}ms...", 
                            attempt + 1, e, delay_ms);
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    } else {
                        // Only warn on final failure, not every single timeout
                        warn!("Solana RPC health check failed after {} attempts: {}", 
                            MAX_RETRIES + 1, e);
                        return Ok(false);
                    }
                }
                Err(_) => {
                    // Timeout
                    if attempt < MAX_RETRIES {
                        let delay_ms = 500 * (attempt + 1) as u64;
                        debug!("Solana RPC health check timed out (attempt {}). Retrying in {}ms...", 
                            attempt + 1, delay_ms);
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    } else {
                        // Only warn on final failure
                        warn!("Solana RPC health check timed out after {} attempts ({}s timeout)", 
                            MAX_RETRIES + 1, TIMEOUT_SECS);
                        return Ok(false);
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Get the current slot.
    ///
    /// The slot is Solana's block number equivalent.
    pub async fn get_slot(&self) -> Result<u64, String> {
        use actix_web::web;
        
        let rpc_url = self.rpc_url.clone();
        self.retry_rpc_operation(|| {
            let rpc_url = rpc_url.clone();
            async move {
                let client = RpcClient::new_with_commitment(
                    rpc_url,
                    CommitmentConfig::confirmed(),
                );
                web::block(move || client.get_slot())
                    .await
                    .map_err(|e| format!("Failed to execute blocking task: {}", e))?
                    .map_err(|e| format!("Failed to get slot: {}", e))
            }
        }).await
    }

    /// Get a recent blockhash.
    ///
    /// Required for building transactions. Blockhashes expire
    /// after about 2 minutes.
    ///
    /// ## Returns
    ///
    /// The most recent blockhash as a Hash.
    pub async fn get_recent_blockhash(&self) -> Result<Hash, String> {
        use actix_web::web;
        
        let rpc_url = self.rpc_url.clone();
        let blockhash = self.retry_rpc_operation(|| {
            let rpc_url = rpc_url.clone();
            async move {
                let client = RpcClient::new_with_commitment(
                    rpc_url,
                    CommitmentConfig::confirmed(),
                );
                web::block(move || client.get_latest_blockhash())
                    .await
                    .map_err(|e| format!("Failed to execute blocking task: {}", e))?
                    .map_err(|e| format!("Failed to get blockhash: {}", e))
            }
        }).await?;

        debug!("Got recent blockhash: {}", blockhash);
        Ok(blockhash)
    }

    /// Get vault account data for a user.
    ///
    /// Fetches the vault PDA account and deserializes it.
    ///
    /// ## Arguments
    ///
    /// * `owner` - The vault owner's public key (base58)
    ///
    /// ## Returns
    ///
    /// * `Ok(Some(VaultAccountData))` - Vault found and deserialized
    /// * `Ok(None)` - Vault doesn't exist
    /// * `Err(...)` - RPC or deserialization error
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let vault = client.get_vault_account("7xKt9Fj2...").await?;
    /// if let Some(v) = vault {
    ///     println!("Balance: {} USDT", v.total_balance / 1_000_000);
    /// }
    /// ```
    pub async fn get_vault_account(
        &self,
        owner: &str,
    ) -> Result<Option<VaultAccountData>, String> {
        debug!("Fetching vault account for: {}", owner);

        let owner_pubkey = Pubkey::from_str(owner)
            .map_err(|e| format!("Invalid owner pubkey: {}", e))?;

        // Derive the vault PDA
        let (vault_pda, _bump) = Pubkey::find_program_address(
            &[b"vault", owner_pubkey.as_ref()],
            &self.program_id,
        );

        debug!("Vault PDA: {}", vault_pda);

        let rpc_url = self.rpc_url.clone();
        let vault_pda_clone = vault_pda;
        let program_id = self.program_id;

        // Fetch the account with retry logic
        use actix_web::web;
        match self.retry_rpc_operation(|| {
            let rpc_url = rpc_url.clone();
            let vault_pda = vault_pda_clone;
            async move {
                let client = RpcClient::new_with_commitment(
                    rpc_url,
                    CommitmentConfig::confirmed(),
                );
                web::block(move || client.get_account(&vault_pda))
                    .await
                    .map_err(|e| format!("Failed to spawn blocking task: {}", e))?
                    .map_err(|e| format!("Failed to fetch vault: {}", e))
            }
        }).await {
            Ok(account) => {
                // Deserialize the account data
                // Note: In production, use Anchor's account deserialization
                let vault_data = self.deserialize_vault_account(&account.data, &vault_pda)?;
                Ok(Some(vault_data))
            }
            Err(e) => {
                // Check if it's a "not found" error
                if e.contains("AccountNotFound") {
                    debug!("Vault not found for: {}", owner);
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Deserialize vault account data.
    ///
    /// Parses the raw account bytes into a VaultAccountData struct.
    ///
    /// ## Account Structure (Anchor)
    ///
    /// ```text
    /// Offset | Size | Field
    /// -------|------|------
    /// 0      | 8    | Anchor discriminator
    /// 8      | 32   | owner
    /// 40     | 32   | token_account
    /// 72     | 8    | total_balance
    /// 80     | 8    | locked_balance
    /// 88     | 8    | available_balance
    /// 96     | 8    | total_deposited
    /// 104    | 8    | total_withdrawn
    /// 112    | 8    | created_at (i64)
    /// 120    | 1    | bump
    /// ```
    fn deserialize_vault_account(
        &self,
        data: &[u8],
        vault_address: &Pubkey,
    ) -> Result<VaultAccountData, String> {
        if data.len() < 121 {
            return Err("Account data too short".to_string());
        }

        // Skip 8-byte Anchor discriminator
        let data = &data[8..];

        // Parse fields
        // owner: [0..32]
        // token_account: [32..64]
        let token_account = Pubkey::try_from(&data[32..64])
            .map_err(|_| "Invalid token account")?;

        // total_balance: [64..72]
        let total_balance = u64::from_le_bytes(
            data[64..72].try_into().map_err(|_| "Invalid total_balance")?
        );

        // locked_balance: [72..80]
        let locked_balance = u64::from_le_bytes(
            data[72..80].try_into().map_err(|_| "Invalid locked_balance")?
        );

        // available_balance: [80..88]
        let available_balance = u64::from_le_bytes(
            data[80..88].try_into().map_err(|_| "Invalid available_balance")?
        );

        // total_deposited: [88..96]
        let total_deposited = u64::from_le_bytes(
            data[88..96].try_into().map_err(|_| "Invalid total_deposited")?
        );

        // total_withdrawn: [96..104]
        let total_withdrawn = u64::from_le_bytes(
            data[96..104].try_into().map_err(|_| "Invalid total_withdrawn")?
        );

        // created_at: [104..112]
        let created_at_timestamp = i64::from_le_bytes(
            data[104..112].try_into().map_err(|_| "Invalid created_at")?
        );

        let created_at = Utc.timestamp_opt(created_at_timestamp, 0)
            .single()
            .unwrap_or_else(Utc::now);

        Ok(VaultAccountData {
            vault_address: vault_address.to_string(),
            token_account: token_account.to_string(),
            total_balance,
            locked_balance,
            available_balance,
            total_deposited,
            total_withdrawn,
            created_at,
        })
    }

    /// Get the token balance of an account.
    ///
    /// Fetches the actual SPL token balance from a token account.
    ///
    /// ## Arguments
    ///
    /// * `token_account` - The token account address (base58)
    ///
    /// ## Returns
    ///
    /// Token balance in smallest units.
    pub async fn get_token_balance(
        &self,
        token_account: &str,
    ) -> Result<u64, String> {
        let token_pubkey = Pubkey::from_str(token_account)
            .map_err(|e| format!("Invalid token account: {}", e))?;

        let rpc_url = self.rpc_url.clone();
        let token_pubkey_clone = token_pubkey;

        use actix_web::web;
        let balance = self.retry_rpc_operation(|| {
            let rpc_url = rpc_url.clone();
            let token_pubkey = token_pubkey_clone;
            async move {
                let client = RpcClient::new_with_commitment(
                    rpc_url,
                    CommitmentConfig::confirmed(),
                );
                web::block(move || client.get_token_account_balance(&token_pubkey))
                    .await
                    .map_err(|e| format!("Failed to execute blocking task: {}", e))?
                    .map_err(|e| format!("Failed to get token balance: {}", e))
            }
        }).await?;

        let amount = balance
            .amount
            .parse::<u64>()
            .map_err(|e| format!("Invalid balance amount: {}", e))?;

        Ok(amount)
    }

    /// Check if a transaction is confirmed.
    ///
    /// ## Arguments
    ///
    /// * `signature` - Transaction signature (base58)
    ///
    /// ## Returns
    ///
    /// * `Ok(true)` - Transaction is confirmed
    /// * `Ok(false)` - Transaction not found or not confirmed
    /// * `Err(...)` - RPC error
    pub async fn is_transaction_confirmed(
        &self,
        signature: &str,
    ) -> Result<bool, String> {
        let sig = Signature::from_str(signature)
            .map_err(|e| format!("Invalid signature: {}", e))?;

        let rpc_url = self.rpc_url.clone();
        let sig_clone = sig;

        use actix_web::web;
        match self.retry_rpc_operation(|| {
            let rpc_url = rpc_url.clone();
            let sig = sig_clone;
            async move {
                let client = RpcClient::new_with_commitment(
                    rpc_url,
                    CommitmentConfig::confirmed(),
                );
                web::block(move || client.get_signature_status(&sig))
                    .await
                    .map_err(|e| format!("Failed to execute blocking task: {}", e))?
                    .map_err(|e| format!("Failed to get signature status: {}", e))
            }
        }).await {
            Ok(Some(status)) => {
                match status {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
            Ok(None) => Ok(false),
            Err(e) => Err(format!("Failed to check signature: {}", e)),
        }
    }

    /// Get the vault program ID.
    pub fn program_id(&self) -> &Pubkey {
        &self.program_id
    }

    /// Get the USDT mint address.
    pub fn usdt_mint(&self) -> &Pubkey {
        &self.usdt_mint
    }
}

