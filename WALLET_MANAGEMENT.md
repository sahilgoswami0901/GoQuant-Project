# 🔐 Solana Wallet Management Guide

Complete guide to managing Solana wallets and keypairs for your Collateral Vault project.

---

## 📚 Understanding Solana Wallets

### Key Concepts

1. **Wallet = Keypair**: A Solana wallet is just a keypair (private key + public key)
2. **One Keypair = One Address**: Each keypair has a unique wallet address (public key)
3. **Multiple Wallets**: You can have **unlimited** keypairs - each is stored as a separate JSON file
4. **Default Wallet**: Solana CLI uses `~/.config/solana/id.json` as the default

### Important Notes

- ✅ **You CAN have multiple wallets** - each keypair file is independent
- ✅ **Each wallet has a unique address** - derived from its private key
- ✅ **You can switch between wallets** - by changing which keypair file you use
- ❌ **One address ≠ One wallet** - Actually, one keypair = one address, but you can have many keypairs

---

## 🔄 Current Situation

You've deployed a smart contract on devnet using your current wallet. Now you want to:
- Clear/backup the current wallet
- Create a new wallet for a fresh deployment

---

## 📋 Step-by-Step Guide

### Option 1: Backup Current Wallet & Create New Default

This keeps your old wallet safe and creates a new default wallet.

```bash
# 1. Check your current wallet address
solana address

# 2. Backup your current wallet (IMPORTANT!)
cp ~/.config/solana/id.json ~/.config/solana/id.json.backup

# 3. Check your current balance (in case you want to transfer funds)
solana balance

# 4. Create a NEW default wallet
solana-keygen new --outfile ~/.config/solana/id.json --force

# 5. Verify the new wallet address
solana address

# 6. Get devnet SOL for the new wallet
solana airdrop 2
solana balance
```

**Note**: The `--force` flag will overwrite your existing `id.json`. Make sure you backed it up first!

---

### Option 2: Keep Current Wallet & Create Additional Wallet

This keeps your current wallet as default and creates a separate wallet for new deployments.

```bash
# 1. Check your current wallet
solana address
echo "Current wallet: $(solana address)"

# 2. Create a NEW wallet with a custom name
solana-keygen new --outfile ~/.config/solana/new-deployment-keypair.json

# 3. Get the address of the new wallet
solana address -k ~/.config/solana/new-deployment-keypair.json

# 4. Airdrop SOL to the new wallet
solana airdrop 2 -k ~/.config/solana/new-deployment-keypair.json

# 5. Check balance
solana balance -k ~/.config/solana/new-deployment-keypair.json
```

**To use this new wallet for deployment:**

```bash
# Option A: Use it for a specific command
anchor deploy --provider.cluster devnet --provider.wallet ~/.config/solana/new-deployment-keypair.json

# Option B: Temporarily set it as default
cp ~/.config/solana/id.json ~/.config/solana/id.json.old
cp ~/.config/solana/new-deployment-keypair.json ~/.config/solana/id.json
anchor deploy
# Then restore: cp ~/.config/solana/id.json.old ~/.config/solana/id.json
```

---

### Option 3: List All Your Wallets

```bash
# List all keypair files in your Solana config directory
ls -la ~/.config/solana/*.json

# Check addresses of specific keypairs
solana address -k ~/.config/solana/id.json
solana address -k ~/.config/solana/new-deployment-keypair.json
```

---

## 🎯 Recommended Approach for Your Use Case

Since you want to start fresh with a new deployment:

### Step 1: Backup Current Wallet

```bash
# Create backup directory
mkdir -p ~/.config/solana/backups

# Backup with timestamp
cp ~/.config/solana/id.json ~/.config/solana/backups/id-$(date +%Y%m%d-%H%M%S).json

# Also save the address for reference
echo "Old wallet address: $(solana address)" > ~/.config/solana/backups/old-wallet-address.txt
cat ~/.config/solana/backups/old-wallet-address.txt
```

### Step 2: Create New Wallet

```bash
# Generate new default wallet
solana-keygen new --outfile ~/.config/solana/id.json --force

# Verify new address
NEW_ADDRESS=$(solana address)
echo "New wallet address: $NEW_ADDRESS"
```

### Step 3: Get Devnet SOL

```bash
# Switch to devnet (if not already)
solana config set --url devnet

# Airdrop SOL (you may need to do this multiple times)
solana airdrop 2
sleep 5
solana airdrop 2
sleep 5
solana airdrop 2

# Check balance (need at least 4-5 SOL for deployment)
solana balance
```

### Step 4: Update Your Configuration

If you're using the new wallet for deployment, update your `Anchor.toml`:

```toml
[provider]
cluster = "devnet"
wallet = "~/.config/solana/id.json"  # This now points to your new wallet
```

### Step 5: Deploy with New Wallet

```bash
# Build the program
anchor build

# Deploy to devnet
anchor deploy

# Get your new program ID
solana address -k target/deploy/collateral_vault-keypair.json
```

### Step 6: Update Backend Configuration

Update your `backend/.env` file with the new program ID:

```env
VAULT_PROGRAM_ID=YOUR_NEW_PROGRAM_ID_HERE
KEYPAIR_PATH=~/.config/solana/id.json
```

---

## 🔍 Verify Your Setup

```bash
# Check current wallet
solana address

# Check Solana config
solana config get

# Check balance
solana balance

# List all your keypairs
ls -la ~/.config/solana/*.json
```

---

## 💡 Pro Tips

### 1. Use Named Keypairs for Different Projects

```bash
# Create project-specific keypairs
solana-keygen new --outfile ~/.config/solana/collateral-vault-devnet.json
solana-keygen new --outfile ~/.config/solana/collateral-vault-mainnet.json
solana-keygen new --outfile ~/.config/solana/other-project.json
```

### 2. Transfer Funds Between Wallets

If you want to transfer SOL from old wallet to new:

```bash
# First, temporarily use old wallet
OLD_WALLET=~/.config/solana/backups/id-YYYYMMDD-HHMMSS.json
NEW_ADDRESS=$(solana address -k ~/.config/solana/id.json)

# Transfer SOL (using old wallet)
solana transfer $NEW_ADDRESS 1 --from $OLD_WALLET
```

### 3. Use Environment Variables

```bash
# Set wallet for current session
export SOLANA_WALLET=~/.config/solana/new-deployment-keypair.json

# Or use in commands
anchor deploy --provider.wallet $SOLANA_WALLET
```

---

## ⚠️ Important Security Notes

1. **Never share your private key** - The JSON file contains your private key
2. **Backup important wallets** - Always backup before overwriting
3. **Use different wallets for different networks** - Devnet vs Mainnet
4. **Don't commit keypairs to git** - Add `*.json` to `.gitignore` (except program keypairs in `target/deploy/`)

---

## 🆘 Troubleshooting

### "Insufficient funds" error

```bash
# Airdrop more SOL
solana airdrop 2
# Wait a few seconds between airdrops
```

### "Account not found" error

```bash
# Make sure you're on the right network
solana config get

# Switch to devnet if needed
solana config set --url devnet
```

### Want to restore old wallet?

```bash
# Restore from backup
cp ~/.config/solana/backups/id-YYYYMMDD-HHMMSS.json ~/.config/solana/id.json

# Verify
solana address
```

---

## 📝 Summary

- ✅ **You CAN have multiple wallets** - each is a separate keypair file
- ✅ **One keypair = one wallet address** - but you can have unlimited keypairs
- ✅ **Backup before overwriting** - always save your old wallet
- ✅ **Use named keypairs** - for better organization
- ✅ **Update configs** - after creating new wallet, update your deployment configs

Your old deployment is still on-chain with the old wallet address. The new wallet will create a completely separate deployment with a new program instance.

---

## 🔗 Related Files

- `backend/.env` - Backend configuration (update `KEYPAIR_PATH` and `VAULT_PROGRAM_ID`)
- `Anchor.toml` - Anchor configuration (update `wallet` path)
- `~/.config/solana/id.json` - Default Solana wallet
- `~/.config/solana/cli/config.yml` - Solana CLI configuration

---

Need help? Check:
- Solana CLI docs: `solana-keygen --help`
- Your deployment guide: `DEPLOYMENT_GUIDE.md`
- Backend setup: `BACKEND_SETUP.md`
