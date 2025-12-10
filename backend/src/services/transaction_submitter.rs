//! # Transaction Submitter Service
//!
//! This service signs and submits Solana transactions automatically.
//! For user operations (deposit/withdraw), requires the user's keypair.

use std::fs;
use solana_sdk::{
    signature::Keypair,
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::{info, error};

use crate::config::AppConfig;

/// Errors that can occur when submitting transactions.
#[derive(Debug, thiserror::Error)]
pub enum TransactionSubmitterError {
    /// Failed to load keypair.
    #[error("Failed to load keypair: {0}")]
    KeypairError(String),

    /// Failed to decode transaction.
    #[error("Failed to decode transaction: {0}")]
    DecodeError(String),

    /// Failed to get recent blockhash.
    #[error("Failed to get blockhash: {0}")]
    BlockhashError(String),

    /// RPC error.
    #[error("RPC error: {0}")]
    RpcError(String),
}

/// Transaction Submitter service.
///
/// Signs and submits Solana transactions automatically.
#[derive(Clone)]
pub struct TransactionSubmitter {
    /// Application configuration.
    config: AppConfig,
}

impl TransactionSubmitter {
    /// Create a new TransactionSubmitter.
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Sign and submit a transaction using a keypair.
    ///
    /// ## Arguments
    ///
    /// * `unsigned_tx_base64` - Base64-encoded unsigned transaction
    /// * `keypair_path` - Path to keypair file to sign with
    ///
    /// ## Returns
    ///
    /// Transaction signature if successful.

    pub async fn sign_and_submit(
        &self,
        unsigned_tx_base64: &str,
        keypair_path: &str,
    ) -> Result<String, TransactionSubmitterError> {
        use actix_web::web;

        info!("Signing and submitting transaction...");

        // Load keypair
        let keypair_path_expanded = shellexpand::full(keypair_path)
            .map_err(|e| TransactionSubmitterError::KeypairError(format!("Invalid path: {}", e)))?;
        
        let keypair_bytes: Vec<u8> = serde_json::from_str(
            &fs::read_to_string(keypair_path_expanded.as_ref())
                .map_err(|e| TransactionSubmitterError::KeypairError(format!("Failed to read keypair: {}", e)))?
        )
        .map_err(|e| TransactionSubmitterError::KeypairError(format!("Failed to parse keypair: {}", e)))?;

        // Decode transaction
        let tx_bytes = BASE64.decode(unsigned_tx_base64)
            .map_err(|e| TransactionSubmitterError::DecodeError(e.to_string()))?;
        
        let mut transaction: Transaction = bincode::deserialize(&tx_bytes)
            .map_err(|e| TransactionSubmitterError::DecodeError(format!("Failed to deserialize: {}", e)))?;

        // Submit transaction
        let rpc_url = self.config.solana_rpc_url.clone();
        let keypair_bytes_clone = keypair_bytes.clone();
        let tx_bytes = bincode::serialize(&transaction)
            .map_err(|e| TransactionSubmitterError::DecodeError(format!("Failed to serialize: {}", e)))?;

        let signature = tokio::task::spawn_blocking(move || {
            let rpc_client = RpcClient::new(rpc_url);
            let keypair = Keypair::from_bytes(&keypair_bytes_clone)
                .map_err(|e| TransactionSubmitterError::KeypairError(format!("Failed to recreate keypair: {}", e)))?;

            // Get recent blockhash (transaction might have expired, so get fresh one)
            let recent_blockhash = rpc_client.get_latest_blockhash()
                .map_err(|e| TransactionSubmitterError::BlockhashError(e.to_string()))?;

            // Deserialize and update transaction with fresh blockhash and sign
            let mut transaction: Transaction = bincode::deserialize(&tx_bytes)
                .map_err(|e| TransactionSubmitterError::DecodeError(format!("Failed to deserialize: {}", e)))?;
            
            transaction.sign(&[&keypair], recent_blockhash);

            // Submit transaction
            let signature = rpc_client.send_and_confirm_transaction(&transaction)
                .map_err(|e| TransactionSubmitterError::RpcError(format!("Failed to submit: {}", e)))?;

            Ok::<String, TransactionSubmitterError>(signature.to_string())
        }).await
        .map_err(|e| TransactionSubmitterError::RpcError(format!("Blocking task failed: {}", e)))?
        .map_err(|e| TransactionSubmitterError::RpcError(e.to_string()))?;

        info!("✅ Transaction submitted: {}", signature);
        Ok(signature)
    }

    /// Sign a transaction without submitting it.
    ///
    /// ## Arguments
    ///
    /// * `unsigned_tx_base64` - Base64-encoded unsigned transaction
    /// * `keypair_path` - Path to keypair file to sign with
    ///
    /// ## Returns
    ///
    /// Tuple of (signed_transaction_base64, transaction_signature)
    pub async fn sign_transaction(
        &self,
        unsigned_tx_base64: &str,
        keypair_path: &str,
    ) -> Result<(String, String), TransactionSubmitterError> {
        info!("Signing transaction with keypair: {}", keypair_path);

        // Load keypair
        let keypair_path_expanded = shellexpand::full(keypair_path)
            .map_err(|e| TransactionSubmitterError::KeypairError(format!("Invalid path: {}", e)))?;
        
        let keypair_bytes: Vec<u8> = serde_json::from_str(
            &fs::read_to_string(keypair_path_expanded.as_ref())
                .map_err(|e| TransactionSubmitterError::KeypairError(format!("Failed to read keypair: {}", e)))?
        )
        .map_err(|e| TransactionSubmitterError::KeypairError(format!("Failed to parse keypair: {}", e)))?;

        // Decode transaction
        let tx_bytes = BASE64.decode(unsigned_tx_base64)
            .map_err(|e| TransactionSubmitterError::DecodeError(e.to_string()))?;

        // Sign transaction
        let rpc_url = self.config.solana_rpc_url.clone();
        let keypair_bytes_clone = keypair_bytes.clone();

        let signed_tx_base64 = tokio::task::spawn_blocking(move || {
            let rpc_client = RpcClient::new(rpc_url);
            let keypair = Keypair::from_bytes(&keypair_bytes_clone)
                .map_err(|e| TransactionSubmitterError::KeypairError(format!("Failed to recreate keypair: {}", e)))?;

            // Get recent blockhash (transaction might have expired, so get fresh one)
            let recent_blockhash = rpc_client.get_latest_blockhash()
                .map_err(|e| TransactionSubmitterError::BlockhashError(e.to_string()))?;

            // Deserialize transaction
            let mut transaction: Transaction = bincode::deserialize(&tx_bytes)
                .map_err(|e| TransactionSubmitterError::DecodeError(format!("Failed to deserialize: {}", e)))?;

            // Update transaction with fresh blockhash and sign
            transaction.sign(&[&keypair], recent_blockhash);

            // Get the signature from the signed transaction
            let signature = transaction.signatures[0].to_string();

            // Serialize signed transaction
            let signed_tx_bytes = bincode::serialize(&transaction)
                .map_err(|e| TransactionSubmitterError::DecodeError(format!("Failed to serialize signed tx: {}", e)))?;

            Ok::<(String, String), TransactionSubmitterError>((BASE64.encode(&signed_tx_bytes), signature))
        }).await
        .map_err(|e| TransactionSubmitterError::RpcError(format!("Blocking task failed: {}", e)))?
        .map_err(|e| TransactionSubmitterError::RpcError(e.to_string()))?;

        info!("✅ Transaction signed with signature: {}", signed_tx_base64.1);
        Ok(signed_tx_base64)
    }

    /// Submit an already-signed transaction.
    ///
    /// Uses `send_and_confirm_transaction` which waits for confirmation.
    /// Runs in a blocking task (`tokio::task::spawn_blocking`) to avoid blocking the async runtime.
    ///
    /// ## Arguments
    ///
    /// * `signed_tx_base64` - Base64-encoded signed transaction
    ///
    /// ## Returns
    ///
    /// Transaction signature (as string) if successful.
    pub async fn submit_signed_transaction(
        &self,
        signed_tx_base64: &str,
    ) -> Result<String, TransactionSubmitterError> {
        use actix_web::web;

        info!("Submitting signed transaction...");

        // Decode transaction
        let tx_bytes = BASE64.decode(signed_tx_base64)
            .map_err(|e| TransactionSubmitterError::DecodeError(e.to_string()))?;
        
        let transaction: Transaction = bincode::deserialize(&tx_bytes)
            .map_err(|e| TransactionSubmitterError::DecodeError(format!("Failed to deserialize: {}", e)))?;

        // Submit transaction
        let rpc_url = self.config.solana_rpc_url.clone();
        let tx_bytes = bincode::serialize(&transaction)
            .map_err(|e| TransactionSubmitterError::DecodeError(format!("Failed to serialize: {}", e)))?;

        let signature = tokio::task::spawn_blocking(move || {
            let rpc_client = RpcClient::new(rpc_url);
            let transaction: Transaction = bincode::deserialize(&tx_bytes)
                .map_err(|e| TransactionSubmitterError::DecodeError(format!("Failed to deserialize: {}", e)))?;

            // Submit transaction
            let signature = rpc_client.send_and_confirm_transaction(&transaction)
                .map_err(|e| TransactionSubmitterError::RpcError(format!("Failed to submit: {}", e)))?;

            Ok::<String, TransactionSubmitterError>(signature.to_string())
        }).await
        .map_err(|e| TransactionSubmitterError::RpcError(format!("Blocking task failed: {}", e)))?
        .map_err(|e| TransactionSubmitterError::RpcError(e.to_string()))?;

        info!("✅ Transaction submitted: {}", signature);
        Ok(signature)
    }

}
