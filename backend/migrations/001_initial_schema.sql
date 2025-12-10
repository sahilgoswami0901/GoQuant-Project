-- ============================================
-- COLLATERAL VAULT DATABASE SCHEMA
-- ============================================
-- Migration 001: Initial Schema Setup
--
-- This migration creates all the tables needed for the
-- Collateral Vault backend service.
--
-- Tables Created:
-- 1. vaults - Cached vault account data
-- 2. transactions - Transaction history
-- 3. balance_snapshots - Periodic balance recordings
-- 4. reconciliation_logs - Audit trail
-- 5. tvl_snapshots - Total Value Locked history
-- 6. alerts - System alerts

-- ============================================
-- VAULTS TABLE
-- ============================================
-- Stores cached vault data from the blockchain.
-- This is the primary table that represents user vaults.
--
-- Notes:
-- - 'owner' is the primary key (user's wallet pubkey)
-- - Balance fields use BIGINT (64-bit) for large values
-- - All balances are in smallest units (6 decimals for USDT)
--   Example: 1 USDT = 1,000,000

CREATE TABLE IF NOT EXISTS vaults (
    -- User's wallet public key (base58 encoded)
    -- This uniquely identifies the vault
    owner VARCHAR(64) PRIMARY KEY,
    
    -- Vault PDA address (derived from program + user)
    vault_address VARCHAR(64) NOT NULL,
    
    -- Token account that holds the actual USDT
    token_account VARCHAR(64) NOT NULL,
    
    -- Current total balance (in smallest units)
    total_balance BIGINT NOT NULL DEFAULT 0,
    
    -- Balance locked for trading positions
    locked_balance BIGINT NOT NULL DEFAULT 0,
    
    -- Balance available for withdrawal (total - locked)
    available_balance BIGINT NOT NULL DEFAULT 0,
    
    -- Lifetime total deposits (only increases)
    total_deposited BIGINT NOT NULL DEFAULT 0,
    
    -- Lifetime total withdrawals (only increases)
    total_withdrawn BIGINT NOT NULL DEFAULT 0,
    
    -- When the vault was created on-chain
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- When this record was last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Vault status: 'active', 'paused', 'closed'
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    
    -- Constraints
    CONSTRAINT positive_balances CHECK (
        total_balance >= 0 AND 
        locked_balance >= 0 AND 
        available_balance >= 0
    ),
    CONSTRAINT balance_consistency CHECK (
        total_balance = locked_balance + available_balance
    )
);

-- Index for finding vaults by status
CREATE INDEX idx_vaults_status ON vaults(status);

-- Index for finding vaults by update time (for monitoring)
CREATE INDEX idx_vaults_updated_at ON vaults(updated_at DESC);

-- ============================================
-- TRANSACTIONS TABLE
-- ============================================
-- Records every vault operation for auditing and history.
-- Every deposit, withdrawal, lock, unlock, and transfer
-- creates a record here.

CREATE TABLE IF NOT EXISTS transactions (
    -- Unique transaction ID (UUID v4)
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Vault owner's public key (foreign key to vaults)
    vault_owner VARCHAR(64) NOT NULL REFERENCES vaults(owner),
    
    -- Type of transaction
    -- Values: 'deposit', 'withdrawal', 'lock', 'unlock', 
    --         'transfer_in', 'transfer_out', 'fee'
    transaction_type VARCHAR(20) NOT NULL,
    
    -- Amount involved (in smallest units)
    amount BIGINT NOT NULL,
    
    -- Solana transaction signature (base58)
    -- NULL if transaction is pending
    signature VARCHAR(128),
    
    -- Transaction status: 'pending', 'confirmed', 'failed'
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    
    -- Balance before this transaction
    balance_before BIGINT NOT NULL,
    
    -- Balance after this transaction
    balance_after BIGINT NOT NULL,
    
    -- For transfers: the other party's vault owner
    counterparty VARCHAR(64),
    
    -- Optional note or reason
    note TEXT,
    
    -- When the transaction was initiated
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- When the record was last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- When confirmed on-chain (NULL if pending/failed)
    confirmed_at TIMESTAMPTZ,
    
    -- Constraints
    CONSTRAINT positive_amount CHECK (amount >= 0)
);

-- Index for finding transactions by vault
CREATE INDEX idx_transactions_vault_owner ON transactions(vault_owner);

-- Index for finding transactions by type
CREATE INDEX idx_transactions_type ON transactions(transaction_type);

-- Index for finding transactions by status
CREATE INDEX idx_transactions_status ON transactions(status);

-- Index for finding transactions by time
CREATE INDEX idx_transactions_created_at ON transactions(created_at DESC);

-- Index for finding by signature (for verification)
CREATE INDEX idx_transactions_signature ON transactions(signature) 
WHERE signature IS NOT NULL;

-- ============================================
-- BALANCE SNAPSHOTS TABLE
-- ============================================
-- Periodic recordings of vault balances for analytics.
-- Snapshots are taken at regular intervals and on significant events.

CREATE TABLE IF NOT EXISTS balance_snapshots (
    -- Unique snapshot ID
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Vault owner's public key
    vault_owner VARCHAR(64) NOT NULL REFERENCES vaults(owner),
    
    -- Total balance at snapshot time
    total_balance BIGINT NOT NULL,
    
    -- Locked balance at snapshot time
    locked_balance BIGINT NOT NULL,
    
    -- Available balance at snapshot time
    available_balance BIGINT NOT NULL,
    
    -- When the snapshot was taken
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Type: 'periodic', 'event', 'daily_summary'
    snapshot_type VARCHAR(20) NOT NULL DEFAULT 'periodic'
);

-- Index for querying snapshots by vault and time range
CREATE INDEX idx_snapshots_vault_time 
ON balance_snapshots(vault_owner, timestamp DESC);

-- Index for cleanup of old snapshots
CREATE INDEX idx_snapshots_timestamp ON balance_snapshots(timestamp);

-- ============================================
-- RECONCILIATION LOGS TABLE
-- ============================================
-- Records of balance reconciliation between on-chain and database.
-- Used for auditing and detecting discrepancies.

CREATE TABLE IF NOT EXISTS reconciliation_logs (
    -- Unique log ID
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Vault owner (NULL for system-wide reconciliation)
    vault_owner VARCHAR(64),
    
    -- Balance we expected (from database)
    expected_balance BIGINT NOT NULL,
    
    -- Balance found on-chain
    actual_balance BIGINT NOT NULL,
    
    -- Difference (actual - expected)
    difference BIGINT NOT NULL,
    
    -- Whether discrepancy was auto-fixed
    auto_fixed BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- Notes about the reconciliation
    notes TEXT,
    
    -- When reconciliation was performed
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for finding discrepancies
CREATE INDEX idx_reconciliation_difference 
ON reconciliation_logs(difference) 
WHERE difference != 0;

-- Index for finding reconciliations by vault
CREATE INDEX idx_reconciliation_vault 
ON reconciliation_logs(vault_owner) 
WHERE vault_owner IS NOT NULL;

-- ============================================
-- TVL SNAPSHOTS TABLE
-- ============================================
-- Total Value Locked snapshots for protocol analytics.

CREATE TABLE IF NOT EXISTS tvl_snapshots (
    -- Unique snapshot ID
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Total USDT across all vaults
    total_value_locked BIGINT NOT NULL,
    
    -- Number of active vaults
    active_vaults BIGINT NOT NULL,
    
    -- Total locked for positions
    total_locked BIGINT NOT NULL,
    
    -- Total available for withdrawal
    total_available BIGINT NOT NULL,
    
    -- When snapshot was taken
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for time-based queries
CREATE INDEX idx_tvl_timestamp ON tvl_snapshots(timestamp DESC);

-- ============================================
-- ALERTS TABLE
-- ============================================
-- System alerts for monitoring and notifications.

CREATE TABLE IF NOT EXISTS alerts (
    -- Unique alert ID
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Severity: 'info', 'warning', 'critical'
    severity VARCHAR(20) NOT NULL,
    
    -- Alert type: 'low_balance', 'large_tx', 'failed_tx', etc.
    alert_type VARCHAR(50) NOT NULL,
    
    -- Related vault owner (NULL for system alerts)
    vault_owner VARCHAR(64),
    
    -- Human-readable message
    message TEXT NOT NULL,
    
    -- Additional data as JSON
    data JSONB,
    
    -- Whether alert has been acknowledged
    acknowledged BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- When alert was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- When alert was acknowledged
    acknowledged_at TIMESTAMPTZ
);

-- Index for unacknowledged alerts
CREATE INDEX idx_alerts_unacknowledged 
ON alerts(created_at DESC) 
WHERE acknowledged = FALSE;

-- Index for alerts by severity
CREATE INDEX idx_alerts_severity ON alerts(severity, created_at DESC);

-- ============================================
-- FUNCTIONS AND TRIGGERS
-- ============================================

-- Function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger to auto-update updated_at on vaults
CREATE TRIGGER update_vaults_updated_at
    BEFORE UPDATE ON vaults
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger to auto-update updated_at on transactions
CREATE TRIGGER update_transactions_updated_at
    BEFORE UPDATE ON transactions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

