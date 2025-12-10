# Fix: Program Authority Mismatch

## 🔴 Error

```
Error: Program's authority Some(HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m) 
does not match authority provided 5cGSnAXuUBvm6gbAnEhUjy54m6REqrfP5Z6K99tuF6mZ
```

## 📋 What This Means

- **Original Upgrade Authority**: `HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m` (wallet2.json)
- **Current Deployer**: `5cGSnAXuUBvm6gbAnEhUjy54m6REqrfP5Z6K99tuF6mZ` (id.json)
- **Problem**: You can't upgrade a program unless you're the upgrade authority

---

## ✅ Solutions

### **Solution 1: Use Original Wallet for Deployment (Easiest)**

If you have access to `wallet2.json`, use it for deployments:

```bash
# Option A: Temporarily set wallet2.json as default
solana config set --keypair ~/.config/solana/wallet2.json

# Then deploy
anchor deploy --provider.cluster devnet

# Option B: Specify wallet in Anchor.toml
# Edit Anchor.toml:
[provider]
wallet = "~/.config/solana/wallet2.json"  # Use original wallet

# Then deploy
anchor deploy --provider.cluster devnet
```

### **Solution 2: Transfer Upgrade Authority**

If you have access to the original wallet, transfer authority to the new wallet:

```bash
# Transfer upgrade authority from old wallet to new wallet
solana program set-upgrade-authority cYT3s7FH9R6AViiHeB9uFd4ruwtdnyHQFQyc27oDmAS \
  --new-upgrade-authority 5cGSnAXuUBvm6gbAnEhUjy54m6REqrfP5Z6K99tuF6mZ \
  --keypair ~/.config/solana/wallet2.json \
  --url devnet

# Then you can deploy with the new wallet
anchor deploy --provider.cluster devnet
```

### **Solution 3: Deploy New Program (Fresh Start)**

If you want to start completely fresh with the new wallet:

```bash
# 1. Generate new program keypair
solana-keygen new \
  --outfile target/deploy/collateral_vault-keypair.json \
  --no-bip39-passphrase

# 2. Get new program ID
NEW_PROGRAM_ID=$(solana address -k target/deploy/collateral_vault-keypair.json)
echo "New Program ID: $NEW_PROGRAM_ID"

# 3. Update lib.rs
# Edit programs/collateral-vault/src/lib.rs:
# declare_id!("$NEW_PROGRAM_ID");

# 4. Sync Anchor config
anchor keys sync

# 5. Build and deploy
anchor build
anchor deploy --provider.cluster devnet

# 6. Update backend .env
# VAULT_PROGRAM_ID=$NEW_PROGRAM_ID
```

---

## 🎯 Recommended Approach

**If you have access to `wallet2.json`:**

1. **Quick fix**: Use wallet2.json for deployments
   ```bash
   # Update Anchor.toml
   [provider]
   wallet = "~/.config/solana/wallet2.json"
   ```

2. **Better long-term**: Transfer authority to new wallet
   ```bash
   solana program set-upgrade-authority cYT3s7FH9R6AViiHeB9uFd4ruwtdnyHQFQyc27oDmAS \
     --new-upgrade-authority 5cGSnAXuUBvm6gbAnEhUjy54m6REqrfP5Z6K99tuF6mZ \
     --keypair ~/.config/solana/wallet2.json \
     --url devnet
   ```

**If you DON'T have access to `wallet2.json`:**

- You'll need to deploy a new program (Solution 3)
- This means losing access to existing vaults/data tied to the old program

---

## 🔍 Check Current Authority

```bash
# Check who owns the program
solana program show cYT3s7FH9R6AViiHeB9uFd4ruwtdnyHQFQyc27oDmAS --url devnet
```

---

## 💡 Quick Fix Command

**Use the original wallet for this deployment:**

```bash
# Temporarily use wallet2.json
anchor deploy --provider.cluster devnet --provider.wallet ~/.config/solana/wallet2.json
```

Or update `Anchor.toml`:
```toml
[provider]
wallet = "~/.config/solana/wallet2.json"  # Use original wallet
```

