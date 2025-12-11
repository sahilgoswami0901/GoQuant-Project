# Collateral Vault Management System

A Solana smart contract (Anchor program) for managing user collateral in a decentralized perpetual futures exchange.

## 📋 Table of Contents

1. [Overview](#overview)
2. [System Flow](#system-flow)
3. [Database Schema](#database-schema)
4. [API Commands](#api-commands)
5. [Architecture](#architecture)
6. [Key Concepts](#key-concepts)
7. [Project Structure](#project-structure)
8. [Account Structures](#account-structures)
9. [Instructions Reference](#instructions-reference)
10. [PDA Derivation](#pda-derivation)
11. [Security Model](#security-model)
12. [Events](#events)
13. [Error Handling](#error-handling)
14. [Setup & Installation](#setup--installation)
15. [Testing](#testing)
16. [Deployment](#deployment)

---

## Overview

### What is this?

This is the **custody layer** for a perpetual futures DEX on Solana. It manages:

- **User Deposits**: Users deposit USDT collateral into secure vaults
- **Balance Tracking**: Tracks total, locked, and available balances
- **Position Margin**: Locks collateral when users open trading positions
- **Settlements**: Transfers funds between users for trade settlement
- **Withdrawals**: Allows users to withdraw available funds

### Why is it needed?

In a perpetual futures exchange:

```
User deposits $1000 USDT
    ↓
Opens 10x leveraged $5000 position
    ↓
System locks $500 as margin
    ↓
Position moves in profit (+$200)
    ↓
User closes position
    ↓
$500 unlocked + $200 profit = $700 available
    ↓
User can withdraw $1700 total
```

This program manages all of the above securely on-chain.

---

## System Flow

### Complete Deposit Flow: From Minting to Vault

The collateral vault system follows a specific flow from acquiring USDT to depositing it into the vault:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    COMPLETE DEPOSIT FLOW                                │
└─────────────────────────────────────────────────────────────────────────┘

Step 1: Mint USDT (Devnet Testing Only)
    │
    ├─ POST /vault/mint-usdt
    │  └─ Mints test USDT to user's token account
    │
    ↓
    User's Associated Token Account (ATA)
    └── USDT Balance: 1000 USDT ✅
    │
    ↓
Step 2: Initialize Vault (One-time setup)
    │
    ├─ POST /vault/initialize
    │  └─ Creates Vault PDA account on-chain
    │  └─ Creates Vault Token Account (ATA for Vault PDA)
    │
    ↓
    Vault PDA Account
    ├── owner: User's Public Key
    ├── total_balance: 0
    ├── locked_balance: 0
    └── available_balance: 0
    │
    Vault Token Account (ATA)
    └── USDT Balance: 0
    │
    ↓
Step 3: Deposit USDT
    │
    ├─ POST /vault/deposit
    │  └─ Amount: 100 USDT (100,000,000 in smallest units)
    │
    ↓
    [On-Chain Transaction via CPI]
    │
    ├─ SPL Token Program Transfer
    │  ├─ From: User's Token Account
    │  ├─ To: Vault Token Account
    │  └─ Authority: User (signs transaction)
    │
    ↓
    User's Token Account
    └── USDT Balance: 900 USDT (-100) ✅
    │
    Vault Token Account
    └── USDT Balance: 100 USDT (+100) ✅
    │
    Vault PDA Account (Updated)
    ├── total_balance: 100 USDT
    ├── locked_balance: 0
    └── available_balance: 100 USDT ✅
```

### Account Relationships Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         ACCOUNT STRUCTURE                                │
└─────────────────────────────────────────────────────────────────────────┘

User Wallet (Keypair)
│
├─ Public Key: 5cGSnAXuUBvm6gbAnEhUjy54m6REqrfP5Z6K99tuF6mZ
│
├─ Associated Token Account (ATA) ────────────┐
│  │                                          │
│  ├─ Mint: USDT Mint Address                 │
│  ├─ Owner: User's Public Key                │
│  └─ Balance: 1000 USDT                      │
│                                             │
│                                             │ CPI Transfer
│                                             │ (User signs)
│                                             │
│                                             ▼
│                          ┌─────────────────────────────────────┐
│                          │      Vault PDA (Program Account)    │
│                          │                                     │
│                          │  Seeds: ["vault", user_pubkey]      │
│                          │  Bump: 255                          │
│                          │                                     │
│                          │  State:                             │
│                          │  ├─ owner: User's Public Key        │
│                          │  ├─ total_balance: 100 USDT         │
│                          │  ├─ locked_balance: 0               │
│                          │  └─ available_balance: 100 USDT     │
│                          │                                     │
│                          │  └─ Associated Token Account (ATA)  │
│                          │     ├─ Mint: USDT Mint Address      │
│                          │     ├─ Owner: Vault PDA             │
│                          │     └─ Balance: 100 USDT ✅         │
│                          └─────────────────────────────────────┘
│
└─ Note: Vault PDA has no private key - only program can control it
```

### Key Components Explained

1. **User Token Account**: SPL Token account owned by the user where they hold USDT
2. **Vault PDA**: Program Derived Address that stores vault state (balances, owner info)
3. **Vault Token Account**: SPL Token account owned by the Vault PDA where collateral is stored
4. **SPL Token Program**: Solana's standard token program that handles all token transfers via CPI

---

## Database Schema

The backend uses PostgreSQL to maintain an off-chain database for fast queries, transaction history, and reconciliation with on-chain state.

### Schema Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        DATABASE SCHEMA                                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────┐
│      vaults         │ ◄─── Central Entity
├─────────────────────┤
│ owner (PK)          │ ────┐
│ vault_address       │     │
│ token_account       │     │
│ total_balance       │     │
│ locked_balance      │     │
│ available_balance   │     │
│ total_deposited     │     │
│ total_withdrawn     │     │
│ created_at          │     │
│ updated_at          │     │
│ status              │     │
└─────────────────────┘     │
                            │
        ┌───────────────────┼
        │                   │                  
        ▼                   ▼                   
┌──────────────────┐ ┌──────────────┐ ┌─────────────────────┐
│balance_snapshots │ │ transactions │ │reconciliation_logs  │
├──────────────────┤ ├──────────────┤ ├─────────────────────┤
│ id (PK)          │ │ id (PK)      │ │ id (PK)             │
│ vault_owner (FK) │ │ vault_owner  │ │ vault_owner         │
│ total_balance    │ │ (FK)         │ │ expected_balance    │
│ locked_balance   │ │ type         │ │ actual_balance      │
│ available_balance│ │ amount       │ │ difference          │
│ timestamp        │ │ signature    │ │ auto_fixed          │
│ snapshot_type    │ │ status       │ │ notes               │
└──────────────────┘ │ balance_*    │ │ created_at          │
                     │ counterparty │ └─────────────────────┘
                     │ note         │
                     │ created_at   │
                     │ updated_at   │
                     │ confirmed_at │
                     └──────────────┘
                            │
                            │
┌───────────────────────────┐
│      tvl_snapshots        │ (Aggregate Data)
├───────────────────────────┤
│ id (PK)                   │
│ total_value_locked        │
│ active_vaults             │
│ total_locked              │
│ total_available           │
│ timestamp                 │
└───────────────────────────┘

┌───────────────────────────┐
│        alerts             │ (System-wide)
├───────────────────────────┤
│ id (PK)                   │
│ severity                  │
│ alert_type                │
│ vault_owner (FK, nullable)│
│ message                   │
│ data (jsonb)              │
│ acknowledged              │
│ created_at                │
│ acknowledged_at           │
└───────────────────────────┘
```

### Table Descriptions

#### 1. `vaults` (Central Entity)
Stores core information for each user's collateral vault.

- **`owner`** (PK): User's Solana public key (varchar(64))
- **`vault_address`**: On-chain PDA address of the vault
- **`token_account`**: On-chain SPL token account address for the vault
- **`total_balance`**: Total USDT in vault (int8, in smallest units)
- **`locked_balance`**: Locked USDT for positions (int8)
- **`available_balance`**: Available USDT for withdrawal (int8)
- **`total_deposited`**: Lifetime deposits (int8)
- **`total_withdrawn`**: Lifetime withdrawals (int8)
- **`created_at`**: Vault creation timestamp (timestamptz)
- **`updated_at`**: Last update timestamp (timestamptz)
- **`status`**: Vault status (varchar(20), e.g., 'active', 'inactive')

#### 2. `balance_snapshots`
Historical balance records for auditing and trend analysis.

- **`id`** (PK): Unique snapshot identifier (uuid)
- **`vault_owner`** (FK): References `vaults.owner`
- **`total_balance`**, **`locked_balance`**, **`available_balance`**: Balance at snapshot time
- **`timestamp`**: When snapshot was taken (timestamptz)
- **`snapshot_type`**: Reason for snapshot (varchar(20), e.g., 'hourly', 'event_driven')

#### 3. `transactions`
Complete audit trail of all vault operations.

- **`id`** (PK): Unique transaction identifier (uuid)
- **`vault_owner`** (FK): References `vaults.owner`
- **`transaction_type`**: Type of operation (varchar(20), e.g., 'deposit', 'withdraw', 'lock', 'unlock', 'transfer')
- **`amount`**: Transaction amount (int8)
- **`signature`**: Solana transaction signature (varchar(128), nullable)
- **`status`**: Transaction status (varchar(20), e.g., 'pending', 'submitted', 'confirmed', 'failed')
- **`balance_before`**, **`balance_after`**: Vault balance before/after transaction
- **`counterparty`**: Other party in transfer (varchar(64), nullable)
- **`note`**: Additional notes (text, nullable)
- **`created_at`**, **`updated_at`**, **`confirmed_at`**: Timestamps

#### 4. `tvl_snapshots`
System-wide Total Value Locked metrics.

- **`id`** (PK): Unique snapshot identifier (uuid)
- **`total_value_locked`**: Sum of all vault balances (int8)
- **`active_vaults`**: Number of active vaults (int8)
- **`total_locked`**: Aggregate locked balance (int8)
- **`total_available`**: Aggregate available balance (int8)
- **`timestamp`**: Snapshot time (timestamptz)

#### 5. `reconciliation_logs`
Discrepancies between database and on-chain state.

- **`id`** (PK): Unique log identifier (uuid)
- **`vault_owner`** (FK): References `vaults.owner`
- **`expected_balance`**: Database balance (int8)
- **`actual_balance`**: On-chain balance (int8)
- **`difference`**: Calculated difference (int8)
- **`auto_fixed`**: Whether discrepancy was auto-resolved (bool)
- **`notes`**: Detailed explanation (text, nullable)
- **`created_at`**: Log creation timestamp (timestamptz)

#### 6. `alerts`
System alerts for critical events and issues.

- **`id`** (PK): Unique alert identifier (uuid)
- **`severity`**: Alert level (varchar(20), e.g., 'info', 'warning', 'error', 'critical')
- **`alert_type`**: Alert category (varchar(50), e.g., 'reconciliation_failure', 'rpc_timeout')
- **`vault_owner`** (FK, nullable): References `vaults.owner` if vault-specific
- **`message`**: Human-readable description (text)
- **`data`**: Additional structured data (jsonb, nullable)
- **`acknowledged`**: Whether alert was reviewed (bool)
- **`created_at`**, **`acknowledged_at`**: Timestamps

### Relationships

- **One-to-Many**: One `vaults` entry can have many `balance_snapshots`, `transactions`, and `reconciliation_logs`
- **Optional**: `alerts` can be vault-specific (via `vault_owner`) or system-wide (null `vault_owner`)
- **Aggregate**: `tvl_snapshots` derives data from all `vaults` entries

---

## API Commands

The backend provides a REST API for interacting with the collateral vault system. All endpoints are documented in [`API_COMMANDS.md`](./API_COMMANDS.md).

### Available Endpoints

| Method | Endpoint | Description | Signer |
|--------|----------|-------------|--------|
| `GET` | `/health` | Health check | None |
| `POST` | `/vault/initialize` | Create a new vault | User |
| `POST` | `/vault/deposit` | Deposit USDT into vault | User |
| `POST` | `/vault/withdraw` | Withdraw USDT from vault | User |
| `POST` | `/vault/lock-collateral` | Lock collateral for position | Position Manager |
| `POST` | `/vault/unlock-collateral` | Unlock collateral | Position Manager |
| `POST` | `/vault/transfer-collateral` | Transfer between vaults | Liquidation Engine |
| `POST` | `/vault/mint-usdt` | Mint test USDT (devnet only) | User |
| `GET` | `/vault/balance` | Get vault balance | None |
| `GET` | `/vault/transactions` | Get transaction history | None |
| `GET` | `/vault/tvl` | Get Total Value Locked | None |

### Quick Examples

#### Initialize Vault
```bash
curl -X POST http://127.0.0.1:8080/vault/initialize \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS",
    "userKeypairPath": "~/.config/solana/id.json"
  }'
```

#### Deposit USDT
```bash
curl -X POST http://127.0.0.1:8080/vault/deposit \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS",
    "amount": 100000000,
    "userKeypairPath": "~/.config/solana/id.json"
  }'
```

#### Lock Collateral (Position Manager)
```bash
curl -X POST http://127.0.0.1:8080/vault/lock-collateral \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "USER_VAULT_OWNER",
    "amount": 50000000,
    "positionId": "pos_123",
    "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
  }'
```

For complete API documentation with all request/response formats, error codes, and examples, see [`API_COMMANDS.md`](./API_COMMANDS.md).

---

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           CLIENT (Web/Mobile)                           │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         BACKEND SERVICE (Rust)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │   Vault     │  │  Balance    │  │ Transaction │  │    Vault    │     │
│  │  Manager    │  │  Tracker    │  │   Builder   │  │   Monitor   │     │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            SOLANA BLOCKCHAIN                            │
│                                                                         │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                    COLLATERAL VAULT PROGRAM                        │ │
│  │                                                                    │ │
│  │   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐              │ │
│  │   │ User Vault  │   │ User Vault  │   │ User Vault  │    ...       │ │
│  │   │   (PDA)     │   │   (PDA)     │   │   (PDA)     │              │ │
│  │   └─────────────┘   └─────────────┘   └─────────────┘              │ │
│  │                                                                    │ │
│  │   ┌──────────────────────────────────────────────────────────────┐ │ │
│  │   │                    Vault Authority (PDA)                     │ │ │
│  │   │  • Admin control  • Authorized programs  • Pause state       │ │ │
│  │   └──────────────────────────────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                    │                                    │
│                                    │ CPI                                │
│                                    ▼                                    │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                       SPL TOKEN PROGRAM                            │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Interactions

```
Position Manager Program              Vault Program                 SPL Token Program
        │                                  │                              │
        │ 1. User opens position           │                              │
        │──────────────────────────────────>                              │
        │   lock_collateral(100 USDT)      │                              │
        │                                  │                              │
        │                                  │ 2. Validate & update state   │
        │                                  │                              │
        │<─────────────────────────────────│                              │
        │   Success                        │                              │
        │                                  │                              │
        │ 3. User closes position          │                              │
        │──────────────────────────────────>                              │
        │   unlock_collateral(100 USDT)    │                              │
        │                                  │                              │
        │                                  │ 4. Update state              │
        │<─────────────────────────────────│                              │
        │   Success                        │                              │
```

---

## Key Concepts

### 1. Program Derived Addresses (PDAs)

PDAs are special Solana addresses that:
- Have no private key
- Can only be "signed" by the program
- Are derived deterministically from seeds

```rust
// Vault PDA derivation
seeds = ["vault", user_public_key]
bump = 255, 254, 253... (until valid PDA found)
```

**Why PDAs?**
- Trustless: No one can steal funds, even developers
- Deterministic: Anyone can calculate vault address from user's pubkey
- Secure: Only the program can authorize vault transactions

### 2. SPL Token Program

The standard token program on Solana. Our program uses it via CPI (Cross-Program Invocation):

```rust
// Deposit: User → Vault
token::transfer(
    CpiContext::new(...),
    Transfer {
        from: user_token_account,
        to: vault_token_account,
        authority: user,  // User signs
    }
)?;

// Withdraw: Vault → User
token::transfer(
    CpiContext::new_with_signer(..., signer_seeds),  // PDA signs
    Transfer {
        from: vault_token_account,
        to: user_token_account,
        authority: vault_pda,  // Program provides seeds
    }
)?;
```

### 3. Balance Types

| Balance Type | Description | Can Withdraw? |
|--------------|-------------|---------------|
| `total_balance` | All USDT in vault | - |
| `locked_balance` | Reserved for positions | ❌ |
| `available_balance` | Free to use | ✅ |

**Invariant**: `total_balance = locked_balance + available_balance`

### 4. Cross-Program Invocation (CPI)

When one Solana program calls another. The Collateral Vault program uses CPIs extensively to interact with the SPL Token Program:

#### CPI Usage Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    CPI (Cross-Program Invocation)                       │
└─────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────┐                    ┌──────────────────────────┐
│  Collateral Vault        │                    │   SPL Token Program      │
│  Program                 │                    │   (Standard Library)     │
│                          │                    │                          │
│  ┌────────────────────┐  │                    │  ┌──────────────────────┐│
│  │ deposit()          │  │                    │  │ transfer()           ││
│  │  └─> CPI call      │─ ┼─── transfer(100) ──┼─>│ mint_to()            ││
│  └────────────────────┘  │                    │  │ burn()               ││
│                          │                    │  │ approve()            ││
│  ┌────────────────────┐  │                    │  └──────────────────────┘│
│  │ withdraw()         │  │                    │                          │
│  │  └─> CPI call      │─ ┼─── transfer(50) ───┼─>                        │
│  └────────────────────┘  │                    │                          │
│                          │                    │                          │
│  ┌────────────────────┐  │                    │                          │
│  │ transfer_collateral│  │                    │                          │
│  │  └─> CPI call      │─ ┼─── transfer(25) ───┼─>                        │
│  └────────────────────┘  │                    │                          │
│                          │                    │                          │
│  CPI Context:            │                    │  Executes token          │
│  ├─ program_id           │                    │  operations securely     │
│  ├─ accounts             │                    │                          │
│  └─ signers (PDA seeds)  │                    │                          │
└──────────────────────────┘                    └──────────────────────────┘
```

#### CPI Examples in Code

**Deposit (User → Vault):**
```rust
// User signs transaction, transfers from their token account
token::transfer(
    CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(), // User signs
        },
    ),
    amount,
)?;
```

**Withdraw (Vault → User):**
```rust
// Vault PDA signs via seeds, transfers from vault token account
token::transfer(
    CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(), // Vault PDA
        },
        &[&[
            b"vault",
            ctx.accounts.user.key.as_ref(),
            &[ctx.accounts.vault.bump],
        ]], // PDA seeds for signing
    ),
    amount,
)?;
```

**Transfer Collateral (Vault → Vault):**
```rust
// Authorized program signs, transfers between vaults
token::transfer(
    CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.from_token_account.to_account_info(),
            to: ctx.accounts.to_token_account.to_account_info(),
            authority: ctx.accounts.from_vault.to_account_info(), // From vault PDA
        },
        &[&[
            b"vault",
            ctx.accounts.from_vault.owner.as_ref(),
            &[ctx.accounts.from_vault.bump],
        ]], // From vault PDA seeds
    ),
    amount,
)?;
```

### 5. PDA (Program Derived Address) Usage

PDAs are used extensively for security and deterministic addressing:

#### PDA Derivation Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│              PDA (Program Derived Address) Derivation                   │
└─────────────────────────────────────────────────────────────────────────┘

Vault PDA:
┌─────────────────────────────────────────────────────────────┐
│ Seeds: ["vault", user_public_key]                           │
│ Program ID: CollateralVault Program                         │
│                                                             │
│ find_program_address(seeds, program_id)                     │
│   → Try bump = 255                                          │
│   → Try bump = 254                                          │
│   → Try bump = 253                                          │
│   → ... until valid PDA found                               │
│                                                             │
│ Result:                                                     │
│ ├─ Address: 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU    │
│ ├─ Bump: 255                                                │
│ └─ No private key exists!                                   │
└─────────────────────────────────────────────────────────────┘

Vault Authority PDA:
┌─────────────────────────────────────────────────────────────┐
│ Seeds: ["vault_authority"]                                  │
│ Program ID: CollateralVault Program                         │
│                                                             │
│ Result:                                                     │
│ ├─ Address: 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM    │
│ ├─ Bump: 254                                                │
│ └─ Singleton (one per program)                              │
└─────────────────────────────────────────────────────────────┘

Vault Token Account (ATA):
┌─────────────────────────────────────────────────────────────┐
│ get_associated_token_address(                               │
│   mint: USDT_MINT,                                          │
│   owner: Vault PDA,                                         │
│   allow_owner_off_curve: true  ← Important for PDAs!        │
│ )                                                           │
│                                                             │
│ Result:                                                     │
│ ├─ Address: 5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1    │
│ └─ Owned by Vault PDA                                       │
└─────────────────────────────────────────────────────────────┘
```

#### PDA Signing Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    PDA Signing Mechanism                                 │
└─────────────────────────────────────────────────────────────────────────┘

Regular Account:                          PDA Account:
┌──────────────────┐                     ┌──────────────────┐
│ Private Key:     │                     │ Private Key:     │
│ └─ Exists ✅     │                     │ └─ Does NOT exist │
│                  │                     │                  │
│ Signing:         │                     │ Signing:         │
│ └─ Use private   │                     │ └─ Use seeds +   │
│    key directly  │                     │    bump (program │
│                  │                     │    provides)     │
└──────────────────┘                     └──────────────────┘
        │                                          │
        │                                          │
        ▼                                          ▼
┌──────────────────┐                     ┌──────────────────┐
│ transaction.sign │                     │ invoke_signed()  │
│ ([keypair])      │                     │ with seeds array │
└──────────────────┘                     └──────────────────┘
```

#### PDA Usage in Instructions

**Vault PDA:**
- **Seeds**: `["vault", user_pubkey]`
- **Purpose**: Stores vault state, signs for withdrawals
- **Security**: Only program can sign (no private key)

**Vault Authority PDA:**
- **Seeds**: `["vault_authority"]`
- **Purpose**: Global configuration, authorized programs list
- **Security**: Admin-controlled, program-enforced

**Vault Token Account (ATA):**
- **Derived from**: Vault PDA + USDT Mint
- **Purpose**: Holds actual USDT tokens
- **Security**: Owned by Vault PDA (program-controlled)

### 6. SPL Token Program Integration

The Collateral Vault program relies heavily on the SPL Token Program for all token operations:

#### SPL Token Program Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│              SPL Token Program Integration                              │
└─────────────────────────────────────────────────────────────────────────┘

┌──────────────────────┐
│  Token Mint          │
│  (USDT)              │
│  └─ Address: ...     │
└──────────────────────┘
         │
         │ Creates
         ▼
┌──────────────────────┐              ┌──────────────────────┐
│  User Token Account  │              │  Vault Token Account │
│  (ATA)               │              │  (ATA)               │
│                      │              │                      │
│  Owner: User         │              │  Owner: Vault PDA    │
│  Mint: USDT          │              │  Mint: USDT          │
│  Balance: 1000 USDT  │              │  Balance: 100 USDT   │
└──────────────────────┘              └──────────────────────┘
         │                                      │
         │                                      │
         │  Transfer (CPI)                      │
         └──────────────────────────────────────┘
                      │
                      ▼
         ┌──────────────────────┐
         │  SPL Token Program   │
         │                      │
         │  Validates:          │
         │  ├─ Authority        │
         │  ├─ Balance          │
         │  └─ Accounts         │
         │                      │
         │  Executes Transfer   │
         └──────────────────────┘
```

#### Token Account Types

| Account Type | Owner | Purpose | Example |
|--------------|-------|---------|---------|
| **User Token Account** | User's Public Key | User holds USDT | User's ATA for USDT |
| **Vault Token Account** | Vault PDA | Vault holds collateral | Vault PDA's ATA for USDT |
| **Mint Account** | Mint Authority | Token definition | USDT Mint |

#### SPL Token Instructions Used

1. **`transfer`**: Move tokens between accounts (used in deposit, withdraw, transfer_collateral)
2. **`mint_to`**: Create new tokens (used in mint-usdt endpoint for testing)
3. **`get_associated_token_address`**: Derive ATA addresses deterministically

All token operations go through the SPL Token Program via CPI, ensuring security and standardization.

---

## Project Structure

```
collateral-vault/
├── Anchor.toml                 # Anchor configuration
├── Cargo.toml                  # Rust workspace config
├── programs/
│   └── collateral-vault/
│       ├── Cargo.toml          # Program dependencies
│       └── src/
│           ├── lib.rs          # Main program entry point
│           ├── errors/
│           │   └── mod.rs      # Custom error definitions
│           ├── events/
│           │   └── mod.rs      # Event definitions for indexing
│           ├── instructions/
│           │   ├── mod.rs      # Instruction exports
│           │   ├── initialize_vault.rs
│           │   ├── initialize_vault_authority.rs
│           │   ├── deposit.rs
│           │   ├── withdraw.rs
│           │   ├── lock_collateral.rs
│           │   ├── unlock_collateral.rs
│           │   └── transfer_collateral.rs
│           └── state/
│               ├── mod.rs      # State exports
│               ├── vault.rs    # CollateralVault account
│               └── vault_authority.rs  # VaultAuthority account
├── tests/                      # Integration tests
└── README.md                   # This file
```

---

## Account Structures

### CollateralVault

The main vault account for each user.

```rust
#[account]
pub struct CollateralVault {
    pub owner: Pubkey,              // 32 bytes - Vault owner
    pub token_account: Pubkey,      // 32 bytes - USDT token account
    pub total_balance: u64,         // 8 bytes  - Total USDT
    pub locked_balance: u64,        // 8 bytes  - Locked for positions
    pub available_balance: u64,     // 8 bytes  - Free to withdraw
    pub total_deposited: u64,       // 8 bytes  - Lifetime deposits
    pub total_withdrawn: u64,       // 8 bytes  - Lifetime withdrawals
    pub created_at: i64,            // 8 bytes  - Creation timestamp
    pub bump: u8,                   // 1 byte   - PDA bump
}
// Total: 8 (discriminator) + 121 = 129 bytes
```

**PDA Seeds**: `["vault", user_pubkey]`

### VaultAuthority

Global configuration for the program.

```rust
#[account]
pub struct VaultAuthority {
    pub admin: Pubkey,                      // 32 bytes
    pub authorized_programs: Vec<Pubkey>,   // 4 + (32 * 10) bytes max
    pub bump: u8,                           // 1 byte
    pub is_paused: bool,                    // 1 byte
    pub last_updated: i64,                  // 8 bytes
}
```

**PDA Seeds**: `["vault_authority"]` (singleton)

---

## Instructions Reference

### 1. initialize_vault

Creates a new vault for a user.

```typescript
await program.methods
    .initializeVault()
    .accounts({
        user: wallet.publicKey,
        vault: vaultPda,
        usdtMint: USDT_MINT,
        vaultTokenAccount: vaultTokenAccountPda,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    })
    .rpc();
```

### 2. deposit

Deposits USDT into the vault.

```typescript
const amount = new BN(100 * 1_000_000); // 100 USDT

await program.methods
    .deposit(amount)
    .accounts({
        user: wallet.publicKey,
        vault: vaultPda,
        userTokenAccount: userUsdtAccount,
        vaultTokenAccount: vaultTokenAccountPda,
        vaultAuthority: vaultAuthorityPda,
        tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();
```

### 3. withdraw

Withdraws USDT from the vault.

```typescript
const amount = new BN(50 * 1_000_000); // 50 USDT

await program.methods
    .withdraw(amount)
    .accounts({
        user: wallet.publicKey,
        vault: vaultPda,
        userTokenAccount: userUsdtAccount,
        vaultTokenAccount: vaultTokenAccountPda,
        vaultAuthority: vaultAuthorityPda,
        tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();
```

### 4. lock_collateral

Locks collateral for trading (called by authorized programs).

```typescript
await program.methods
    .lockCollateral(new BN(100 * 1_000_000))
    .accounts({
        authority: positionManagerPda,
        vault: userVaultPda,
        vaultAuthority: vaultAuthorityPda,
    })
    .rpc();
```

### 5. unlock_collateral

Unlocks previously locked collateral.

```typescript
await program.methods
    .unlockCollateral(new BN(100 * 1_000_000))
    .accounts({
        authority: positionManagerPda,
        vault: userVaultPda,
        vaultAuthority: vaultAuthorityPda,
    })
    .rpc();
```

### 6. transfer_collateral

Transfers between vaults (settlements/liquidations).

```typescript
await program.methods
    .transferCollateral(
        new BN(50 * 1_000_000),
        { settlement: {} }  // TransferReason enum
    )
    .accounts({
        authority: settlementRelayerPda,
        fromVault: loserVaultPda,
        fromTokenAccount: loserTokenAccountPda,
        toVault: winnerVaultPda,
        toTokenAccount: winnerTokenAccountPda,
        vaultAuthority: vaultAuthorityPda,
        tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();
```

---

## PDA Derivation

### Vault PDA

```typescript
const [vaultPda, vaultBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), userPublicKey.toBuffer()],
    PROGRAM_ID
);
```

### Vault Authority PDA

```typescript
const [vaultAuthorityPda, authorityBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority")],
    PROGRAM_ID
);
```

### Vault Token Account

```typescript
const vaultTokenAccount = getAssociatedTokenAddressSync(
    USDT_MINT,
    vaultPda,
    true  // allowOwnerOffCurve = true for PDAs
);
```

---

## Security Model

### 1. Access Control

| Action | Who Can Do It |
|--------|---------------|
| Create vault | Any user (for themselves) |
| Deposit | Vault owner only |
| Withdraw | Vault owner only |
| Lock/Unlock | Authorized programs only |
| Transfer | Authorized programs only |
| Add/Remove programs | Admin only |
| Pause system | Admin only |

### 2. Balance Protection

```rust
// Cannot withdraw locked funds
require!(
    vault.available_balance >= amount,
    VaultError::InsufficientBalance
);

// Cannot unlock more than locked
require!(
    vault.locked_balance >= amount,
    VaultError::InsufficientLockedBalance
);
```

### 3. Overflow Prevention

```rust
// All arithmetic uses checked operations
vault.total_balance = vault
    .total_balance
    .checked_add(amount)
    .ok_or(VaultError::Overflow)?;
```

### 4. PDA Security

- No private keys exist for vault PDAs
- Only the program can sign for PDAs
- Funds cannot be stolen even if backend is compromised

---

## Events

Events are emitted for off-chain indexing and real-time updates.

### DepositEvent
```rust
#[event]
pub struct DepositEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}
```

### WithdrawEvent
```rust
#[event]
pub struct WithdrawEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub remaining_balance: u64,
    pub timestamp: i64,
}
```

### LockCollateralEvent
```rust
#[event]
pub struct LockCollateralEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub new_locked_balance: u64,
    pub new_available_balance: u64,
    pub locked_by: Pubkey,
    pub timestamp: i64,
}
```

### Listening to Events (TypeScript)

```typescript
program.addEventListener("DepositEvent", (event, slot) => {
    console.log(`Deposit: ${event.amount} to ${event.vault}`);
    // Update database, send notification, etc.
});
```

---

## Error Handling

### Error Codes

| Code | Name | Description |
|------|------|-------------|
| 6000 | InvalidAmount | Amount must be > 0 |
| 6001 | BelowMinimumDeposit | Below minimum deposit |
| 6002 | InvalidTokenMint | Wrong token type |
| 6010 | InsufficientBalance | Not enough available |
| 6011 | HasOpenPositions | Cannot withdraw with positions |
| 6012 | InsufficientLockedBalance | Cannot unlock that much |
| 6020 | Unauthorized | Not vault owner |
| 6021 | UnauthorizedProgram | Program not in whitelist |
| 6022 | NotAdmin | Only admin can do this |
| 6030 | VaultAlreadyExists | Vault already initialized |
| 6031 | VaultNotFound | Vault doesn't exist |
| 6032 | VaultPaused | System is paused |
| 6040 | Overflow | Arithmetic overflow |
| 6041 | Underflow | Arithmetic underflow |

---

## Setup & Installation

### Prerequisites

```bash
# Rust 1.75+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"

# Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install 0.29.0
avm use 0.29.0

# Node.js (for testing)
nvm install 18
nvm use 18
```

### Build

```bash
cd collateral-vault
anchor build
```

### Test

```bash
anchor test
```

### Deploy

```bash
# Configure for devnet
solana config set --url devnet

# Get some devnet SOL
solana airdrop 2

# Deploy
anchor deploy
```

---

## Testing

### Unit Tests

Located in `programs/collateral-vault/src/` as Rust doc tests.

### Integration Tests

Located in `tests/`:

```typescript
describe("collateral-vault", () => {
    it("initializes vault", async () => {
        await program.methods.initializeVault().rpc();
        const vault = await program.account.collateralVault.fetch(vaultPda);
        expect(vault.owner.toString()).to.equal(user.publicKey.toString());
    });

    it("deposits USDT", async () => {
        const amount = new BN(100 * 1_000_000);
        await program.methods.deposit(amount).rpc();
        const vault = await program.account.collateralVault.fetch(vaultPda);
        expect(vault.totalBalance.toNumber()).to.equal(amount.toNumber());
    });

    it("prevents unauthorized withdrawal", async () => {
        try {
            await program.methods.withdraw(amount)
                .accounts({ user: attacker.publicKey })
                .signers([attacker])
                .rpc();
            expect.fail("Should have thrown");
        } catch (e) {
            expect(e.message).to.include("Unauthorized");
        }
    });
});
```

---

## Deployment

### 1. Build for Production

```bash
anchor build --verifiable
```

### 2. Deploy to Devnet

```bash
anchor deploy --provider.cluster devnet
```

### 3. Initialize VaultAuthority

```typescript
const [vaultAuthorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority")],
    PROGRAM_ID
);

await program.methods
    .initializeVaultAuthority()
    .accounts({
        admin: adminKeypair.publicKey,
        vaultAuthority: vaultAuthorityPda,
        systemProgram: SystemProgram.programId,
    })
    .signers([adminKeypair])
    .rpc();
```

### 4. Add Authorized Programs

```typescript
await program.methods
    .addAuthorizedProgram(POSITION_MANAGER_PROGRAM_ID)
    .accounts({
        admin: adminKeypair.publicKey,
        vaultAuthority: vaultAuthorityPda,
    })
    .signers([adminKeypair])
    .rpc();
```

---

## License

Confidential - GoQuant Assignment

---

## Author

Built as part of the GoQuant recruitment process.

