# Collateral Vault Backend Service

A Rust backend service for managing the Collateral Vault system on Solana.

## 📋 Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Components](#components)
4. [API Reference](#api-reference)
5. [Database Schema](#database-schema)
6. [Configuration](#configuration)
7. [Setup & Running](#setup--running)
8. [Development](#development)

---

## Overview

This backend service provides:

- **REST API** for vault operations (deposit, withdraw, balance queries)
- **WebSocket** connections for real-time updates
- **Background Services** for monitoring and reconciliation
- **Database Storage** for transaction history and analytics

### What It Does

```
┌─────────────────────────────────────────────────────────────────┐
│                     USER / FRONTEND                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    BACKEND SERVICE                               │
│                                                                  │
│  1. Receives deposit/withdraw request                            │
│  2. Builds Solana transaction                                    │
│  3. Returns unsigned transaction to user                         │
│  4. User signs with their wallet                                 │
│  5. User submits to Solana                                       │
│  6. Backend monitors for confirmation                            │
│  7. Updates database with new state                              │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌─────────────────────────┐   ┌─────────────────────────┐
│      PostgreSQL          │   │        Solana           │
│      (History)           │   │      (Source of Truth)  │
└─────────────────────────┘   └─────────────────────────┘
```

---

## Architecture

### Directory Structure

```
backend/
├── Cargo.toml              # Dependencies
├── migrations/             # Database migrations
│   └── 001_initial_schema.sql
├── src/
│   ├── main.rs            # Entry point
│   ├── config/            # Configuration loading
│   │   └── mod.rs
│   ├── db/                # Database layer
│   │   ├── mod.rs         # Connection & pool
│   │   ├── models.rs      # Data structures
│   │   └── queries.rs     # SQL queries
│   ├── models/            # API request/response models
│   │   ├── mod.rs
│   │   ├── requests.rs
│   │   └── responses.rs
│   ├── services/          # Business logic
│   │   ├── mod.rs
│   │   ├── vault_manager.rs
│   │   ├── balance_tracker.rs
│   │   ├── transaction_builder.rs
│   │   └── vault_monitor.rs
│   ├── solana/            # Solana RPC client
│   │   └── mod.rs
│   ├── api/               # HTTP handlers
│   │   ├── mod.rs
│   │   ├── routes.rs
│   │   └── handlers.rs
│   ├── websocket/         # Real-time updates
│   │   └── mod.rs
│   └── utils/             # Helper functions
│       └── mod.rs
└── README.md
```

### Service Components

| Component | Description |
|-----------|-------------|
| **VaultManager** | Core vault operations (deposit, withdraw, lock/unlock) |
| **BalanceTracker** | Monitors balances, reconciles with blockchain |
| **TransactionBuilder** | Creates Solana transactions |
| **VaultMonitor** | Background monitoring, alerts, TVL tracking |

---

## Components

### 1. Vault Manager

The central service for vault operations.

```rust
// Initialize a vault
let result = vault_manager.initialize_vault("user_pubkey").await?;

// Deposit USDT
let result = vault_manager.deposit(DepositRequest {
    user_pubkey: "7xKt9Fj2...".to_string(),
    amount: 100_000_000, // 100 USDT
    signature: None,
}).await?;

// Withdraw USDT
let result = vault_manager.withdraw(WithdrawRequest {
    user_pubkey: "7xKt9Fj2...".to_string(),
    amount: 50_000_000, // 50 USDT
}).await?;

// Get balance
let balance = vault_manager.get_vault_balance("7xKt9Fj2...").await?;
```

### 2. Balance Tracker

Ensures database cache is synchronized with blockchain.

```rust
// Start reconciliation loop (background)
tracker.start_reconciliation_loop().await;

// Manually reconcile a vault
let had_discrepancy = tracker.reconcile_vault("7xKt9Fj2...").await?;

// Get TVL
let tvl = tracker.get_tvl().await?;
```

### 3. Transaction Builder

Creates properly formatted Solana transactions.

```rust
// Build deposit transaction
let tx_base64 = builder.build_deposit("user_pubkey", amount).await?;

// Returns base64-encoded unsigned transaction
// Frontend signs and submits
```

### 4. Vault Monitor

Background monitoring and alerting.

```rust
// Start monitoring (background)
monitor.start().await;

// Get system status
let status = monitor.get_system_status().await;
```

---

## API Reference

### Base URL

```
http://localhost:8080
```

### Endpoints

#### Health Check

```http
GET /health
```

**Response:**
```json
{
    "success": true,
    "data": {
        "status": "healthy",
        "database": true,
        "solanaRpc": true,
        "version": "0.1.0"
    }
}
```

#### Initialize Vault

```http
POST /vault/initialize
Content-Type: application/json

{
    "userPubkey": "7xKt9Fj2abc123..."
}
```

**Response:**
```json
{
    "success": true,
    "data": {
        "transactionId": "550e8400-e29b-41d4-a716-446655440000",
        "status": "pending",
        "unsignedTransaction": "base64...",
        "message": "Sign and submit this transaction"
    }
}
```

#### Deposit

```http
POST /vault/deposit
Content-Type: application/json

{
    "userPubkey": "7xKt9Fj2abc123...",
    "amount": 100000000
}
```

**Note:** Amount is in smallest units (6 decimals). 1 USDT = 1,000,000.

#### Withdraw

```http
POST /vault/withdraw
Content-Type: application/json

{
    "userPubkey": "7xKt9Fj2abc123...",
    "amount": 50000000
}
```

#### Get Balance

```http
GET /vault/balance/7xKt9Fj2abc123...
```

**Response:**
```json
{
    "success": true,
    "data": {
        "owner": "7xKt9Fj2abc123...",
        "totalBalance": 1000000000,
        "lockedBalance": 200000000,
        "availableBalance": 800000000,
        "formattedTotal": "1000.00 USDT",
        "formattedAvailable": "800.00 USDT"
    }
}
```

#### Get Transaction History

```http
GET /vault/transactions/7xKt9Fj2abc123...?limit=20&offset=0
```

#### Get TVL

```http
GET /vault/tvl
```

**Response:**
```json
{
    "success": true,
    "data": {
        "totalValueLocked": 50000000000,
        "formattedTvl": "50,000.00 USDT",
        "activeVaults": 1234
    }
}
```

### WebSocket

Connect to receive real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:8080/ws/7xKt9Fj2abc123...');

ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    console.log('Event:', message.event);
    console.log('Data:', message.data);
};
```

**Events:**
- `balance_update` - Balance changed
- `transaction_confirmed` - Transaction confirmed on-chain
- `collateral_locked` - Collateral locked for position
- `collateral_unlocked` - Collateral released

---

## Database Schema

### Tables

#### `vaults`
Cached vault account data.

| Column | Type | Description |
|--------|------|-------------|
| owner | VARCHAR(64) | Primary key, user's pubkey |
| vault_address | VARCHAR(64) | Vault PDA address |
| total_balance | BIGINT | Total USDT balance |
| locked_balance | BIGINT | Locked for positions |
| available_balance | BIGINT | Free to withdraw |
| status | VARCHAR(20) | active, paused, closed |

#### `transactions`
Transaction history.

| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| vault_owner | VARCHAR(64) | Foreign key to vaults |
| transaction_type | VARCHAR(20) | deposit, withdrawal, etc. |
| amount | BIGINT | Transaction amount |
| signature | VARCHAR(128) | Solana tx signature |
| status | VARCHAR(20) | pending, confirmed, failed |

#### `balance_snapshots`
Historical balance recordings.

#### `reconciliation_logs`
Audit trail for balance reconciliation.

#### `tvl_snapshots`
Total Value Locked history.

---

## Configuration

### Environment Variables

```bash
# Database
DATABASE_URL=postgres://postgres:password@localhost:5432/collateral_vault

# Solana
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_WS_URL=wss://api.devnet.solana.com
VAULT_PROGRAM_ID=AVRBwuFHdU51wxP3a8brB95KL1VT7PCrFAVt1zzmjde
USDT_MINT=Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB

# Server
SERVER_HOST=127.0.0.1
SERVER_PORT=8080

# Monitoring
BALANCE_CHECK_INTERVAL=30
RECONCILIATION_INTERVAL=300
LOW_BALANCE_THRESHOLD=100
```

---

## Setup & Running

### Prerequisites

- Rust 1.75+
- PostgreSQL 14+
- Solana CLI (for testing)

### Installation

```bash
# Clone and navigate
cd collateral-vault/backend

# Create database
createdb collateral_vault

# Set environment variables
cp .env.example .env
# Edit .env with your values

# Run migrations
cargo install sqlx-cli
sqlx migrate run

# Build and run
cargo run
```

### Docker (Optional)

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/vault-backend /usr/local/bin/
CMD ["vault-backend"]
```

---

## Development

### Running Tests

```bash
cargo test
```

### Logging

Set log level via `RUST_LOG`:

```bash
RUST_LOG=debug cargo run
RUST_LOG=info,collateral_vault_backend=debug cargo run
```

### Database Migrations

```bash
# Create new migration
sqlx migrate add <name>

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

---

## License

Confidential - GoQuant Assignment

