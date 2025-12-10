# How to Add Authorized Programs

This guide explains how to add Position Manager and Liquidation Engine to the authorized programs list.

---

## ✅ **Recommended: Add During Test Setup**

The easiest way is to add them automatically when running your devnet tests. The test file (`tests/devnet-test.ts`) has been updated to automatically add authorized programs if the keypairs exist.

### **Step 1: Create the Keypairs**

```bash
# Create Position Manager keypair
solana-keygen new \
  --outfile ~/.config/solana/position-manager.json \
  --no-bip39-passphrase

# Create Liquidation Engine keypair
solana-keygen new \
  --outfile ~/.config/solana/liquidation-engine.json \
  --no-bip39-passphrase

# Fund them (devnet)
solana airdrop 1 ~/.config/solana/position-manager.json
solana airdrop 1 ~/.config/solana/liquidation-engine.json
```

### **Step 2: Run Your Tests**

```bash
cd collateral-vault
anchor test --provider.cluster devnet --skip-deploy
```

The test will:
1. Initialize Vault Authority (if not already initialized)
2. **Automatically add Position Manager** (if keypair exists)
3. **Automatically add Liquidation Engine** (if keypair exists)
4. Display the final authorized programs list

**Output:**
```
Initializing vault authority...
✓ Vault authority initialized!

Adding authorized programs...
✓ Found Position Manager: 7xKt9Fj2abc123...
✓ Found Liquidation Engine: 9Yht3Mkxyz789...
✓ Added Position Manager as authorized program
  Transaction: 5Ht3Rjabc123...
✓ Added Liquidation Engine as authorized program
  Transaction: 5Ht3Rjdef456...

📋 Authorized Programs List:
  1. 7xKt9Fj2abc123...  (Position Manager)
  2. 9Yht3Mkxyz789...  (Liquidation Engine)
```

---

## 🔧 **Alternative: Manual Addition via API**

If you prefer to add them manually or after the fact, you can use the API endpoint:

### **Endpoint:**
```
POST /vault/add-authorized-program
```

### **Request:**
```bash
curl -X POST http://127.0.0.1:8080/vault/add-authorized-program \
  -H "Content-Type: application/json" \
  -d '{
    "programId": "POSITION_MANAGER_PUBKEY",
    "adminKeypairPath": "~/.config/solana/id.json"
  }'
```

**Note:** The `adminKeypairPath` must point to the keypair that initialized VaultAuthority (usually your deployer wallet).

---

## 📋 **Step-by-Step Process**

### **1. Find Your Admin Address**

The admin is whoever called `initialize_vault_authority`. Usually this is your deployer wallet:

```bash
# Check your current wallet (likely the admin)
solana address

# Or check the deployer wallet
solana address -k ~/.config/solana/id.json
```

### **2. Get Position Manager & Liquidation Engine Addresses**

```bash
# Position Manager
solana address -k ~/.config/solana/position-manager.json

# Liquidation Engine
solana address -k ~/.config/solana/liquidation-engine.json
```

### **3. Add Them via Test (Recommended)**

Just run your tests - they'll be added automatically if the keypairs exist!

```bash
anchor test --provider.cluster devnet --skip-deploy
```

### **4. Verify They're Added**

Check the test output or query the VaultAuthority account:

```typescript
const authority = await program.account.vaultAuthority.fetch(vaultAuthorityPda);
console.log("Authorized Programs:", authority.authorizedPrograms.map(p => p.toBase58()));
```

---

## 🎯 **What Happens**

When you run the test:

1. **Test checks if keypairs exist** → If yes, loads them
2. **Test checks if already authorized** → Skips if already added
3. **Test calls `add_authorized_program`** → Adds to the list
4. **Test verifies** → Confirms they're in the authorized list

---

## ⚠️ **Important Notes**

1. **Admin Only**: Only the admin (who initialized VaultAuthority) can add programs
2. **Keypair Location**: Keypairs must be at:
   - `~/.config/solana/position-manager.json`
   - `~/.config/solana/liquidation-engine.json`
3. **One-Time Setup**: Once added, they stay authorized (unless removed)
4. **Maximum 10**: The VaultAuthority can hold up to 10 authorized programs

---

## 🚀 **Quick Start**

```bash
# 1. Create keypairs
solana-keygen new --outfile ~/.config/solana/position-manager.json --no-bip39-passphrase
solana-keygen new --outfile ~/.config/solana/liquidation-engine.json --no-bip39-passphrase

# 2. Fund them
solana airdrop 1 ~/.config/solana/position-manager.json
solana airdrop 1 ~/.config/solana/liquidation-engine.json

# 3. Run tests (they'll be added automatically)
anchor test --provider.cluster devnet --skip-deploy
```

That's it! The authorized programs will be added automatically when you run your tests. 🎉

