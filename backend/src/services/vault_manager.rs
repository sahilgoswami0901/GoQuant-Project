//! # Vault Manager Service
//!
//! The VaultManager is the central service for managing vault operations.
//! It coordinates between the database, Solana blockchain, and other services.
//!
//! ## Responsibilities
//!
//! - Initialize new vaults for users
//! - Process deposit requests
//! - Handle withdrawal requests
//! - Query vault balances
//! - Track transaction history
//!
//! ## Flow Example: Deposit
//!
//! ```text
//! 1. User requests deposit via API
//!                ↓
//! 2. VaultManager.deposit() called
//!                ↓
//! 3. TransactionBuilder builds Solana tx
//!                ↓
//! 4. Transaction sent to blockchain
//!                ↓
//! 5. Wait for confirmation
//!                ↓
//! 6. Update database with new balance
//!                ↓
//! 7. Return success to user
//! ```

use chrono::Utc;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::db::{Database, VaultRecord, TransactionRecord};
use crate::db::queries;
use crate::models::{InitializeVaultRequest, DepositRequest, WithdrawRequest, VaultBalanceResponse, OperationResponse};
use crate::solana::SolanaClient;

use super::{TransactionBuilder, TransactionSubmitter};

/// Errors that can occur in vault operations.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    /// Vault not found for the given user.
    #[error("Vault not found for user: {0}")]
    VaultNotFound(String),

    /// Insufficient balance for the operation.
    #[error("Insufficient balance: available {available}, requested {requested}")]
    InsufficientBalance { available: u64, requested: u64 },

    /// Database operation failed.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Solana transaction failed.
    #[error("Transaction failed: {0}")]
    TransactionError(String),

    /// Invalid input provided.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// System is paused.
    #[error("System is currently paused")]
    SystemPaused,
}

/// The main service for managing vault operations.
///
/// VaultManager coordinates:
/// - Database reads/writes for vault state
/// - Solana transactions for on-chain operations
/// - Transaction history tracking
///
/// ## Usage
///
/// ```rust,ignore
/// let manager = VaultManager::new(db, solana, config);
///
/// // Get vault balance
/// let balance = manager.get_vault_balance("7xKt9Fj2...").await?;
///
/// // Process a deposit
/// let result = manager.deposit(request).await?;
/// ```
#[derive(Clone)]
pub struct VaultManager {
    /// Database connection for storing vault state.
    db: Database,

    /// Solana client for blockchain operations.
    solana: SolanaClient,

    /// Application configuration.
    #[allow(dead_code)]
    config: AppConfig,

    /// Transaction builder for creating Solana transactions.
    tx_builder: TransactionBuilder,

    /// Transaction submitter for signing and submitting transactions.
    tx_submitter: TransactionSubmitter,
}

impl VaultManager {
    /// Create a new VaultManager instance.
    ///
    /// ## Arguments
    ///
    /// * `db` - Database connection
    /// * `solana` - Solana RPC client
    /// * `config` - Application configuration
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let manager = VaultManager::new(db, solana, config);
    /// ```
    pub fn new(db: Database, solana: SolanaClient, config: AppConfig) -> Self {
        let tx_builder = TransactionBuilder::new(solana.clone(), config.clone());
        let tx_submitter = TransactionSubmitter::new(config.clone());

        Self {
            db,
            solana,
            config,
            tx_builder,
            tx_submitter,
        }
    }

    // ==========================================
    // VAULT QUERIES
    // ==========================================

    /// Get the current balance of a vault.
    ///
    /// This first checks the database cache, then optionally
    /// refreshes from the blockchain for accuracy.
    ///
    /// ## Arguments
    ///
    /// * `owner` - The vault owner's public key (base58 encoded)
    ///
    /// ## Returns
    ///
    /// * `Ok(VaultBalanceResponse)` - Vault balance information
    /// * `Err(VaultError::VaultNotFound)` - Vault doesn't exist
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let balance = manager.get_vault_balance("7xKt9Fj2...").await?;
    /// println!("Available: {} USDT", balance.available_balance / 1_000_000);
    /// ```
    pub async fn get_vault_balance(
        &self,
        owner: &str,
    ) -> Result<VaultBalanceResponse, VaultError> {
        debug!("Getting vault balance for: {}", owner);

        // First, try to get from database
        let vault = queries::get_vault_by_owner(self.db.pool(), owner)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        match vault {
            Some(v) => {
                // Optionally refresh from blockchain
                // (For now, trust the cache)
                Ok(VaultBalanceResponse {
                    owner: v.owner.clone(),
                    vault_address: v.vault_address,
                    token_account: v.token_account,
                    total_balance: v.total_balance,
                    locked_balance: v.locked_balance,
                    available_balance: v.available_balance,
                    total_deposited: v.total_deposited,
                    total_withdrawn: v.total_withdrawn,
                    formatted_total: VaultBalanceResponse::format_usdt(v.total_balance),
                    formatted_available: VaultBalanceResponse::format_usdt(v.available_balance),
                    created_at: v.created_at,
                    last_updated: v.updated_at,
                })
            }
            None => {
                // Vault not in database - try to fetch from blockchain
                info!("Vault not in cache, checking blockchain for: {}", owner);
                
                match self.fetch_vault_from_chain(owner).await {
                    Ok(Some(v)) => {
                        // Cache it for next time
                        if let Err(e) = queries::upsert_vault(self.db.pool(), &v).await {
                            warn!("Failed to cache vault: {}", e);
                        }
                        
                        Ok(VaultBalanceResponse {
                            owner: v.owner.clone(),
                            vault_address: v.vault_address,
                            token_account: v.token_account,
                            total_balance: v.total_balance,
                            locked_balance: v.locked_balance,
                            available_balance: v.available_balance,
                            total_deposited: v.total_deposited,
                            total_withdrawn: v.total_withdrawn,
                            formatted_total: VaultBalanceResponse::format_usdt(v.total_balance),
                            formatted_available: VaultBalanceResponse::format_usdt(v.available_balance),
                            created_at: v.created_at,
                            last_updated: v.updated_at,
                        })
                    }
                    Ok(None) => Err(VaultError::VaultNotFound(owner.to_string())),
                    Err(e) => Err(VaultError::TransactionError(e.to_string())),
                }
            }
        }
    }

    /// Fetch vault data directly from the blockchain.
    ///
    /// This bypasses the database cache and queries Solana directly.
    /// Useful for reconciliation or when cache might be stale.
    async fn fetch_vault_from_chain(
        &self,
        owner: &str,
    ) -> Result<Option<VaultRecord>, VaultError> {
        debug!("Fetching vault from blockchain: {}", owner);

        // Get vault account from Solana
        let vault_data = self
            .solana
            .get_vault_account(owner)
            .await
            .map_err(|e| VaultError::TransactionError(e.to_string()))?;

        match vault_data {
            Some(data) => {
                // Convert Solana account data to our VaultRecord
                Ok(Some(VaultRecord {
                    owner: owner.to_string(),
                    vault_address: data.vault_address,
                    token_account: data.token_account,
                    total_balance: data.total_balance as i64,
                    locked_balance: data.locked_balance as i64,
                    available_balance: data.available_balance as i64,
                    total_deposited: data.total_deposited as i64,
                    total_withdrawn: data.total_withdrawn as i64,
                    created_at: data.created_at,
                    updated_at: Utc::now(),
                    status: "active".to_string(),
                }))
            }
            None => Ok(None),
        }
    }

    // ==========================================
    // VAULT OPERATIONS
    // ==========================================

    /// Initialize a new vault for a user.
    ///
    /// This creates the vault on-chain and in our database.
    ///
    /// ## Arguments
    ///
    /// * `request` - Initialize vault request with user pubkey and optional keypair path
    ///
    /// ## Returns
    ///
    /// * `Ok(OperationResponse)` - Contains unsigned transaction or submitted signature
    /// * `Err(VaultError)` - Vault already exists or other error
    ///
    /// ## Flow
    ///
    /// ```text
    /// 1. Check if vault already exists
    /// 2. Build initialize_vault transaction
    /// 3. If keypair provided: sign and submit automatically
    /// 4. Otherwise: return unsigned tx for user to sign
    /// ```
    pub async fn initialize_vault(
        &self,
        request: InitializeVaultRequest,
    ) -> Result<OperationResponse, VaultError> {
        info!("Initializing vault for user: {}", request.user_pubkey);

        // Check if vault already exists
        let existing = queries::get_vault_by_owner(self.db.pool(), &request.user_pubkey)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        if existing.is_some() {
            return Err(VaultError::InvalidInput(
                "Vault already exists for this user".to_string(),
            ));
        }

        // Build the initialize vault transaction
        let (unsigned_tx, _vault_pda) = self
            .tx_builder
            .build_initialize_vault(&request.user_pubkey)
            .await
            .map_err(|e| VaultError::TransactionError(e.to_string()))?;

        // Create a pending transaction record
        let tx_id = Uuid::new_v4();
        let _tx_record = TransactionRecord {
            id: tx_id,
            vault_owner: request.user_pubkey.clone(),
            transaction_type: "initialize".to_string(),
            amount: 0,
            signature: None,
            status: "pending".to_string(),
            balance_before: 0,
            balance_after: 0,
            counterparty: None,
            note: Some("Vault initialization".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            confirmed_at: None,
        };

        // If user provided keypair path, sign and submit automatically
        if let Some(keypair_path) = &request.user_keypair_path {
            info!("Auto-signing and submitting transaction with user keypair...");
            // First, sign the transaction
            let (signed_tx, tx_signature) = match self.tx_submitter.sign_transaction(&unsigned_tx, keypair_path).await {
                Ok((signed, sig)) => (signed, sig),
                Err(e) => {
                    error!("Failed to sign transaction: {}", e);
                    // Return unsigned transaction if signing failed
                    return Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(unsigned_tx),
                        signature: None,
                        message: format!(
                            "Failed to sign transaction: {}. Please sign and submit manually.",
                            e
                        ),
                    });
                }
            };
            
            info!("✅ Transaction signed with signature: {}", tx_signature);
            
            // Try to submit the signed transaction
            match self.tx_submitter.submit_signed_transaction(&signed_tx).await {
                Ok(signature) => {
                    let signature_clone = signature.clone();
                    
                    // Update transaction record with signature
                    queries::update_transaction_status(
                        self.db.pool(),
                        tx_id,
                        "submitted",
                        Some(&signature),
                    )
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

                    info!("✅ Transaction submitted: {}", signature_clone);

                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "submitted".to_string(),
                        unsigned_transaction: None,
                        signature: Some(signature),
                        message: format!(
                            "Transaction submitted successfully. Signature: {}",
                            signature_clone
                        ),
                    })
                }
                Err(e) => {
                    error!("Failed to submit transaction: {}", e);
                    let sig_clone = tx_signature.clone();
                    // Return signed transaction with its signature even if submission failed
                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(signed_tx),
                        signature: Some(tx_signature),
                        message: format!(
                            "Transaction signed (signature: {}) but failed to submit: {}. You can submit the signed transaction manually using the signature or the transaction data.",
                            sig_clone, e
                        ),
                    })
                }
            }
        } else {
            // Return unsigned transaction for user to sign
        Ok(OperationResponse {
            transaction_id: tx_id,
            status: "pending".to_string(),
            unsigned_transaction: Some(unsigned_tx),
            signature: None,
            message: "Sign and submit this transaction to initialize your vault".to_string(),
        })
        }
    }

    /// Process a deposit request.
    ///
    /// Builds a transaction to deposit USDT into the user's vault.
    ///
    /// ## Arguments
    ///
    /// * `request` - Deposit request with user pubkey and amount
    ///
    /// ## Returns
    ///
    /// * `Ok(OperationResponse)` - Contains unsigned transaction
    /// * `Err(VaultError)` - Validation or build error
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let request = DepositRequest {
    ///     user_pubkey: "7xKt9Fj2...".to_string(),
    ///     amount: 100_000_000, // 100 USDT
    ///     signature: None,
    /// };
    /// let result = manager.deposit(request).await?;
    /// ```
    pub async fn deposit(
        &self,
        request: DepositRequest,
    ) -> Result<OperationResponse, VaultError> {
        info!(
            "Processing deposit: {} USDT for {}",
            request.amount as f64 / 1_000_000.0,
            request.user_pubkey
        );

        // Validate amount
        if request.amount == 0 {
            return Err(VaultError::InvalidInput("Amount must be greater than 0".to_string()));
        }

        // Get current vault state - check database first
        let mut vault = queries::get_vault_by_owner(self.db.pool(), &request.user_pubkey)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If not in database, check on-chain and sync
        if vault.is_none() {
            info!("Vault not in database, checking on-chain...");
            if let Some(on_chain_vault) = self.fetch_vault_from_chain(&request.user_pubkey).await? {
                info!("Found vault on-chain, syncing to database...");
                queries::upsert_vault(self.db.pool(), &on_chain_vault)
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;
                vault = Some(on_chain_vault);
            }
        }

        // If still not found, vault doesn't exist
        let vault = vault.ok_or_else(|| VaultError::VaultNotFound(request.user_pubkey.clone()))?;

        // Check if user's token account exists, create if needed (when auto-submit is enabled)
        if let Some(keypair_path) = &request.user_keypair_path {
            use actix_web::web;
            use spl_associated_token_account::get_associated_token_address;
            use solana_client::rpc_client::RpcClient;
            use std::str::FromStr;
            use solana_sdk::pubkey::Pubkey;
            
            let user = Pubkey::from_str(&request.user_pubkey)
                .map_err(|e| VaultError::TransactionError(format!("Invalid user pubkey: {}", e)))?;
            
            // Clone usdt_mint before moving into closure
            let usdt_mint = *self.solana.usdt_mint();
            let user_token_account = get_associated_token_address(&user, &usdt_mint);
            
            let rpc_url = self.config.solana_rpc_url.clone();
            let account_exists = web::block(move || {
                let rpc_client = RpcClient::new(rpc_url);
                rpc_client.get_account(&user_token_account).is_ok()
            }).await
            .map_err(|e| VaultError::TransactionError(format!("Failed to check token account: {}", e)))?;
            
            if !account_exists {
                info!("User token account doesn't exist, creating it...");
                // Create token account using user's keypair
                self.create_user_token_account(&request.user_pubkey, keypair_path).await
                    .map_err(|e| VaultError::TransactionError(format!("Failed to create token account: {}", e)))?;
            } else {
                // Check if user has enough USDT tokens
                let rpc_url_check = self.config.solana_rpc_url.clone();
                let user_token_account_check = user_token_account;
                let amount_check = request.amount;
                let user_balance = web::block(move || {
                    let rpc_client = RpcClient::new(rpc_url_check);
                    match rpc_client.get_account_data(&user_token_account_check) {
                        Ok(account_data) => {
                            // Deserialize token account to get balance
                            use spl_token::state::Account as TokenAccount;
                            use solana_sdk::program_pack::Pack;
                            match TokenAccount::unpack(&account_data) {
                                Ok(token_account) => Ok(token_account.amount),
                                Err(e) => Err(format!("Failed to parse token account: {}", e)),
                            }
                        }
                        Err(e) => Err(format!("Failed to get token account: {}", e)),
                    }
                }).await
                .map_err(|e| VaultError::TransactionError(format!("Failed to check token balance: {}", e)))?
                .map_err(|e| VaultError::TransactionError(format!("Failed to get token balance: {}", e)))?;
                
                if user_balance < amount_check {
                    return Err(VaultError::InvalidInput(format!(
                        "Insufficient USDT balance. You have {} USDT but trying to deposit {} USDT",
                        user_balance as f64 / 1_000_000.0,
                        amount_check as f64 / 1_000_000.0
                    )));
                }
                
                info!("✅ User has sufficient USDT balance: {} USDT", user_balance as f64 / 1_000_000.0);
            }
        }

        // Build deposit transaction
        let unsigned_tx = self
            .tx_builder
            .build_deposit(&request.user_pubkey, request.amount)
            .await
            .map_err(|e| VaultError::TransactionError(e.to_string()))?;

        // Create transaction record
        let tx_id = Uuid::new_v4();
        let tx_record = TransactionRecord {
            id: tx_id,
            vault_owner: request.user_pubkey.clone(),
            transaction_type: "deposit".to_string(),
            amount: request.amount as i64,
            signature: None,
            status: "pending".to_string(),
            balance_before: vault.total_balance,
            balance_after: vault.total_balance + request.amount as i64,
            counterparty: None,
            note: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            confirmed_at: None,
        };

        // Save pending transaction
        queries::create_transaction(self.db.pool(), &tx_record)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If user provided keypair path, sign and submit automatically
        if let Some(keypair_path) = &request.user_keypair_path {
            info!("Auto-signing and submitting transaction with user keypair...");
            // First, sign the transaction
            let (signed_tx, tx_signature) = match self.tx_submitter.sign_transaction(&unsigned_tx, keypair_path).await {
                Ok((signed, sig)) => (signed, sig),
                Err(e) => {
                    error!("Failed to sign transaction: {}", e);
                    // Return unsigned transaction if signing failed
                    return Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(unsigned_tx),
                        signature: None,
                        message: format!(
                            "Failed to sign transaction: {}. Please sign and submit manually.",
                            e
                        ),
                    });
                }
            };
            
            info!("✅ Transaction signed with signature: {}", tx_signature);
            
            // Try to submit the signed transaction
            match self.tx_submitter.submit_signed_transaction(&signed_tx).await {
                Ok(signature) => {
                    let signature_clone = signature.clone();
                    
                    // Update transaction record with signature
                    queries::update_transaction_status(
                        self.db.pool(),
                        tx_id,
                        "submitted",
                        Some(&signature),
                    )
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

                    // Immediately update vault balance in database
                    // This ensures the UI shows the new balance right away
                    // The BalanceTracker will reconcile with on-chain state later
                    let new_total_balance = vault.total_balance + request.amount as i64;
                    let new_available_balance = vault.available_balance + request.amount as i64;
                    
                    if let Err(e) = queries::update_vault_balance(
                        self.db.pool(),
                        &request.user_pubkey,
                        new_total_balance,
                        vault.locked_balance, // Locked balance unchanged for deposits
                        new_available_balance,
                    )
                    .await
                    {
                        warn!("Failed to update vault balance immediately: {}. BalanceTracker will sync later.", e);
                        // Don't fail the whole operation - balance will be synced by BalanceTracker
                    } else {
                        info!("✅ Vault balance updated: {} -> {} USDT", 
                            vault.total_balance as f64 / 1_000_000.0,
                            new_total_balance as f64 / 1_000_000.0);
                    }

                    info!("✅ Transaction submitted: {}", signature_clone);

                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "submitted".to_string(),
                        unsigned_transaction: None,
                        signature: Some(signature),
                        message: format!(
                            "Transaction submitted successfully. Signature: {}",
                            signature_clone
                        ),
                    })
                }
                Err(e) => {
                    error!("Failed to submit transaction: {}", e);
                    let sig_clone = tx_signature.clone();
                    // Return signed transaction with its signature even if submission failed
                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(signed_tx),
                        signature: Some(tx_signature),
                        message: format!(
                            "Transaction signed (signature: {}) but failed to submit: {}. You can submit the signed transaction manually using the signature or the transaction data.",
                            sig_clone, e
                        ),
                    })
                }
            }
        } else {
            // Return unsigned transaction for user to sign
        Ok(OperationResponse {
            transaction_id: tx_id,
            status: "pending".to_string(),
            unsigned_transaction: Some(unsigned_tx),
            signature: None,
            message: format!(
                "Sign and submit this transaction to deposit {} USDT",
                request.amount as f64 / 1_000_000.0
            ),
        })
        }
    }

    /// Process a withdrawal request.
    ///
    /// Validates the user has sufficient available balance,
    /// then builds a withdrawal transaction.
    ///
    /// ## Arguments
    ///
    /// * `request` - Withdrawal request with user pubkey and amount
    ///
    /// ## Returns
    ///
    /// * `Ok(OperationResponse)` - Contains unsigned transaction
    /// * `Err(VaultError::InsufficientBalance)` - Not enough available
    ///
    /// ## Security
    ///
    /// - Only available (unlocked) balance can be withdrawn
    /// - Locked collateral for positions is protected
    pub async fn withdraw(
        &self,
        request: WithdrawRequest,
    ) -> Result<OperationResponse, VaultError> {
        info!(
            "Processing withdrawal: {} USDT for {}",
            request.amount as f64 / 1_000_000.0,
            request.user_pubkey
        );

        // Validate amount
        if request.amount == 0 {
            return Err(VaultError::InvalidInput("Amount must be greater than 0".to_string()));
        }

        // Get current vault state - check database first
        let mut vault = queries::get_vault_by_owner(self.db.pool(), &request.user_pubkey)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If not in database, check on-chain and sync
        if vault.is_none() {
            info!("Vault not in database, checking on-chain...");
            if let Some(on_chain_vault) = self.fetch_vault_from_chain(&request.user_pubkey).await? {
                info!("Found vault on-chain, syncing to database...");
                queries::upsert_vault(self.db.pool(), &on_chain_vault)
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;
                vault = Some(on_chain_vault);
            }
        }

        // If still not found, vault doesn't exist
        let vault = vault.ok_or_else(|| VaultError::VaultNotFound(request.user_pubkey.clone()))?;

        // Check available balance
        if (vault.available_balance as u64) < request.amount {
            return Err(VaultError::InsufficientBalance {
                available: vault.available_balance as u64,
                requested: request.amount,
            });
        }

        // Build withdrawal transaction
        let unsigned_tx = self
            .tx_builder
            .build_withdraw(&request.user_pubkey, request.amount)
            .await
            .map_err(|e| VaultError::TransactionError(e.to_string()))?;

        // Create transaction record
        let tx_id = Uuid::new_v4();
        let tx_record = TransactionRecord {
            id: tx_id,
            vault_owner: request.user_pubkey.clone(),
            transaction_type: "withdrawal".to_string(),
            amount: request.amount as i64,
            signature: None,
            status: "pending".to_string(),
            balance_before: vault.total_balance,
            balance_after: vault.total_balance - request.amount as i64,
            counterparty: None,
            note: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            confirmed_at: None,
        };

        // Save pending transaction
        queries::create_transaction(self.db.pool(), &tx_record)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If user provided keypair path, sign and submit automatically
        if let Some(keypair_path) = &request.user_keypair_path {
            info!("Auto-signing and submitting transaction with user keypair...");
            // First, sign the transaction
            let (signed_tx, tx_signature) = match self.tx_submitter.sign_transaction(&unsigned_tx, keypair_path).await {
                Ok((signed, sig)) => (signed, sig),
                Err(e) => {
                    error!("Failed to sign transaction: {}", e);
                    // Return unsigned transaction if signing failed
                    return Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(unsigned_tx),
                        signature: None,
                        message: format!(
                            "Failed to sign transaction: {}. Please sign and submit manually.",
                            e
                        ),
                    });
                }
            };
            
            info!("✅ Transaction signed with signature: {}", tx_signature);
            
            // Try to submit the signed transaction
            match self.tx_submitter.submit_signed_transaction(&signed_tx).await {
                Ok(signature) => {
                    let signature_clone = signature.clone();
                    
                    // Update transaction record with signature
                    queries::update_transaction_status(
                        self.db.pool(),
                        tx_id,
                        "submitted",
                        Some(&signature),
                    )
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

                    // Immediately update vault balance in database
                    // This ensures the UI shows the new balance right away
                    // The BalanceTracker will reconcile with on-chain state later
                    let new_total_balance = vault.total_balance - request.amount as i64;
                    let new_available_balance = vault.available_balance - request.amount as i64;
                    
                    if let Err(e) = queries::update_vault_balance(
                        self.db.pool(),
                        &request.user_pubkey,
                        new_total_balance,
                        vault.locked_balance, // Locked balance unchanged for withdrawals
                        new_available_balance,
                    )
                    .await
                    {
                        warn!("Failed to update vault balance immediately: {}. BalanceTracker will sync later.", e);
                        // Don't fail the whole operation - balance will be synced by BalanceTracker
                    } else {
                        info!("✅ Vault balance updated: {} -> {} USDT", 
                            vault.total_balance as f64 / 1_000_000.0,
                            new_total_balance as f64 / 1_000_000.0);
                    }

                    info!("✅ Transaction submitted: {}", signature_clone);

                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "submitted".to_string(),
                        unsigned_transaction: None,
                        signature: Some(signature),
                        message: format!(
                            "Transaction submitted successfully. Signature: {}",
                            signature_clone
                        ),
                    })
                }
                Err(e) => {
                    error!("Failed to submit transaction: {}", e);
                    let sig_clone = tx_signature.clone();
                    // Return signed transaction with its signature even if submission failed
                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(signed_tx),
                        signature: Some(tx_signature),
                        message: format!(
                            "Transaction signed (signature: {}) but failed to submit: {}. You can submit the signed transaction manually using the signature or the transaction data.",
                            sig_clone, e
                        ),
                    })
                }
            }
        } else {
            // Return unsigned transaction for user to sign
        Ok(OperationResponse {
            transaction_id: tx_id,
            status: "pending".to_string(),
            unsigned_transaction: Some(unsigned_tx),
            signature: None,
            message: format!(
                "Sign and submit this transaction to withdraw {} USDT",
                request.amount as f64 / 1_000_000.0
            ),
        })
        }
    }

    /// Lock collateral for a trading position.
    ///
    /// This is called by the position manager (authorized program).
    /// It moves funds from available to locked balance.
    ///
    /// If `position_manager_keypair_path` is provided, the transaction will be
    /// automatically signed and submitted. Otherwise, an unsigned transaction
    /// is returned for manual signing.
    ///
    /// ## Arguments
    ///
    /// * `user_pubkey` - Vault owner
    /// * `amount` - Amount to lock (in smallest units, 6 decimals)
    /// * `position_id` - ID of the position requiring this margin
    /// * `position_manager_keypair_path` - Optional path to position manager keypair for auto-signing
    ///
    /// ## Returns
    ///
    /// * `Ok(OperationResponse)` - Lock transaction details (signed if keypair provided, unsigned otherwise)
    /// * `Err(VaultError::InsufficientBalance)` - Not enough available balance
    /// * `Err(VaultError::InvalidInput)` - Position manager keypair path is required
    pub async fn lock_collateral(
        &self,
        user_pubkey: &str,
        amount: u64,
        position_id: &str,
        position_manager_keypair_path: Option<&str>,
    ) -> Result<OperationResponse, VaultError> {
        info!(
            "Locking {} USDT for position {} (user: {})",
            amount as f64 / 1_000_000.0,
            position_id,
            user_pubkey
        );

        // Get current vault state - check database first
        let mut vault = queries::get_vault_by_owner(self.db.pool(), user_pubkey)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If not in database, check on-chain and sync
        if vault.is_none() {
            info!("Vault not in database, checking on-chain...");
            if let Some(on_chain_vault) = self.fetch_vault_from_chain(user_pubkey).await? {
                info!("Found vault on-chain, syncing to database...");
                queries::upsert_vault(self.db.pool(), &on_chain_vault)
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;
                vault = Some(on_chain_vault);
            }
        }

        // If still not found, vault doesn't exist
        let vault = vault.ok_or_else(|| VaultError::VaultNotFound(user_pubkey.to_string()))?;

        // Check available balance
        if (vault.available_balance as u64) < amount {
            return Err(VaultError::InsufficientBalance {
                available: vault.available_balance as u64,
                requested: amount,
            });
        }

        // Extract authority pubkey from keypair path if provided
        let authority_pubkey = if let Some(keypair_path) = position_manager_keypair_path {
            // Load keypair to get its pubkey
            use std::fs;
            use solana_sdk::signature::Keypair;
            use solana_sdk::signer::Signer;
            
            let keypair_path_expanded = shellexpand::full(keypair_path)
                .map_err(|e| VaultError::TransactionError(format!("Invalid keypair path: {}", e)))?;
            
            let keypair_bytes: Vec<u8> = serde_json::from_str(
                &fs::read_to_string(keypair_path_expanded.as_ref())
                    .map_err(|e| VaultError::TransactionError(format!("Failed to read keypair: {}", e)))?
            )
            .map_err(|e| VaultError::TransactionError(format!("Failed to parse keypair: {}", e)))?;
            
            let keypair = Keypair::from_bytes(&keypair_bytes)
                .map_err(|e| VaultError::TransactionError(format!("Failed to recreate keypair: {}", e)))?;
            
            keypair.pubkey().to_string()
        } else {
            return Err(VaultError::InvalidInput("Position manager keypair path is required for lock collateral".to_string()));
        };

        // Build lock collateral transaction with position manager as authority
        let unsigned_tx = self
            .tx_builder
            .build_lock_collateral(user_pubkey, amount, &authority_pubkey)
            .await
            .map_err(|e| VaultError::TransactionError(e.to_string()))?;

        // Create transaction record
        let tx_id = Uuid::new_v4();
        let tx_record = TransactionRecord {
            id: tx_id,
            vault_owner: user_pubkey.to_string(),
            transaction_type: "lock".to_string(),
            amount: amount as i64,
            signature: None,
            status: "pending".to_string(),
            balance_before: vault.available_balance,
            balance_after: vault.available_balance - amount as i64,
            counterparty: None,
            note: Some(format!("Position: {}", position_id)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            confirmed_at: None,
        };

        queries::create_transaction(self.db.pool(), &tx_record)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If a keypair path is provided, auto-sign and submit (like deposit/withdraw)
        if let Some(keypair_path) = position_manager_keypair_path {
            info!("Auto-signing lock collateral with position manager keypair...");

            // Sign transaction
            let (signed_tx, tx_signature) = match self
                .tx_submitter
                .sign_transaction(&unsigned_tx, keypair_path)
                .await
            {
                Ok((signed, sig)) => (signed, sig),
                Err(e) => {
                    error!("Failed to sign lock transaction: {}", e);
                    return Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(unsigned_tx),
                        signature: None,
                        message: format!(
                            "Failed to sign transaction: {}. Please sign and submit manually.",
                            e
                        ),
                    });
                }
            };

            info!("✅ Lock transaction signed with signature: {}", tx_signature);

            // Submit signed transaction
            match self.tx_submitter.submit_signed_transaction(&signed_tx).await {
                Ok(signature) => {
                    let signature_clone = signature.clone();

                    // Update transaction record
                    queries::update_transaction_status(
                        self.db.pool(),
                        tx_id,
                        "submitted",
                        Some(&signature),
                    )
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

                    // Immediately update vault balance in database
                    // Lock moves funds from available to locked
                    let new_locked_balance = vault.locked_balance + amount as i64;
                    let new_available_balance = vault.available_balance - amount as i64;
                    
                    match queries::update_vault_balance(
                        self.db.pool(),
                        user_pubkey,
                        vault.total_balance, // Total balance unchanged for locks
                        new_locked_balance,
                        new_available_balance,
                    )
                    .await
                    {
                        Ok(_) => {
                            info!("✅ Vault balance updated: Locked {} -> {} USDT, Available {} -> {} USDT", 
                                vault.locked_balance as f64 / 1_000_000.0,
                                new_locked_balance as f64 / 1_000_000.0,
                                vault.available_balance as f64 / 1_000_000.0,
                                new_available_balance as f64 / 1_000_000.0);
                            
                            // Verify the update by fetching the vault again
                            if let Ok(Some(updated_vault)) = queries::get_vault_by_owner(self.db.pool(), user_pubkey).await {
                                info!("✅ Verified balance update - Locked: {} USDT, Available: {} USDT", 
                                    updated_vault.locked_balance as f64 / 1_000_000.0,
                                    updated_vault.available_balance as f64 / 1_000_000.0);
                            } else {
                                warn!("⚠️ Could not verify balance update - vault not found after update");
                            }
                        }
                        Err(e) => {
                            error!("❌ Failed to update vault balance immediately: {}. BalanceTracker will sync later.", e);
                            // Still continue - the transaction was submitted successfully
                        }
                    }

                    info!("✅ Lock transaction submitted: {}", signature_clone);

                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "submitted".to_string(),
                        unsigned_transaction: None,
                        signature: Some(signature),
                        message: format!(
                            "Transaction submitted successfully. Signature: {}",
                            signature_clone
                        ),
                    })
                }
                Err(e) => {
                    error!("Failed to submit lock transaction: {}", e);
                    let sig_clone = tx_signature.clone();
                    // Return signed transaction for manual submit
                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(signed_tx),
                        signature: Some(tx_signature),
                        message: format!(
                            "Transaction signed (signature: {}) but failed to submit: {}. You can submit the signed transaction manually.",
                            sig_clone, e
                        ),
                    })
                }
            }
        } else {
            // No keypair path provided: return unsigned transaction for manual signing
        Ok(OperationResponse {
            transaction_id: tx_id,
            status: "pending".to_string(),
            unsigned_transaction: Some(unsigned_tx),
            signature: None,
                message: format!(
                    "Sign and submit this transaction to lock {} USDT for position {}",
                    amount as f64 / 1_000_000.0,
                    position_id
                ),
        })
        }
    }

    /// Unlock collateral after position close.
    ///
    /// Called by the position manager when a trading position is closed.
    /// Moves funds from locked back to available balance.
    ///
    /// If `position_manager_keypair_path` is provided, the transaction will be
    /// automatically signed and submitted. Otherwise, an unsigned transaction
    /// is returned for manual signing.
    ///
    /// ## Arguments
    ///
    /// * `user_pubkey` - Vault owner
    /// * `amount` - Amount to unlock (in smallest units, 6 decimals)
    /// * `position_id` - ID of the closed position
    /// * `position_manager_keypair_path` - Optional path to position manager keypair for auto-signing
    ///
    /// ## Returns
    ///
    /// * `Ok(OperationResponse)` - Unlock transaction details (signed if keypair provided, unsigned otherwise)
    /// * `Err(VaultError::InvalidInput)` - Not enough locked balance or keypair path required
    pub async fn unlock_collateral(
        &self,
        user_pubkey: &str,
        amount: u64,
        position_id: &str,
        position_manager_keypair_path: Option<&str>,
    ) -> Result<OperationResponse, VaultError> {
        info!(
            "Unlocking {} USDT for position {} (user: {})",
            amount as f64 / 1_000_000.0,
            position_id,
            user_pubkey
        );

        // Get current vault state - check database first
        let mut vault = queries::get_vault_by_owner(self.db.pool(), user_pubkey)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If not in database, check on-chain and sync
        if vault.is_none() {
            info!("Vault not in database, checking on-chain...");
            if let Some(on_chain_vault) = self.fetch_vault_from_chain(user_pubkey).await? {
                info!("Found vault on-chain, syncing to database...");
                queries::upsert_vault(self.db.pool(), &on_chain_vault)
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;
                vault = Some(on_chain_vault);
            }
        }

        // If still not found, vault doesn't exist
        let vault = vault.ok_or_else(|| VaultError::VaultNotFound(user_pubkey.to_string()))?;

        // Check locked balance
        if (vault.locked_balance as u64) < amount {
            return Err(VaultError::InvalidInput(format!(
                "Cannot unlock {} USDT - only {} USDT is locked",
                amount as f64 / 1_000_000.0,
                vault.locked_balance as f64 / 1_000_000.0
            )));
        }

        // Extract authority pubkey from keypair path if provided
        let authority_pubkey = if let Some(keypair_path) = position_manager_keypair_path {
            // Load keypair to get its pubkey
            use std::fs;
            use solana_sdk::signature::Keypair;
            use solana_sdk::signer::Signer;
            
            let keypair_path_expanded = shellexpand::full(keypair_path)
                .map_err(|e| VaultError::TransactionError(format!("Invalid keypair path: {}", e)))?;
            
            let keypair_bytes: Vec<u8> = serde_json::from_str(
                &fs::read_to_string(keypair_path_expanded.as_ref())
                    .map_err(|e| VaultError::TransactionError(format!("Failed to read keypair: {}", e)))?
            )
            .map_err(|e| VaultError::TransactionError(format!("Failed to parse keypair: {}", e)))?;
            
            let keypair = Keypair::from_bytes(&keypair_bytes)
                .map_err(|e| VaultError::TransactionError(format!("Failed to recreate keypair: {}", e)))?;
            
            keypair.pubkey().to_string()
        } else {
            return Err(VaultError::InvalidInput("Position manager keypair path is required for unlock collateral".to_string()));
        };

        // Build unlock collateral transaction with position manager as authority
        let unsigned_tx = self
            .tx_builder
            .build_unlock_collateral(user_pubkey, amount, &authority_pubkey)
            .await
            .map_err(|e| VaultError::TransactionError(e.to_string()))?;

        // Create transaction record
        let tx_id = Uuid::new_v4();
        let tx_record = TransactionRecord {
            id: tx_id,
            vault_owner: user_pubkey.to_string(),
            transaction_type: "unlock".to_string(),
            amount: amount as i64,
            signature: None,
            status: "pending".to_string(),
            balance_before: vault.available_balance,
            balance_after: vault.available_balance + amount as i64,
            counterparty: None,
            note: Some(format!("Position: {}", position_id)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            confirmed_at: None,
        };

        queries::create_transaction(self.db.pool(), &tx_record)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If a keypair path is provided, auto-sign and submit
        if let Some(keypair_path) = position_manager_keypair_path {
            info!("Auto-signing unlock collateral with position manager keypair...");

            // Sign transaction
            let (signed_tx, tx_signature) = match self
                .tx_submitter
                .sign_transaction(&unsigned_tx, keypair_path)
                .await
            {
                Ok((signed, sig)) => (signed, sig),
                Err(e) => {
                    error!("Failed to sign unlock transaction: {}", e);
                    return Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(unsigned_tx),
                        signature: None,
                        message: format!(
                            "Failed to sign transaction: {}. Please sign and submit manually.",
                            e
                        ),
                    });
                }
            };

            info!("✅ Unlock transaction signed with signature: {}", tx_signature);

            // Submit signed transaction
            match self.tx_submitter.submit_signed_transaction(&signed_tx).await {
                Ok(signature) => {
                    let signature_clone = signature.clone();

                    // Update transaction record
                    queries::update_transaction_status(
                        self.db.pool(),
                        tx_id,
                        "submitted",
                        Some(&signature),
                    )
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

                    // Immediately update vault balance in database
                    // Unlock moves funds from locked to available
                    let new_locked_balance = vault.locked_balance - amount as i64;
                    let new_available_balance = vault.available_balance + amount as i64;
                    
                    if let Err(e) = queries::update_vault_balance(
                        self.db.pool(),
                        user_pubkey,
                        vault.total_balance, // Total balance unchanged for unlocks
                        new_locked_balance,
                        new_available_balance,
                    )
                    .await
                    {
                        warn!("Failed to update vault balance immediately: {}. BalanceTracker will sync later.", e);
                    } else {
                        info!("✅ Vault balance updated: Locked {} -> {} USDT, Available {} -> {} USDT", 
                            vault.locked_balance as f64 / 1_000_000.0,
                            new_locked_balance as f64 / 1_000_000.0,
                            vault.available_balance as f64 / 1_000_000.0,
                            new_available_balance as f64 / 1_000_000.0);
                    }

                    info!("✅ Unlock transaction submitted: {}", signature_clone);

                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "submitted".to_string(),
                        unsigned_transaction: None,
                        signature: Some(signature),
                        message: format!(
                            "Transaction submitted successfully. Signature: {}",
                            signature_clone
                        ),
                    })
                }
                Err(e) => {
                    error!("Failed to submit unlock transaction: {}", e);
                    let sig_clone = tx_signature.clone();
                    // Return signed transaction for manual submit
                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(signed_tx),
                        signature: Some(tx_signature),
                        message: format!(
                            "Transaction signed (signature: {}) but failed to submit: {}. You can submit the signed transaction manually.",
                            sig_clone, e
                        ),
                    })
                }
            }
        } else {
            // No keypair path provided: return unsigned transaction for manual signing
        Ok(OperationResponse {
            transaction_id: tx_id,
            status: "pending".to_string(),
            unsigned_transaction: Some(unsigned_tx),
            signature: None,
            message: format!(
                    "Sign and submit this transaction to unlock {} USDT from position {}",
                amount as f64 / 1_000_000.0,
                position_id
            ),
        })
        }
    }

    /// Transfer collateral between vaults.
    ///
    /// This is called by authorized programs (liquidation engine, settlement relayer).
    /// It transfers actual tokens from one vault to another.
    ///
    /// If `liquidation_engine_keypair_path` is provided, the transaction will be
    /// automatically signed and submitted. Otherwise, an unsigned transaction
    /// is returned for manual signing.
    ///
    /// ## Arguments
    ///
    /// * `from_pubkey` - Source vault owner
    /// * `to_pubkey` - Destination vault owner
    /// * `amount` - Amount to transfer (in smallest units, 6 decimals)
    /// * `reason` - Transfer reason ("settlement", "liquidation", or "fee")
    /// * `liquidation_engine_keypair_path` - Optional path to liquidation engine keypair for auto-signing
    ///
    /// ## Returns
    ///
    /// * `Ok(OperationResponse)` - Transfer transaction details (signed if keypair provided, unsigned otherwise)
    /// * `Err(VaultError::InsufficientBalance)` - Source vault doesn't have enough balance
    /// * `Err(VaultError::InvalidInput)` - Liquidation engine keypair path is required
    pub async fn transfer_collateral(
        &self,
        from_pubkey: &str,
        to_pubkey: &str,
        amount: u64,
        reason: &str,
        liquidation_engine_keypair_path: Option<&str>,
    ) -> Result<OperationResponse, VaultError> {
        info!(
            "Transferring {} USDT from {} to {} (reason: {})",
            amount as f64 / 1_000_000.0,
            from_pubkey,
            to_pubkey,
            reason
        );

        // Get source vault state
        let mut from_vault = queries::get_vault_by_owner(self.db.pool(), from_pubkey)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If not in database, check on-chain and sync
        if from_vault.is_none() {
            info!("Source vault not in database, checking on-chain...");
            if let Some(on_chain_vault) = self.fetch_vault_from_chain(from_pubkey).await? {
                info!("Found source vault on-chain, syncing to database...");
                queries::upsert_vault(self.db.pool(), &on_chain_vault)
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;
                from_vault = Some(on_chain_vault);
            }
        }

        let from_vault = from_vault.ok_or_else(|| VaultError::VaultNotFound(from_pubkey.to_string()))?;

        // Get destination vault state
        let mut to_vault = queries::get_vault_by_owner(self.db.pool(), to_pubkey)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If not in database, check on-chain and sync
        if to_vault.is_none() {
            info!("Destination vault not in database, checking on-chain...");
            if let Some(on_chain_vault) = self.fetch_vault_from_chain(to_pubkey).await? {
                info!("Found destination vault on-chain, syncing to database...");
                queries::upsert_vault(self.db.pool(), &on_chain_vault)
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;
                to_vault = Some(on_chain_vault);
            }
        }

        let to_vault = to_vault.ok_or_else(|| VaultError::VaultNotFound(to_pubkey.to_string()))?;

        // Check source vault has sufficient balance
        if (from_vault.total_balance as u64) < amount {
            return Err(VaultError::InsufficientBalance {
                available: from_vault.total_balance as u64,
                requested: amount,
            });
        }

        // Convert reason string to u8
        let reason_code = match reason.to_lowercase().as_str() {
            "settlement" => 0,
            "liquidation" => 1,
            "fee" => 2,
            _ => 0, // Default to settlement
        };

        // Extract authority pubkey from keypair path if provided
        let authority_pubkey = if let Some(keypair_path) = liquidation_engine_keypair_path {
            // Load keypair to get its pubkey
            use std::fs;
            use solana_sdk::signature::Keypair;
            use solana_sdk::signer::Signer;
            
            let keypair_path_expanded = shellexpand::full(keypair_path)
                .map_err(|e| VaultError::TransactionError(format!("Invalid keypair path: {}", e)))?;
            
            let keypair_bytes: Vec<u8> = serde_json::from_str(
                &fs::read_to_string(keypair_path_expanded.as_ref())
                    .map_err(|e| VaultError::TransactionError(format!("Failed to read keypair: {}", e)))?
            )
            .map_err(|e| VaultError::TransactionError(format!("Failed to parse keypair: {}", e)))?;
            
            let keypair = Keypair::from_bytes(&keypair_bytes)
                .map_err(|e| VaultError::TransactionError(format!("Failed to recreate keypair: {}", e)))?;
            
            keypair.pubkey().to_string()
        } else {
            return Err(VaultError::InvalidInput("Liquidation engine keypair path is required for transfer collateral".to_string()));
        };

        // Build transfer transaction with liquidation engine as authority
        let unsigned_tx = self
            .tx_builder
            .build_transfer_collateral(from_pubkey, to_pubkey, amount, reason_code, &authority_pubkey)
            .await
            .map_err(|e| VaultError::TransactionError(e.to_string()))?;

        // Create transaction records for both vaults
        let tx_id = Uuid::new_v4();
        
        // Source vault transaction (outgoing)
        let from_tx = TransactionRecord {
            id: Uuid::new_v4(),
            vault_owner: from_pubkey.to_string(),
            transaction_type: "transfer_out".to_string(),
            amount: -(amount as i64), // Negative for outgoing
            signature: None,
            status: "pending".to_string(),
            balance_before: from_vault.total_balance,
            balance_after: from_vault.total_balance - amount as i64,
            counterparty: Some(to_pubkey.to_string()),
            note: Some(format!("Transfer: {}", reason)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            confirmed_at: None,
        };

        // Destination vault transaction (incoming)
        let to_tx = TransactionRecord {
            id: Uuid::new_v4(),
            vault_owner: to_pubkey.to_string(),
            transaction_type: "transfer_in".to_string(),
            amount: amount as i64,
            signature: None,
            status: "pending".to_string(),
            balance_before: to_vault.total_balance,
            balance_after: to_vault.total_balance + amount as i64,
            counterparty: Some(from_pubkey.to_string()),
            note: Some(format!("Transfer: {}", reason)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            confirmed_at: None,
        };

        queries::create_transaction(self.db.pool(), &from_tx)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        queries::create_transaction(self.db.pool(), &to_tx)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // If a keypair path is provided, auto-sign and submit
        if let Some(keypair_path) = liquidation_engine_keypair_path {
            info!("Auto-signing transfer collateral with liquidation engine keypair...");

            // Sign transaction
            let (signed_tx, tx_signature) = match self
                .tx_submitter
                .sign_transaction(&unsigned_tx, keypair_path)
                .await
            {
                Ok((signed, sig)) => (signed, sig),
                Err(e) => {
                    error!("Failed to sign transfer transaction: {}", e);
                    return Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(unsigned_tx),
                        signature: None,
                        message: format!(
                            "Failed to sign transaction: {}. Please sign and submit manually.",
                            e
                        ),
                    });
                }
            };

            info!("✅ Transfer transaction signed with signature: {}", tx_signature);

            // Submit signed transaction
            match self.tx_submitter.submit_signed_transaction(&signed_tx).await {
                Ok(signature) => {
                    let signature_clone = signature.clone();

                    // Update transaction records with signature
                    queries::update_transaction_status(
                        self.db.pool(),
                        tx_id,
                        "submitted",
                        Some(&signature),
                    )
                    .await
                    .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

                    // Also update the individual transfer_in/out records (best effort)
                    let _ = queries::update_transaction_status(
                        self.db.pool(),
                        from_tx.id,
                        "submitted",
                        Some(&signature),
                    ).await;
                    let _ = queries::update_transaction_status(
                        self.db.pool(),
                        to_tx.id,
                        "submitted",
                        Some(&signature),
                    ).await;

                    // Immediately update vault balances in database
                    // Transfer moves funds from source to destination
                    let new_from_total = from_vault.total_balance - amount as i64;
                    let new_to_total = to_vault.total_balance + amount as i64;
                    
                    // Update source vault (decrease total, prioritize locked if available)
                    let (new_from_locked, new_from_available) = if from_vault.locked_balance >= amount as i64 {
                        (from_vault.locked_balance - amount as i64, from_vault.available_balance)
                    } else {
                        let remaining = amount as i64 - from_vault.locked_balance;
                        (0, from_vault.available_balance - remaining)
                    };
                    
                    // Update destination vault (increase total and available)
                    let new_to_locked = to_vault.locked_balance;
                    let new_to_available = to_vault.available_balance + amount as i64;
                    
                    // Update source vault
                    if let Err(e) = queries::update_vault_balance(
                        self.db.pool(),
                        from_pubkey,
                        new_from_total,
                        new_from_locked,
                        new_from_available,
                    )
                    .await
                    {
                        warn!("Failed to update source vault balance: {}. BalanceTracker will sync later.", e);
                    }
                    
                    // Update destination vault
                    if let Err(e) = queries::update_vault_balance(
                        self.db.pool(),
                        to_pubkey,
                        new_to_total,
                        new_to_locked,
                        new_to_available,
                    )
                    .await
                    {
                        warn!("Failed to update destination vault balance: {}. BalanceTracker will sync later.", e);
                    } else {
                        info!("✅ Transfer balances updated: From {} -> {} USDT, To {} -> {} USDT", 
                            from_vault.total_balance as f64 / 1_000_000.0,
                            new_from_total as f64 / 1_000_000.0,
                            to_vault.total_balance as f64 / 1_000_000.0,
                            new_to_total as f64 / 1_000_000.0);
                    }

                    info!("✅ Transfer transaction submitted: {}", signature_clone);

                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "submitted".to_string(),
                        unsigned_transaction: None,
                        signature: Some(signature),
                        message: format!(
                            "Transaction submitted successfully. Signature: {}",
                            signature_clone
                        ),
                    })
                }
                Err(e) => {
                    error!("Failed to submit transfer transaction: {}", e);
                    let sig_clone = tx_signature.clone();
                    // Return signed transaction for manual submit
                    Ok(OperationResponse {
                        transaction_id: tx_id,
                        status: "pending".to_string(),
                        unsigned_transaction: Some(signed_tx),
                        signature: Some(tx_signature),
                        message: format!(
                            "Transaction signed (signature: {}) but failed to submit: {}. You can submit the signed transaction manually.",
                            sig_clone, e
                        ),
                    })
                }
            }
        } else {
            // No keypair path provided: return unsigned transaction for manual signing
            Ok(OperationResponse {
                transaction_id: tx_id,
                status: "pending".to_string(),
                unsigned_transaction: Some(unsigned_tx),
                signature: None,
                message: format!(
                    "Sign and submit this transaction to transfer {} USDT from {} to {} (reason: {})",
                    amount as f64 / 1_000_000.0,
                    from_pubkey,
                    to_pubkey,
                    reason
                ),
            })
        }
    }

    // ==========================================
    // TRANSACTION HISTORY
    // ==========================================

    /// Get transaction history for a vault.
    ///
    /// ## Arguments
    ///
    /// * `owner` - Vault owner's public key
    /// * `limit` - Max transactions to return
    /// * `offset` - Number to skip (pagination)
    pub async fn get_transactions(
        &self,
        owner: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TransactionRecord>, VaultError> {
        queries::get_vault_transactions(self.db.pool(), owner, limit, offset)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))
    }

    /// Create a user's associated token account if it doesn't exist.
    ///
    /// This is a helper function that creates the token account
    /// before deposit/withdraw operations.
    async fn create_user_token_account(
        &self,
        user_pubkey: &str,
        keypair_path: &str,
    ) -> Result<(), VaultError> {
        use actix_web::web;
        use spl_associated_token_account::{get_associated_token_address, instruction::create_associated_token_account};
        use solana_client::rpc_client::RpcClient;
        use solana_sdk::{
            pubkey::Pubkey,
            signature::{Keypair, Signer},
            transaction::Transaction,
        };
        use std::str::FromStr;
        use std::fs;

        let user = Pubkey::from_str(user_pubkey)
            .map_err(|e| VaultError::TransactionError(format!("Invalid user pubkey: {}", e)))?;
        
        // Clone usdt_mint before moving into closure
        let usdt_mint = *self.solana.usdt_mint();
        let user_token_account = get_associated_token_address(&user, &usdt_mint);

        // Load user's keypair
        let keypair_path_expanded = shellexpand::full(keypair_path)
            .map_err(|e| VaultError::TransactionError(format!("Invalid keypair path: {}", e)))?;
        
        let keypair_bytes: Vec<u8> = serde_json::from_str(
            &fs::read_to_string(keypair_path_expanded.as_ref())
                .map_err(|e| VaultError::TransactionError(format!("Failed to read keypair: {}", e)))?
        )
        .map_err(|e| VaultError::TransactionError(format!("Failed to parse keypair: {}", e)))?;

        let rpc_url = self.config.solana_rpc_url.clone();
        let keypair_bytes_clone = keypair_bytes.clone();

        web::block(move || {
            let rpc_client = RpcClient::new(rpc_url);
            let keypair = Keypair::from_bytes(&keypair_bytes_clone)
                .map_err(|e| VaultError::TransactionError(format!("Invalid keypair: {}", e)))?;

            // Build create ATA instruction
            let create_ata_instruction = create_associated_token_account(
                &keypair.pubkey(),  // payer (user pays rent)
                &user,             // owner
                &usdt_mint,        // mint
                &spl_token::id(),  // token program
            );

            // Get recent blockhash
            let recent_blockhash = rpc_client.get_latest_blockhash()
                .map_err(|e| VaultError::TransactionError(format!("Failed to get blockhash: {}", e)))?;

            // Build transaction
            let mut transaction = Transaction::new_with_payer(
                &[create_ata_instruction],
                Some(&keypair.pubkey()),
            );
            transaction.sign(&[&keypair], recent_blockhash);

            // Submit transaction
            rpc_client.send_and_confirm_transaction(&transaction)
                .map_err(|e| VaultError::TransactionError(format!("Failed to create token account: {}", e)))?;

            Ok::<(), VaultError>(())
        }).await
        .map_err(|e| VaultError::TransactionError(format!("Blocking task failed: {}", e)))?
        .map_err(|e| VaultError::TransactionError(e.to_string()))?;

        info!("✓ Created user token account: {}", user_token_account);
        Ok(())
    }

    /// Confirm a transaction after it's been included on-chain.
    ///
    /// Called when we detect the transaction has been confirmed.
    /// Updates the transaction status and vault balances.
    ///
    /// ## Arguments
    ///
    /// * `tx_id` - Internal transaction ID
    /// * `signature` - Solana transaction signature
    pub async fn confirm_transaction(
        &self,
        tx_id: Uuid,
        signature: &str,
    ) -> Result<(), VaultError> {
        info!("Confirming transaction {} with signature {}", tx_id, signature);

        queries::update_transaction_status(self.db.pool(), tx_id, "confirmed", Some(signature))
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        // Note: Balance updates should be done by the BalanceTracker
        // when it syncs with the blockchain

        Ok(())
    }
}

