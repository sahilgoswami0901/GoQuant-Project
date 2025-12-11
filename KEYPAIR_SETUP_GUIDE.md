# Keypair Setup Guide for Position Manager & Liquidation Engine

This guide explains how to create keypairs for the **Position Manager** and **Liquidation Engine** that will automatically sign lock/unlock/transfer transactions.

---

## 🎯 Overview

You need **two separate keypairs**:

1. **Position Manager Keypair** - Signs lock/unlock transactions
2. **Liquidation Engine Keypair** - Signs transfer transactions

These keypairs represent the authorized programs that can perform these operations.

---

## 📝 Step 1: Create Position Manager Keypair

```bash
# Create a new keypair for position manager
solana-keygen new \
  --outfile ~/.config/solana/position-manager.json \
  --no-bip39-passphrase
```

**Output:**
```
Generating a new keypair

For added security, enter a BIP39 passphrase

⚠️  WARNING: Do not use this keypair for mainnet funds!

pubkey: 7xKt9Fj2abc123...  <-- This is your Position Manager address
```

**Save the pubkey** - you'll need to add it to the authorized programs list later.

---

## 📝 Step 2: Create Liquidation Engine Keypair

```bash
# Create a new keypair for liquidation engine
solana-keygen new \
  --outfile ~/.config/solana/liquidation-engine.json \
  --no-bip39-passphrase
```

**Output:**
```
Generating a new keypair

For added security, enter a BIP39 passphrase

⚠️  WARNING: Do not use this keypair for mainnet funds!

pubkey: 9Yht3Mkxyz789...  <-- This is your Liquidation Engine address
```

**Save the pubkey** - you'll need to add it to the authorized programs list later.

---

## 🔐 Step 3: Fund the Keypairs (Devnet)

These keypairs need SOL to pay for transaction fees:

```bash
# Fund position manager
solana airdrop 1 ~/.config/solana/position-manager.json

# Fund liquidation engine
solana airdrop 1 ~/.config/solana/liquidation-engine.json
```

**Verify balances:**
```bash
solana balance ~/.config/solana/position-manager.json
solana balance ~/.config/solana/liquidation-engine.json
```

---

## ✅ Step 4: Verify Keypairs Exist

```bash
# Check position manager keypair
ls -la ~/.config/solana/position-manager.json

# Check liquidation engine keypair
ls -la ~/.config/solana/liquidation-engine.json

# View their public keys
solana address -k ~/.config/solana/position-manager.json
solana address -k ~/.config/solana/liquidation-engine.json
```

---

## 🚀 Step 5: Use the Keypairs in API Calls

### **Lock Collateral (Position Manager)**

```bash
curl -X POST http://127.0.0.1:8080/vault/lock-collateral \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m",
    "amount": 10000000,
    "positionId": "pos_123",
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

### **Unlock Collateral (Position Manager)**

```bash
curl -X POST http://127.0.0.1:8080/vault/unlock-collateral \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m",
    "amount": 10000000,
    "positionId": "pos_123",
    "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
  }'
```

---

### **Transfer Collateral (Liquidation Engine)**

```bash
curl -X POST http://127.0.0.1:8080/vault/transfer-collateral \
  -H "Content-Type: application/json" \
  -d '{
    "fromPubkey": "HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m",
    "toPubkey": "GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M",
    "amount": 50000000,
    "reason": "settlement",
    "liquidationEngineKeypairPath": "~/.config/solana/liquidation-engine.json"
  }'
```

---

## 🔑 Step 6: Add to Authorized Programs (Important!)

**Before these keypairs can actually work on-chain**, you need to add them to the `VaultAuthority`'s authorized programs list.

### **Get the Public Keys:**

```bash
# Position Manager pubkey
POSITION_MANAGER_PUBKEY=$(solana address -k ~/.config/solana/position-manager.json)
echo "Position Manager: $POSITION_MANAGER_PUBKEY"

# Liquidation Engine pubkey
LIQUIDATION_ENGINE_PUBKEY=$(solana address -k ~/.config/solana/liquidation-engine.json)
echo "Liquidation Engine: $LIQUIDATION_ENGINE_PUBKEY"
```

### **Add to Authorized Programs:**

You'll need to call the `add_authorized_program` instruction on your smart contract. This is typically done via:

1. **Anchor CLI** (if you have an admin script)
2. **TypeScript/JavaScript** (using Anchor client)
3. **Direct Solana CLI** (building the instruction manually)

**Example TypeScript:**
```typescript
// Add position manager to authorized programs
await program.methods
  .addAuthorizedProgram(new PublicKey(POSITION_MANAGER_PUBKEY))
  .accounts({
    admin: adminKeypair.publicKey,
    vaultAuthority: vaultAuthorityPda,
  })
  .rpc();

// Add liquidation engine to authorized programs
await program.methods
  .addAuthorizedProgram(new PublicKey(LIQUIDATION_ENGINE_PUBKEY))
  .accounts({
    admin: adminKeypair.publicKey,
    vaultAuthority: vaultAuthorityPda,
  })
  .rpc();
```

---

## 📁 File Locations

After creating the keypairs, you'll have:

```
~/.config/solana/
├── id.json                    # Your main wallet (user operations)
├── position-manager.json      # Position manager keypair (lock/unlock)
└── liquidation-engine.json    # Liquidation engine keypair (transfers)
```

---

## 🔒 Security Notes

### **For Development/Testing:**
- ✅ It's fine to use these keypairs on devnet
- ✅ Store them in `~/.config/solana/` for easy access
- ✅ Use `--no-bip39-passphrase` for automated signing

### **For Production:**
- ⚠️ **NEVER** commit keypair files to git
- ⚠️ Use secure key management (HSM, hardware wallets, or secure vaults)
- ⚠️ Consider using PDAs (Program Derived Addresses) instead of keypairs
- ⚠️ Implement proper access controls and rate limiting

---

## 🧪 Quick Test

After creating the keypairs, test them:

```bash
# Test position manager can sign
solana balance -k ~/.config/solana/position-manager.json

# Test liquidation engine can sign
solana balance -k ~/.config/solana/liquidation-engine.json
```

---

## 📋 Summary

1. ✅ Create position manager keypair: `~/.config/solana/position-manager.json`
2. ✅ Create liquidation engine keypair: `~/.config/solana/liquidation-engine.json`
3. ✅ Fund both keypairs with SOL (for transaction fees)
4. ✅ Add their pubkeys to `VaultAuthority.authorized_programs` on-chain
5. ✅ Use them in API calls with `positionManagerKeypairPath` and `liquidationEngineKeypairPath`

---

## 🎯 Complete Example

```bash
# 1. Create keypairs
solana-keygen new --outfile ~/.config/solana/position-manager.json --no-bip39-passphrase
solana-keygen new --outfile ~/.config/solana/liquidation-engine.json --no-bip39-passphrase

# 2. Fund them
solana airdrop 1 ~/.config/solana/position-manager.json
solana airdrop 1 ~/.config/solana/liquidation-engine.json

# 3. Get their addresses
echo "Position Manager: $(solana address -k ~/.config/solana/position-manager.json)"
echo "Liquidation Engine: $(solana address -k ~/.config/solana/liquidation-engine.json)"

# 4. Use in API calls
curl -X POST http://127.0.0.1:8080/vault/lock-collateral \
  -H "Content-Type: application/json" \
  -d '{
    "userPubkey": "YOUR_USER_ADDRESS",
    "amount": 10000000,
    "positionId": "pos_123",
    "positionManagerKeypairPath": "~/.config/solana/position-manager.json"
  }'
```

That's it! Your keypairs are ready to use. 🚀


