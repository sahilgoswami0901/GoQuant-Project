# SOL Transfer Guide

Quick guide for transferring SOL between Solana wallets.

---

## 🚀 Quick Transfer

### **Method 1: Using Solana CLI (Recommended)**

```bash
# Transfer SOL from default wallet to another address
solana transfer <RECIPIENT_ADDRESS> <AMOUNT> --allow-unfunded-recipient

# Example: Transfer 1 SOL
solana transfer 7xKt9Fj2abc123... 1

# Transfer from a specific keypair
solana transfer <RECIPIENT_ADDRESS> <AMOUNT> \
  --from ~/.config/solana/id.json \
  --allow-unfunded-recipient

# Example: Transfer 0.5 SOL from position-manager to liquidation-engine
solana transfer $(solana address -k ~/.config/solana/liquidation-engine.json) 0.5 \
  --from ~/.config/solana/position-manager.json
```

---

## 📋 Common Use Cases

### **1. Fund a New Wallet**

```bash
# Get the address you want to fund
solana address -k ~/.config/solana/new-wallet.json

# Transfer SOL to it (from your default wallet)
solana transfer <NEW_WALLET_ADDRESS> 1
```

### **2. Transfer Between Your Wallets**

```bash
# From default wallet to position-manager
solana transfer \
  $(solana address -k ~/.config/solana/position-manager.json) \
  1 \
  --from ~/.config/solana/id.json

# From position-manager to liquidation-engine
solana transfer \
  $(solana address -k ~/.config/solana/liquidation-engine.json) \
  0.5 \
  --from ~/.config/solana/position-manager.json
```

### **3. Check Balance Before/After**

```bash
# Check sender balance
solana balance

# Check recipient balance
solana balance <RECIPIENT_ADDRESS>

# Check specific keypair balance
solana balance -k ~/.config/solana/position-manager.json
```

---

## ⚙️ Options

### **Network Selection**

```bash
# Devnet (default for testing)
solana transfer <ADDRESS> <AMOUNT> --url devnet

# Mainnet (real money!)
solana transfer <ADDRESS> <AMOUNT> --url mainnet-beta

# Localnet
solana transfer <ADDRESS> <AMOUNT> --url localhost
```

### **Transaction Options**

```bash
# Allow unfunded recipient (creates account if needed)
solana transfer <ADDRESS> <AMOUNT> --allow-unfunded-recipient

# Specify fee payer (different from sender)
# Use this when the sender doesn't have SOL for fees, or you want another account to pay fees
solana transfer <ADDRESS> <AMOUNT> \
  --from ~/.config/solana/wallet2.json \    # Sender (pays transfer amount)
  --fee-payer ~/.config/solana/id.json      # Fee payer (pays ~0.000005 SOL fee)

# Dry run (simulate without sending)
solana transfer <ADDRESS> <AMOUNT> --dry-run
```

---

## 💡 Examples

### **Example 1: Fund Position Manager**

```bash
# 1. Get position manager address
POS_MGR=$(solana address -k ~/.config/solana/position-manager.json)
echo "Position Manager: $POS_MGR"

# 2. Transfer 2 SOL
solana transfer $POS_MGR 2

# 3. Verify
solana balance -k ~/.config/solana/position-manager.json
```

### **Example 2: Fund Liquidation Engine**

```bash
# 1. Get liquidation engine address
LIQ_ENG=$(solana address -k ~/.config/solana/liquidation-engine.json)
echo "Liquidation Engine: $LIQ_ENG"

# 2. Transfer 1 SOL
solana transfer $LIQ_ENG 1

# 3. Verify
solana balance -k ~/.config/solana/liquidation-engine.json
```

### **Example 3: Transfer from One Keypair to Another**

```bash
# Transfer from position-manager to liquidation-engine
solana transfer \
  $(solana address -k ~/.config/solana/liquidation-engine.json) \
  0.5 \
  --from ~/.config/solana/position-manager.json \
  --url devnet
```

---

## ⚠️ Important Notes

1. **Transaction Fees**: Each transfer costs ~0.000005 SOL (5,000 lamports) in fees
2. **Minimum Balance**: Accounts need a small amount of SOL for rent exemption
3. **Network**: Make sure you're on the correct network (devnet/mainnet)
4. **Confirmations**: Transactions usually confirm in 1-2 seconds

---

## 🔍 Troubleshooting

### **Error: "Insufficient funds"**

```bash
# Check your balance
solana balance

# You need: amount + transaction fee (~0.000005 SOL)
# Example: To send 1 SOL, you need at least 1.000005 SOL
```

### **Error: "Account not found"**

```bash
# Use --allow-unfunded-recipient to create the account
solana transfer <ADDRESS> <AMOUNT> --allow-unfunded-recipient
```

### **Error: "Network mismatch"**

```bash
# Check which network you're on
solana config get

# Set correct network
solana config set --url devnet
# or
solana config set --url mainnet-beta
```

---

## 📊 Quick Reference

| Command | Description |
|---------|-------------|
| `solana transfer <ADDR> <AMT>` | Transfer SOL to address |
| `solana balance` | Check default wallet balance |
| `solana balance <ADDR>` | Check specific address balance |
| `solana balance -k <KEYPAIR>` | Check keypair balance |
| `solana address` | Show default wallet address |
| `solana address -k <KEYPAIR>` | Show keypair address |

---

## 🎯 For Your Project

To fund your Position Manager and Liquidation Engine:

```bash
# Fund Position Manager (2 SOL)
solana transfer \
  $(solana address -k ~/.config/solana/position-manager.json) \
  2 \
  --url devnet

# Fund Liquidation Engine (2 SOL)
solana transfer \
  $(solana address -k ~/.config/solana/liquidation-engine.json) \
  2 \
  --url devnet
```

That's it! 🚀
