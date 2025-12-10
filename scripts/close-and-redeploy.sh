#!/bin/bash

# Script to close existing program and redeploy with new wallet
# This is needed when you want to switch upgrade authority

set -e

PROGRAM_ID="AVRBwuFHdU51wxP3a8brB95KL1VT7PCrFAVt1zzmjde"
OLD_WALLET="~/.config/solana/id.json.backup"
NEW_WALLET="~/.config/solana/id.json"

echo "🔄 Closing program and redeploying with new wallet"
echo "=================================================="
echo ""
echo "Program ID: $PROGRAM_ID"
echo "Old wallet (authority): $OLD_WALLET"
echo "New wallet: $NEW_WALLET"
echo ""

# Check if old wallet exists
if [ ! -f ~/.config/solana/id.json.backup ]; then
    echo "❌ Error: Old wallet not found at ~/.config/solana/id.json.backup"
    exit 1
fi

# Check if new wallet exists
if [ ! -f ~/.config/solana/id.json ]; then
    echo "❌ Error: New wallet not found at ~/.config/solana/id.json"
    exit 1
fi

# Get program data address
echo "📋 Getting program data address..."
PROGRAM_DATA=$(solana program show $PROGRAM_ID --output json | jq -r '.programdataAddress // empty')

if [ -z "$PROGRAM_DATA" ]; then
    echo "❌ Error: Could not find program data address"
    exit 1
fi

echo "Program Data Address: $PROGRAM_DATA"
echo ""

# Check balance
BALANCE=$(solana program show $PROGRAM_ID --output json | jq -r '.accountBalance // 0')
echo "Program Balance: $BALANCE SOL"
echo ""

# Step 1: Close the program using old wallet
echo "Step 1: Closing program with old wallet..."
echo "This will recover the rent (${BALANCE} SOL) to the old wallet"
echo ""

read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 1
fi

# Close the program
echo "Closing program..."
solana program close $PROGRAM_ID \
    --bypass-warning \
    --keypair $OLD_WALLET

if [ $? -eq 0 ]; then
    echo "✅ Program closed successfully!"
    echo "   Rent recovered to old wallet"
else
    echo "❌ Failed to close program"
    exit 1
fi

echo ""
echo "Step 2: Building program..."
anchor build

if [ $? -ne 0 ]; then
    echo "❌ Build failed"
    exit 1
fi

echo ""
echo "Step 3: Deploying with new wallet..."
anchor deploy --provider.cluster devnet

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Deployment successful!"
    echo ""
    NEW_PROGRAM_ID=$(solana address -k target/deploy/collateral_vault-keypair.json)
    echo "New Program ID: $NEW_PROGRAM_ID"
    echo ""
    echo "⚠️  Note: If the program ID changed, update:"
    echo "   - Anchor.toml"
    echo "   - backend/.env (VAULT_PROGRAM_ID)"
    echo "   - programs/collateral-vault/src/lib.rs (declare_id!)"
else
    echo "❌ Deployment failed"
    exit 1
fi
