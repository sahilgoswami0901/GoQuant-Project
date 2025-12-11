# Collateral Vault API Commands

Complete guide for interacting with the Collateral Vault API on Solana Devnet.

## Prerequisites

1. **Backend running**: Make sure the backend is running on `http://127.0.0.1:8080`
2. **Your wallet address**: Replace `YOUR_WALLET_ADDRESS` with your actual Solana wallet public key
3. **USDT on Devnet**: You'll need test USDT tokens for deposits

---

## 📋 Available Endpoints

### 1. **Health Check**
Check if the backend is running and healthy.

```bash
curl http://127.0.0.1:8080/health
```

**Expected Response:**
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "database": true,
    "solanaRpc": true,
    "version": "0.1.0",
    "timestamp": "2025-12-08T12:00:00Z"
  }
}
```

---

### 2. **Initialize Vault**
Create a new vault for your wallet. This creates the PDA account on-chain.

#### Option A: Auto-Submit (Recommended for Devnet/Testing)

```bash
curl -X POST http://127.0.0.1:8080/vault/initialize \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS",
    "userKeypairPath": "~/.config/solana/id.json"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "transactionId": "...",
    "status": "submitted",
    "signature": "5Ht3Rjabc123...",
    "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
  }
}
```

#### Option B: Manual Sign & Submit

```bash
curl -X POST http://127.0.0.1:8080/vault/initialize \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS"
  }'
```

**Example (with auto-submit):**
```bash
curl -X POST http://127.0.0.1:8080/vault/initialize \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m",
    "userKeypairPath": "~/.config/solana/id.json"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "transactionId": "550e8400-e29b-41d4-a716-446655440000",
    "status": "pending",
    "unsignedTransaction": "base64_encoded_transaction...",
    "message": "Sign and submit this transaction with your wallet"
  }
}
```

**Next Steps:**
1. Copy the `unsignedTransaction` from the response
2. Sign it with your wallet (using Solana CLI, Phantom, or another wallet)
3. Submit the signed transaction to Solana Devnet

---

### 3. **Deposit USDT**
Deposit USDT tokens into your vault.

**⚠️ Important:** Before depositing, you must have USDT in your token account. The vault does NOT provide USDT - you need to acquire it first through:
- Buying USDT on a DEX (Jupiter, Raydium, Orca)
- Receiving USDT from another user
- Bridging USDT from another blockchain
- Using the `/vault/mint-usdt` endpoint (devnet only)

See `USDT_ACQUISITION_GUIDE.md` for detailed instructions.

**Note:** Amount is in smallest units (6 decimals). 
- 1 USDT = 1,000,000
- 100 USDT = 100,000,000

#### Option A: Auto-Submit (Recommended for Devnet/Testing)

The backend can automatically sign and submit the transaction if you provide your keypair path:

```bash
curl -X POST http://127.0.0.1:8080/vault/deposit \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS",
    "amount": 100000000,
    "userKeypairPath": "~/.config/solana/id.json"
  }'
```

**Response (Auto-Submit):**
```json
{
  "success": true,
  "data": {
    "transactionId": "...",
    "status": "submitted",
    "signature": "5Ht3Rjabc123...",
    "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
  }
}
```

#### Option B: Manual Sign & Submit

If you don't provide `userKeypairPath`, you'll receive an unsigned transaction:

```bash
curl -X POST http://127.0.0.1:8080/vault/deposit \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS",
    "amount": 100000000
  }'
```

**Example (deposit 100 USDT):**
```bash
curl -X POST http://127.0.0.1:8080/vault/deposit \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M",
    "amount": 100000000
  }'
```

**Response (Manual Sign):**
```json
{
  "success": true,
  "data": {
    "transactionId": "...",
    "status": "pending",
    "unsignedTransaction": "base64...",
    "message": "Sign and submit this transaction"
  }
}
```

**Response (Auto-Submit):**
```json
{
  "success": true,
  "data": {
    "transactionId": "...",
    "status": "submitted",
    "signature": "5Ht3Rjabc123...",
    "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
  }
}
```

---

### 4. **Lock Collateral** (Position Manager)
Locks a portion of a user's available balance so it cannot be withdrawn. Must be signed by the **position manager** (authorized program).

**⚠️ Note:** The `positionId` is a unique identifier for the trading position that requires this collateral lock. It can be any string (e.g., "pos_123abc", "position_001", etc.).

**Request Body:**
```json
{
  "userPubkey": "USER_VAULT_OWNER",
  "amount": 50000000,
  "positionId": "pos_123abc",
  "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
}
```

**Call:**
```bash
curl -X POST http://127.0.0.1:8080/vault/lock-collateral \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS",
    "amount": 50000000,
    "positionId": "pos_1",
    "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
  }'
```

**Example (lock 50 USDT for position "pos_001"):**
```bash
curl -X POST http://127.0.0.1:8080/vault/lock-collateral \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m",
    "amount": 50000000,
    "positionId": "pos_001",
    "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "transactionId": "...",
    "status": "submitted",
    "signature": "5Ht3Rjabc123...",
    "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
  }
}
```

---

### 5. **Unlock Collateral** (Position Manager)
Releases previously locked collateral back to available balance. Must be signed by the **position manager** (authorized program).

**⚠️ Note:** The `positionId` must match the position ID used when the collateral was originally locked.

**Request Body:**
```json
{
  "userPubkey": "USER_VAULT_OWNER",
  "amount": 50000000,
  "positionId": "pos_1",
  "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
}
```

**Call:**
```bash
curl -X POST http://127.0.0.1:8080/vault/unlock-collateral \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS",
    "amount": 50000000,
    "positionId": "pos_1",
    "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
  }'
```

**Example (unlock 50 USDT for position "pos_001"):**
```bash
curl -X POST http://127.0.0.1:8080/vault/unlock-collateral \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m",
    "amount": 50000000,
    "positionId": "pos_001",
    "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "transactionId": "...",
    "status": "submitted",
    "signature": "5Ht3Rjabc123...",
    "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
  }
}
```

---

### 6. **Transfer Collateral (Internal)** (Liquidation Engine / Settlement)
Moves collateral between two vaults (e.g., settlement or liquidation). Must be signed by the **liquidation engine** keypair (authorized program). `reason` helps auditing (`settlement`, `liquidation`, `fee`).

**Request Body:**
```json
{
  "fromPubkey": "SOURCE_VAULT_OWNER",
  "toPubkey": "DESTINATION_VAULT_OWNER",
  "amount": 50000000,
  "reason": "settlement",
  "liquidationEngineKeypairPath": "~/.config/solana/liquidation-engine.json"
}
```

**Call:**
```bash
curl -X POST http://127.0.0.1:8080/vault/transfer-collateral \
  -H "Content-Type: application/json" \
  -d '{
    "fromPubkey": "SOURCE_WALLET",
    "toPubkey": "DEST_WALLET",
    "amount": 50000000,
    "reason": "settlement",
    "liquidationEngineKeypairPath": "~/.config/solana/liquidation-engine.json"
  }'
```

---

### 4. **Mint Test USDT (Devnet Only)**
Mint test USDT tokens to your token account. **This endpoint only works on devnet!**

**⚠️ Important:**
- Only works on devnet/localhost
- Requires backend keypair to have mint authority for the USDT mint
- For testing purposes only
- Amount is in smallest units (6 decimals)

```bash
curl -X POST http://127.0.0.1:8080/vault/mint-usdt \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS",
    "amount": 1000000000
  }'
```

**Example (mint 1000 USDT):**
```bash
curl -X POST http://127.0.0.1:8080/vault/mint-usdt \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M",
    "amount": 1000000000
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "signature": "5Ht3Rjabc...",
    "amount": 1000000000,
    "formattedAmount": "1000.00 USDT",
    "userPubkey": "GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M",
    "message": "Successfully minted 1000.00 USDT"
  }
}
```

**Note:** This will automatically create the user's token account if it doesn't exist.

---

### 5. **Withdraw USDT**
Withdraw USDT from your vault back to your wallet.

#### Option A: Auto-Submit (Recommended for Devnet/Testing)

```bash
curl -X POST http://127.0.0.1:8080/vault/withdraw \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS",
    "amount": 50000000,
    "userKeypairPath": "~/.config/solana/id.json"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "transactionId": "...",
    "status": "submitted",
    "signature": "5Ht3Rjabc123...",
    "message": "Transaction submitted successfully. Signature: 5Ht3Rjabc123..."
  }
}
```

#### Option B: Manual Sign & Submit

```bash
curl -X POST http://127.0.0.1:8080/vault/withdraw \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_WALLET_ADDRESS",
    "amount": 50000000
  }'
```

**Example (withdraw 50 USDT with auto-submit):**
```bash
curl -X POST http://127.0.0.1:8080/vault/withdraw \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m",
    "amount": 50000000,
    "userKeypairPath": "~/.config/solana/id.json"
  }'
```

---

### 6. **Get Balance**
Check your vault balance (total, locked, and available).

```bash
curl http://127.0.0.1:8080/vault/balance/YOUR_WALLET_ADDRESS
```

**Example:**
```bash
curl http://127.0.0.1:8080/vault/balance/GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M
```

**Response:**
```json
{
  "success": true,
  "data": {
    "owner": "GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M",
    "totalBalance": 100000000,
    "lockedBalance": 0,
    "availableBalance": 100000000,
    "totalDeposited": 100000000,
    "totalWithdrawn": 0
  }
}
```

---

### 6. **Get Transaction History**
View all transactions for your vault.

```bash
curl "http://127.0.0.1:8080/vault/transactions/YOUR_WALLET_ADDRESS?limit=20&offset=0"
```

**Example:**
```bash
curl "http://127.0.0.1:8080/vault/transactions/GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M?limit=20&offset=0"
```

**Query Parameters:**
- `limit`: Number of transactions (default: 20, max: 100)
- `offset`: Skip N transactions (for pagination)
- `type`: Filter by type (deposit, withdrawal, lock, unlock, etc.)

**Example with filters:**
```bash
curl "http://127.0.0.1:8080/vault/transactions/GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M?limit=10&type=deposit"
```

---

### 7. **Get Total Value Locked (TVL)**
See the total value locked across all vaults.

```bash
curl http://127.0.0.1:8080/vault/tvl
```

**Response:**
```json
{
  "success": true,
  "data": {
    "totalValueLocked": 500000000,
    "activeVaults": 5,
    "totalLocked": 200000000,
    "totalAvailable": 300000000,
    "timestamp": "2025-12-08T12:00:00Z"
  }
}
```

---

## 🔒 Lock & Unlock (Internal APIs)

**Note:** Lock and Unlock operations are **internal APIs** meant to be called by the trading system, not directly by users. They require special authorization.

However, if you want to test them, you would need to:

1. **Add internal routes** to the backend (currently not exposed)
2. **Use admin authority** to call them

For now, these operations are handled by the smart contract directly via CPI (Cross-Program Invocation) from the trading system.

---

## 🚀 Complete Workflow Example

Here's a complete example workflow:

```bash
# 1. Check health
curl http://127.0.0.1:8080/health

# 2. Initialize vault (replace with your wallet)
curl -X POST http://127.0.0.1:8080/vault/initialize \
  -H "Content-Type: application/json" \
  -d '{"userPubkey": "GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M"}'

# 3. Deposit 100 USDT
curl -X POST http://127.0.0.1:8080/vault/deposit \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M",
    "amount": 100000000
  }'

# 4. Check balance
curl http://127.0.0.1:8080/vault/balance/GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M

# 5. Withdraw 50 USDT
curl -X POST http://127.0.0.1:8080/vault/withdraw \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M",
    "amount": 50000000
  }'

# 6. View transaction history
curl "http://127.0.0.1:8080/vault/transactions/GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M"

# 7. Check TVL
curl http://127.0.0.1:8080/vault/tvl
```

---

## 📝 Important Notes

1. **Amount Format**: All amounts are in smallest units (6 decimals for USDT)
   - 1 USDT = 1,000,000
   - 100 USDT = 100,000,000

2. **Transaction Signing**: The API returns **unsigned transactions**. You must:
   - Sign them with your wallet
   - Submit them to Solana Devnet
   - The backend doesn't have access to your private keys (security!)

3. **Wallet Address**: Replace `YOUR_WALLET_ADDRESS` with your actual Solana wallet public key

4. **Devnet**: Make sure you're using Solana Devnet and have test SOL/USDT

---

## 🔍 View on Solana Explorer

After transactions are confirmed, view them on Solana Explorer:

```
https://explorer.solana.com/address/YOUR_WALLET_ADDRESS?cluster=devnet
```

Replace `YOUR_WALLET_ADDRESS` with your wallet address.

---

## 🛠️ Troubleshooting

**Error: "Empty reply from server"**
- Backend might not be running
- Check: `curl http://127.0.0.1:8080/health`

**Error: "Vault not found"**
- Initialize the vault first using `/vault/initialize`

**Error: "Insufficient balance"**
- Check your balance: `curl http://127.0.0.1:8080/vault/balance/YOUR_WALLET_ADDRESS`
- Make sure you have enough USDT in your vault

**Transaction fails**
- Check Solana Devnet status
- Verify you have enough SOL for transaction fees
- Check transaction on Solana Explorer

