//! # Database Queries
//!
//! This module contains all the SQL queries for interacting with the database.
//! Each function performs a specific database operation.
//!
//! ## Query Organization
//!
//! Queries are grouped by the table they operate on:
//! - `vault_*` - Vault table operations
//! - `transaction_*` - Transaction table operations
//! - `snapshot_*` - Balance snapshot operations
//! - `reconciliation_*` - Reconciliation log operations
//!
//! ## Error Handling
//!
//! All queries return `Result<T, DatabaseError>`. Common errors:
//! - `NotFound` - Record doesn't exist
//! - `QueryError` - SQL execution failed

use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use tokio_postgres::Row;
use uuid::Uuid;
use tracing::{debug, error, info};

use super::models::*;
use super::DatabaseError;

// ============================================
// HELPER FUNCTIONS
// ============================================

/// Helper to convert a database row to VaultRecord
fn row_to_vault(row: &Row) -> Result<VaultRecord, DatabaseError> {
    Ok(VaultRecord {
        owner: row.get("owner"),
        vault_address: row.get("vault_address"),
        token_account: row.get("token_account"),
        total_balance: row.get("total_balance"),
        locked_balance: row.get("locked_balance"),
        available_balance: row.get("available_balance"),
        total_deposited: row.get("total_deposited"),
        total_withdrawn: row.get("total_withdrawn"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        status: row.get("status"),
    })
}

/// Helper to convert a database row to TransactionRecord
fn row_to_transaction(row: &Row) -> Result<TransactionRecord, DatabaseError> {
    Ok(TransactionRecord {
        id: row.get("id"),
        vault_owner: row.get("vault_owner"),
        transaction_type: row.get("transaction_type"),
        amount: row.get("amount"),
        signature: row.get("signature"),
        status: row.get("status"),
        balance_before: row.get("balance_before"),
        balance_after: row.get("balance_after"),
        counterparty: row.get("counterparty"),
        note: row.get("note"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        confirmed_at: row.get("confirmed_at"),
    })
}

// ============================================
// VAULT QUERIES
// ============================================

/// Get a vault by owner's public key.
pub async fn get_vault_by_owner(
    pool: &Pool,
    owner: &str,
) -> Result<Option<VaultRecord>, DatabaseError> {
    debug!("Fetching vault for owner: {}", owner);

    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    let rows = client.query(
        r#"
        SELECT 
            owner, vault_address, token_account,
            total_balance, locked_balance, available_balance,
            total_deposited, total_withdrawn,
            created_at, updated_at, status
        FROM vaults
        WHERE owner = $1
        "#,
        &[&owner],
    ).await?;

    if rows.is_empty() {
        Ok(None)
    } else {
        Ok(Some(row_to_vault(&rows[0])?))
    }
}

/// Get all active vaults.
pub async fn get_all_active_vaults(
    pool: &Pool,
    limit: i64,
    offset: i64,
) -> Result<Vec<VaultRecord>, DatabaseError> {
    debug!("Fetching active vaults (limit: {}, offset: {})", limit, offset);

    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    let rows = client.query(
        r#"
        SELECT 
            owner, vault_address, token_account,
            total_balance, locked_balance, available_balance,
            total_deposited, total_withdrawn,
            created_at, updated_at, status
        FROM vaults
        WHERE status = 'active'
        ORDER BY updated_at DESC
        LIMIT $1 OFFSET $2
        "#,
        &[&limit, &offset],
    ).await?;

    let mut vaults = Vec::new();
    for row in rows {
        vaults.push(row_to_vault(&row)?);
    }

    Ok(vaults)
}

/// Create or update a vault record.
pub async fn upsert_vault(
    pool: &Pool,
    vault: &VaultRecord,
) -> Result<(), DatabaseError> {
    debug!("Upserting vault for owner: {}", vault.owner);

    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    client.execute(
        r#"
        INSERT INTO vaults (
            owner, vault_address, token_account,
            total_balance, locked_balance, available_balance,
            total_deposited, total_withdrawn,
            created_at, updated_at, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (owner) DO UPDATE SET
            vault_address = EXCLUDED.vault_address,
            token_account = EXCLUDED.token_account,
            total_balance = EXCLUDED.total_balance,
            locked_balance = EXCLUDED.locked_balance,
            available_balance = EXCLUDED.available_balance,
            total_deposited = EXCLUDED.total_deposited,
            total_withdrawn = EXCLUDED.total_withdrawn,
            updated_at = EXCLUDED.updated_at,
            status = EXCLUDED.status
        "#,
        &[
            &vault.owner,
            &vault.vault_address,
            &vault.token_account,
            &vault.total_balance,
            &vault.locked_balance,
            &vault.available_balance,
            &vault.total_deposited,
            &vault.total_withdrawn,
            &vault.created_at,
            &vault.updated_at,
            &vault.status,
        ],
    ).await?;

    info!("Vault upserted for owner: {}", vault.owner);
    Ok(())
}

/// Update vault balance fields only.
pub async fn update_vault_balance(
    pool: &Pool,
    owner: &str,
    total_balance: i64,
    locked_balance: i64,
    available_balance: i64,
) -> Result<(), DatabaseError> {
    debug!("Updating balance for vault: {}", owner);

    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    let rows_affected = client.execute(
        r#"
        UPDATE vaults
        SET 
            total_balance = $2,
            locked_balance = $3,
            available_balance = $4,
            updated_at = NOW()
        WHERE owner = $1
        "#,
        &[&owner, &total_balance, &locked_balance, &available_balance],
    ).await?;

    if rows_affected == 0 {
        return Err(DatabaseError::NotFound(format!("Vault not found: {}", owner)));
    }

    info!("✅ Balance updated for vault {}: Total={}, Locked={}, Available={}", 
        owner, 
        total_balance as f64 / 1_000_000.0,
        locked_balance as f64 / 1_000_000.0,
        available_balance as f64 / 1_000_000.0);
    Ok(())
}

// ============================================
// TRANSACTION QUERIES
// ============================================

/// Record a new transaction.
pub async fn create_transaction(
    pool: &Pool,
    tx: &TransactionRecord,
) -> Result<Uuid, DatabaseError> {
    debug!(
        "Creating transaction: {:?} for vault: {}",
        tx.transaction_type, tx.vault_owner
    );

    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    client.execute(
        r#"
        INSERT INTO transactions (
            id, vault_owner, transaction_type, amount,
            signature, status, balance_before, balance_after,
            counterparty, note, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
        &[
            &tx.id,
            &tx.vault_owner,
            &tx.transaction_type,
            &tx.amount,
            &tx.signature,
            &tx.status,
            &tx.balance_before,
            &tx.balance_after,
            &tx.counterparty,
            &tx.note,
            &tx.created_at,
            &tx.updated_at,
        ],
    ).await?;

    info!("Transaction created: {}", tx.id);
    Ok(tx.id)
}

/// Update transaction status and signature.
pub async fn update_transaction_status(
    pool: &Pool,
    id: Uuid,
    status: &str,
    signature: Option<&str>,
) -> Result<(), DatabaseError> {
    debug!("Updating transaction {} status to: {}", id, status);

    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    let confirmed_at = if status == "confirmed" {
        Some(Utc::now())
    } else {
        None
    };

    client.execute(
        r#"
        UPDATE transactions
        SET 
            status = $2,
            signature = COALESCE($3, signature),
            confirmed_at = $4,
            updated_at = NOW()
        WHERE id = $1
        "#,
        &[&id, &status, &signature, &confirmed_at],
    ).await?;

    Ok(())
}

/// Get transactions for a vault.
pub async fn get_vault_transactions(
    pool: &Pool,
    owner: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<TransactionRecord>, DatabaseError> {
    debug!("Fetching transactions for vault: {}", owner);

    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    let rows = client.query(
        r#"
        SELECT 
            id, vault_owner, transaction_type, amount,
            signature, status, balance_before, balance_after,
            counterparty, note, created_at, updated_at, confirmed_at
        FROM transactions
        WHERE vault_owner = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        &[&owner, &limit, &offset],
    ).await?;

    let mut transactions = Vec::new();
    for row in rows {
        transactions.push(row_to_transaction(&row)?);
    }

    Ok(transactions)
}

/// Get recent transactions across all vaults.
#[allow(dead_code)]
pub async fn get_recent_transactions(
    pool: &Pool,
    limit: i64,
) -> Result<Vec<TransactionRecord>, DatabaseError> {
    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    let rows = client.query(
        r#"
        SELECT 
            id, vault_owner, transaction_type, amount,
            signature, status, balance_before, balance_after,
            counterparty, note, created_at, updated_at, confirmed_at
        FROM transactions
        ORDER BY created_at DESC
        LIMIT $1
        "#,
        &[&limit],
    ).await?;

    let mut transactions = Vec::new();
    for row in rows {
        transactions.push(row_to_transaction(&row)?);
    }

    Ok(transactions)
}

// ============================================
// SNAPSHOT QUERIES
// ============================================

/// Create a balance snapshot.
pub async fn create_balance_snapshot(
    pool: &Pool,
    snapshot: &BalanceSnapshot,
) -> Result<Uuid, DatabaseError> {
    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    client.execute(
        r#"
        INSERT INTO balance_snapshots (
            id, vault_owner, total_balance, locked_balance,
            available_balance, timestamp, snapshot_type
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        &[
            &snapshot.id,
            &snapshot.vault_owner,
            &snapshot.total_balance,
            &snapshot.locked_balance,
            &snapshot.available_balance,
            &snapshot.timestamp,
            &snapshot.snapshot_type,
        ],
    ).await?;

    Ok(snapshot.id)
}

/// Get balance history for a vault.
#[allow(dead_code)]
pub async fn get_balance_history(
    pool: &Pool,
    owner: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<BalanceSnapshot>, DatabaseError> {
    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    let rows = client.query(
        r#"
        SELECT id, vault_owner, total_balance, locked_balance,
               available_balance, timestamp, snapshot_type
        FROM balance_snapshots
        WHERE vault_owner = $1 
          AND timestamp >= $2 
          AND timestamp <= $3
        ORDER BY timestamp ASC
        "#,
        &[&owner, &from, &to],
    ).await?;

    let mut snapshots = Vec::new();
    for row in rows {
        snapshots.push(BalanceSnapshot {
            id: row.get("id"),
            vault_owner: row.get("vault_owner"),
            total_balance: row.get("total_balance"),
            locked_balance: row.get("locked_balance"),
            available_balance: row.get("available_balance"),
            timestamp: row.get("timestamp"),
            snapshot_type: row.get("snapshot_type"),
        });
    }

    Ok(snapshots)
}

// ============================================
// TVL QUERIES
// ============================================

/// Get current Total Value Locked with breakdown.
///
/// Returns a tuple of (total_tvl, total_locked, total_available, active_vaults).
/// All values are in smallest units (6 decimals for USDT).
///
/// ## Returns
///
/// * `Ok((total_tvl, total_locked, total_available, active_vaults))` - TVL metrics
/// * `Err(DatabaseError)` - Database query failed
pub async fn get_current_tvl(pool: &Pool) -> Result<(i64, i64, i64, i64), DatabaseError> {
    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    let row = client.query_one(
        r#"
        SELECT 
            COALESCE(SUM(total_balance), 0) as total_tvl,
            COALESCE(SUM(locked_balance), 0) as total_locked,
            COALESCE(SUM(available_balance), 0) as total_available,
            COUNT(*) as active_vaults
        FROM vaults
        WHERE status = 'active'
        "#,
        &[],
    ).await?;

    Ok((
        row.get("total_tvl"),
        row.get("total_locked"),
        row.get("total_available"),
        row.get("active_vaults"),
    ))
}

/// Get vault count.
pub async fn get_vault_count(pool: &Pool) -> Result<i64, DatabaseError> {
    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    let row = client.query_one(
        r#"
        SELECT COUNT(*) as count FROM vaults WHERE status = 'active'
        "#,
        &[],
    ).await?;

    Ok(row.get("count"))
}

/// Create a TVL snapshot.
pub async fn create_tvl_snapshot(
    pool: &Pool,
    snapshot: &TvlSnapshot,
) -> Result<Uuid, DatabaseError> {
    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    client.execute(
        r#"
        INSERT INTO tvl_snapshots (
            id, total_value_locked, active_vaults,
            total_locked, total_available, timestamp
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        &[
            &snapshot.id,
            &snapshot.total_value_locked,
            &snapshot.active_vaults,
            &snapshot.total_locked,
            &snapshot.total_available,
            &snapshot.timestamp,
        ],
    ).await?;

    Ok(snapshot.id)
}

// ============================================
// RECONCILIATION QUERIES
// ============================================

/// Log a reconciliation event.
pub async fn create_reconciliation_log(
    pool: &Pool,
    log: &ReconciliationLog,
) -> Result<Uuid, DatabaseError> {
    let client = pool.get().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    client.execute(
        r#"
        INSERT INTO reconciliation_logs (
            id, vault_owner, expected_balance, actual_balance,
            difference, auto_fixed, notes, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        &[
            &log.id,
            &log.vault_owner,
            &log.expected_balance,
            &log.actual_balance,
            &log.difference,
            &log.auto_fixed,
            &log.notes,
            &log.created_at,
        ],
    ).await?;

    if log.difference != 0 {
        error!(
            "Reconciliation discrepancy: vault={:?}, expected={}, actual={}, diff={}",
            log.vault_owner, log.expected_balance, log.actual_balance, log.difference
        );
    }

    Ok(log.id)
}
