# Understanding Program ID vs Deployer Wallet

## 🔑 Key Concept

**Program ID ≠ Deployer Wallet**

- **Program ID**: The unique address of your program on-chain (from `target/deploy/collateral_vault-keypair.json`)
- **Deployer Wallet**: The wallet that pays for deployment fees (from `~/.config/solana/id.json`)

These are **two different things**!

---

## 📋 Current Setup

- **Program ID**: `cYT3s7FH9R6AViiHeB9uFd4ruwtdnyHQFQyc27oDmAS`
  - Stored in: `target/deploy/collateral_vault-keypair.json`
  - Hardcoded in: `programs/collateral-vault/src/lib.rs` (line 99)
  - Configured in: `Anchor.toml` (lines 9, 12)

- **Deployer Wallet**: `5cGSnAXuUBvm6gbAnEhUjy54m6REqrfP5Z6K99tuF6mZ`
  - Stored in: `~/.config/solana/id.json`
  - Used for: Paying deployment fees

---

## 🎯 Two Scenarios

### **Scenario 1: Keep Same Program ID, Change Deployer**

If you just want to use a different wallet to pay fees (but keep the same program):

✅ **Already done!** Your `Anchor.toml` already points to the new wallet:
```toml
[provider]
wallet = "~/.config/solana/id.json"  # Your new wallet
```

The program ID stays the same: `cYT3s7FH9R6AViiHeB9uFd4ruwtdnyHQFQyc27oDmAS`

---

### **Scenario 2: Generate New Program ID**

If you want a completely new program ID (new program deployment):

#### **Step 1: Generate New Program Keypair**

```bash
cd collateral-vault

# Generate new program keypair
solana-keygen new \
  --outfile target/deploy/collateral_vault-keypair.json \
  --no-bip39-passphrase

# Get the new program ID
solana address -k target/deploy/collateral_vault-keypair.json
```

#### **Step 2: Update declare_id! in lib.rs**

```rust
// In programs/collateral-vault/src/lib.rs
declare_id!("NEW_PROGRAM_ID_HERE");
```

#### **Step 3: Sync Anchor Configuration**

```bash
anchor keys sync
```

This automatically updates `Anchor.toml` with the new program ID.

#### **Step 4: Rebuild and Redeploy**

```bash
anchor build
anchor deploy --provider.cluster devnet
```

---

## ⚠️ Important Notes

1. **New Program = New Deployment**
   - Old program stays on-chain with old ID
   - New program is completely separate
   - All existing vaults/data are tied to the old program ID

2. **Backend Configuration**
   - If you change the program ID, update your backend `.env`:
   ```env
   VAULT_PROGRAM_ID=NEW_PROGRAM_ID_HERE
   ```

3. **Existing Data**
   - Vaults created with old program ID won't work with new program
   - You'll need to recreate everything

---

## 🔍 Check Current Configuration

```bash
# Check program ID
anchor keys list

# Check deployer wallet
solana address -k ~/.config/solana/id.json

# Check program keypair
solana address -k target/deploy/collateral_vault-keypair.json
```

---

## 💡 Recommendation

**If your program is already deployed and working:**
- ✅ Keep the same program ID
- ✅ Just use the new wallet for fees (already configured)
- ✅ No changes needed!

**Only generate a new program ID if:**
- You're starting fresh
- You want a completely new program instance
- You're okay losing access to existing vaults/data

---

## 🚀 Quick Commands

### **Keep Same Program, Just Check:**
```bash
anchor keys list
solana address -k ~/.config/solana/id.json
```

### **Generate New Program ID:**
```bash
solana-keygen new --outfile target/deploy/collateral_vault-keypair.json --no-bip39-passphrase
anchor keys sync
anchor build
```

