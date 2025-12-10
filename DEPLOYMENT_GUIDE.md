# Collateral Vault - Local Deployment & Testing Guide

This guide walks you through deploying and testing the complete Collateral Vault system locally.

---

## 📋 Table of Contents

1. [Prerequisites](#prerequisites)
2. [Step 1: Environment Setup](#step-1-environment-setup)
3. [Step 2: Deploy Smart Contract](#step-2-deploy-smart-contract)
4. [Step 3: Setup Database](#step-3-setup-database)
5. [Step 4: Run Backend Service](#step-4-run-backend-service)
6. [Step 5: Test the System](#step-5-test-the-system)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Software

| Software | Version | Check Command |
|----------|---------|---------------|
| Rust | 1.75+ | `rustc --version` |
| Solana CLI | 1.17+ | `solana --version` |
| Anchor | 0.29+ | `anchor --version` |
| Node.js | 18+ | `node --version` |
| Yarn | 1.22+ | `yarn --version` |
| PostgreSQL | 14+ | `psql --version` |

### Installation Commands (macOS)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install Solana
sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Install Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install 0.29.0
avm use 0.29.0

# Install Node.js (using nvm)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18

# Install Yarn
npm install -g yarn

# Install PostgreSQL (macOS)
brew install postgresql@14
brew services start postgresql@14
```

---

## Step 1: Environment Setup

### 1.1 Create Solana Keypair (if you don't have one)

```bash
# Generate a new keypair
solana-keygen new --outfile ~/.config/solana/id.json

# Or if you already have one, check it
solana address
```

### 1.2 Configure Solana for Local Development

```bash
# Set to localhost (local validator)
solana config set --url localhost

# Verify configuration
solana config get
```

### 1.3 Start Local Solana Validator

Open a **new terminal** and run:

```bash
# Start the local validator (keep this running)
solana-test-validator

# You should see output like:
# Ledger location: test-ledger
# Log: test-ledger/validator.log
# ⠤ Initializing...
# Identity: <your-validator-identity>
# Genesis Hash: <hash>
# Version: 1.17.x
# ...
```

**Keep this terminal open!** The validator needs to run while you test.

### 1.4 Airdrop SOL to Your Wallet

In another terminal:

```bash
# Airdrop 10 SOL for testing
solana airdrop 10

# Check balance
solana balance

# Expected output: 10 SOL
```

---

## Step 2: Deploy Smart Contract

### 2.1 Navigate to Project

```bash
cd "/Users/sahilgoswami/Desktop/GoQuant Project/collateral-vault"
```

### 2.2 Install Dependencies

```bash
# Install Node dependencies for tests
yarn install

# Or if you prefer npm
npm install
```

### 2.3 Build the Anchor Program

```bash
# Build the program
anchor build

# This creates:
# - target/deploy/collateral_vault.so (the program binary)
# - target/idl/collateral_vault.json (the interface definition)
# - target/types/collateral_vault.ts (TypeScript types)
```

### 2.4 Get the Program ID

```bash
# Get the program ID from the built keypair
solana address -k target/deploy/collateral_vault-keypair.json

# Example output: AVRBwuFHdU51wxP3a8brB95KL1VT7PCrFAVt1zzmjde
```

### 2.5 Update Program ID (if different)

If your program ID is different, update it in:

1. `programs/collateral-vault/src/lib.rs`:
```rust
declare_id!("YOUR_PROGRAM_ID_HERE");
```

2. `Anchor.toml`:
```toml
[programs.localnet]
collateral_vault = "YOUR_PROGRAM_ID_HERE"
```

Then rebuild:
```bash
anchor build
```

### 2.6 Deploy the Program

```bash
# Deploy to local validator
anchor deploy

# Expected output:
# Deploying cluster: http://localhost:8899
# Upgrade authority: <your-keypair>
# Deploying program "collateral_vault"...
# Program Id: AVRBwuFHdU51wxP3a8brB95KL1VT7PCrFAVt1zzmjde
# Deploy success
```

### 2.7 Verify Deployment

```bash
# Check if program exists
solana program show <YOUR_PROGRAM_ID>

# Expected output shows program details
```

---

## Step 3: Setup Database

### 3.1 Create PostgreSQL Database

```bash
# Create the database
createdb collateral_vault

# Verify it exists
psql -l | grep collateral_vault
```

### 3.2 Create Environment File

```bash
cd backend

# Create .env file
cat > .env << 'EOF'
# Database
DATABASE_URL=postgres://$(whoami):@localhost:5432/collateral_vault

# Solana (local validator)
SOLANA_RPC_URL=http://127.0.0.1:8899
SOLANA_WS_URL=ws://127.0.0.1:8900
VAULT_PROGRAM_ID=AVRBwuFHdU51wxP3a8brB95KL1VT7PCrFAVt1zzmjde
USDT_MINT=So11111111111111111111111111111111111111112

# Server
SERVER_HOST=127.0.0.1
SERVER_PORT=8080

# Monitoring
BALANCE_CHECK_INTERVAL=30
RECONCILIATION_INTERVAL=300
LOW_BALANCE_THRESHOLD=100

# Logging
RUST_LOG=info,collateral_vault_backend=debug
EOF
```

**Note:** Replace `VAULT_PROGRAM_ID` with your actual program ID from Step 2.6.

### 3.3 Install SQLx CLI

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

### 3.4 Run Database Migrations

```bash
cd backend
sqlx migrate run

# Expected output:
# Applied 001_initial_schema (xxx ms)
```

### 3.5 Verify Database Tables

```bash
psql collateral_vault -c "\dt"

# Expected output:
#              List of relations
#  Schema |        Name         | Type  | Owner
# --------+---------------------+-------+-------
#  public | _sqlx_migrations    | table | ...
#  public | alerts              | table | ...
#  public | balance_snapshots   | table | ...
#  public | reconciliation_logs | table | ...
#  public | transactions        | table | ...
#  public | tvl_snapshots       | table | ...
#  public | vaults              | table | ...
```

---

## Step 4: Run Backend Service

### 4.1 Build the Backend

```bash
cd backend
cargo build
```

### 4.2 Start the Backend Server

```bash
cargo run

# Expected output:
# 🚀 Starting Collateral Vault Backend Service
# 📋 Configuration loaded
#    Solana RPC: http://127.0.0.1:8899
#    Program ID: AVRBwuFHdU51wxP3a8brB95KL1VT7PCrFAVt1zzmjde
# 🗄️  Database connected
# 📦 Database migrations complete
# ⛓️  Solana client initialized
# 🔧 Services initialized
# 👁️  Vault monitor started
# 📊 Balance tracker started
# 🌐 Starting HTTP server on 127.0.0.1:8080
```

**Keep this terminal open!**

---

## Step 5: Test the System

### 5.1 Test Health Endpoint

```bash
curl http://localhost:8080/health | jq

# Expected output:
# {
#   "success": true,
#   "data": {
#     "status": "healthy",
#     "database": true,
#     "solanaRpc": true,
#     "version": "0.1.0"
#   }
# }
```

### 5.2 Run Anchor Tests

In the project root directory:

```bash
cd "/Users/sahilgoswami/Desktop/GoQuant Project/collateral-vault"

# Run all tests
anchor test --skip-local-validator

# Note: We skip local validator since it's already running
```

### 5.3 Manual API Testing

#### Test Initialize Vault

```bash
# Get your wallet address
WALLET=$(solana address)
echo "Wallet: $WALLET"

# Initialize vault (this returns an unsigned transaction)
curl -X POST http://localhost:8080/vault/initialize \
  -H "Content-Type: application/json" \
  -d "{\"userPubkey\": \"$WALLET\"}" | jq
```

#### Test Get Balance

```bash
curl http://localhost:8080/vault/balance/$WALLET | jq
```

#### Test Get TVL

```bash
curl http://localhost:8080/vault/tvl | jq
```

### 5.4 Test with TypeScript Client

Create a test script:

```bash
cd "/Users/sahilgoswami/Desktop/GoQuant Project/collateral-vault"

cat > test-client.ts << 'EOF'
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { 
    PublicKey, 
    Keypair, 
    SystemProgram,
    LAMPORTS_PER_SOL 
} from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createMint,
    getAssociatedTokenAddressSync,
    getOrCreateAssociatedTokenAccount,
    mintTo,
} from "@solana/spl-token";

async function main() {
    // Connect to local validator
    const connection = new anchor.web3.Connection("http://127.0.0.1:8899", "confirmed");
    
    // Load wallet
    const wallet = anchor.Wallet.local();
    console.log("Wallet:", wallet.publicKey.toString());
    
    // Check balance
    const balance = await connection.getBalance(wallet.publicKey);
    console.log("SOL Balance:", balance / LAMPORTS_PER_SOL);
    
    // Load program
    const provider = new anchor.AnchorProvider(connection, wallet, {});
    anchor.setProvider(provider);
    
    const programId = new PublicKey("AVRBwuFHdU51wxP3a8brB95KL1VT7PCrFAVt1zzmjde");
    console.log("Program ID:", programId.toString());
    
    // Derive vault PDA
    const [vaultPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), wallet.publicKey.toBuffer()],
        programId
    );
    console.log("Vault PDA:", vaultPda.toString());
    
    // Check if vault exists
    const vaultAccount = await connection.getAccountInfo(vaultPda);
    if (vaultAccount) {
        console.log("Vault exists! Data length:", vaultAccount.data.length);
    } else {
        console.log("Vault does not exist yet");
    }
    
    console.log("\n✅ Connection test successful!");
}

main().catch(console.error);
EOF

# Run the test
npx ts-node test-client.ts
```

### 5.5 Full Integration Test

Run the complete test suite:

```bash
# In the project root
anchor test --skip-local-validator

# This runs all tests in tests/collateral-vault.ts
# - Initialize vault authority
# - Initialize user vaults
# - Deposit USDT
# - Withdraw USDT
# - Lock/Unlock collateral
# - Transfer between vaults
```

---

## 📊 Testing Checklist

Use this checklist to verify all functionalities:

### Smart Contract (Part 1)
- [ ] Program deploys successfully
- [ ] `initialize_vault_authority` works
- [ ] `initialize_vault` creates user vault
- [ ] `deposit` transfers tokens to vault
- [ ] `withdraw` transfers tokens to user
- [ ] `lock_collateral` locks funds
- [ ] `unlock_collateral` releases funds
- [ ] `transfer_collateral` moves between vaults

### Backend Service (Part 2)
- [ ] Server starts without errors
- [ ] Health endpoint returns healthy
- [ ] Database connection works
- [ ] Solana RPC connection works

### Database (Part 3)
- [ ] Migrations run successfully
- [ ] All tables created
- [ ] Queries work correctly

### API (Part 4)
- [ ] `POST /vault/initialize` works
- [ ] `POST /vault/deposit` works
- [ ] `POST /vault/withdraw` works
- [ ] `GET /vault/balance/:user` works
- [ ] `GET /vault/transactions/:user` works
- [ ] `GET /vault/tvl` works
- [ ] WebSocket connections work

---

## Troubleshooting

### Common Issues

#### 1. "Program not found" Error

```bash
# Check if program is deployed
solana program show <PROGRAM_ID>

# If not found, redeploy
anchor deploy
```

#### 2. "Insufficient funds" Error

```bash
# Airdrop more SOL
solana airdrop 10
```

#### 3. Database Connection Error

```bash
# Check PostgreSQL is running
brew services list | grep postgresql

# Start if not running
brew services start postgresql@14

# Check database exists
psql -l | grep collateral_vault
```

#### 4. Port Already in Use

```bash
# Find process using port 8080
lsof -i :8080

# Kill if needed
kill -9 <PID>
```

#### 5. Validator Not Running

```bash
# Check if running
pgrep solana-test-validator

# Start if not
solana-test-validator
```

### Logs

#### Smart Contract Logs
```bash
# View validator logs
tail -f test-ledger/validator.log
```

#### Backend Logs
The backend outputs logs to stdout. Adjust verbosity with:
```bash
RUST_LOG=debug cargo run
```

---

## 🎉 Success!

If all tests pass, you have successfully deployed:

1. ✅ **Solana Smart Contract** - Running on local validator
2. ✅ **PostgreSQL Database** - Storing transaction history
3. ✅ **Rust Backend** - Serving API at http://localhost:8080
4. ✅ **All APIs** - Ready for frontend integration

### Next Steps

1. **Create a Frontend** - Build a React/Vue app to interact with the API
2. **Deploy to Devnet** - Test on Solana's public devnet
3. **Add More Tests** - Increase test coverage
4. **Record Demo Video** - For the assignment submission


