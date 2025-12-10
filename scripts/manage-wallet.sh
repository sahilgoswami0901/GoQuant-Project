#!/bin/bash

# Solana Wallet Management Script
# This script helps you backup, create, and manage Solana wallets

set -e

WALLET_DIR="$HOME/.config/solana"
BACKUP_DIR="$WALLET_DIR/backups"
DEFAULT_WALLET="$WALLET_DIR/id.json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create backup directory if it doesn't exist
mkdir -p "$BACKUP_DIR"

echo -e "${BLUE}🔐 Solana Wallet Management${NC}"
echo "================================"
echo ""

# Function to show current wallet info
show_current_wallet() {
    if [ -f "$DEFAULT_WALLET" ]; then
        ADDRESS=$(solana address 2>/dev/null || echo "N/A")
        BALANCE=$(solana balance 2>/dev/null | head -n1 || echo "N/A")
        echo -e "${GREEN}Current Default Wallet:${NC}"
        echo "  Address: $ADDRESS"
        echo "  Balance: $BALANCE"
        echo "  Path: $DEFAULT_WALLET"
    else
        echo -e "${YELLOW}No default wallet found at $DEFAULT_WALLET${NC}"
    fi
}

# Function to backup current wallet
backup_wallet() {
    if [ -f "$DEFAULT_WALLET" ]; then
        TIMESTAMP=$(date +%Y%m%d-%H%M%S)
        BACKUP_FILE="$BACKUP_DIR/id-$TIMESTAMP.json"
        cp "$DEFAULT_WALLET" "$BACKUP_FILE"
        
        # Save address info
        ADDRESS=$(solana address)
        echo "$ADDRESS" > "$BACKUP_DIR/id-$TIMESTAMP-address.txt"
        
        echo -e "${GREEN}✅ Wallet backed up!${NC}"
        echo "  Backup: $BACKUP_FILE"
        echo "  Address: $ADDRESS"
        return 0
    else
        echo -e "${YELLOW}⚠️  No wallet to backup${NC}"
        return 1
    fi
}

# Function to create new wallet
create_new_wallet() {
    echo -e "${YELLOW}⚠️  This will OVERWRITE your current default wallet!${NC}"
    read -p "Have you backed up your current wallet? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}Backing up current wallet first...${NC}"
        backup_wallet || true
    fi
    
    echo -e "${BLUE}Creating new wallet...${NC}"
    solana-keygen new --outfile "$DEFAULT_WALLET" --force --no-bip39-passphrase
    
    NEW_ADDRESS=$(solana address)
    echo -e "${GREEN}✅ New wallet created!${NC}"
    echo "  Address: $NEW_ADDRESS"
    echo "  Path: $DEFAULT_WALLET"
}

# Function to create additional wallet (non-default)
create_additional_wallet() {
    read -p "Enter name for new wallet (e.g., 'new-deployment'): " WALLET_NAME
    if [ -z "$WALLET_NAME" ]; then
        WALLET_NAME="wallet-$(date +%Y%m%d-%H%M%S)"
    fi
    
    NEW_WALLET="$WALLET_DIR/$WALLET_NAME-keypair.json"
    solana-keygen new --outfile "$NEW_WALLET" --no-bip39-passphrase
    
    NEW_ADDRESS=$(solana address -k "$NEW_WALLET")
    echo -e "${GREEN}✅ New wallet created!${NC}"
    echo "  Address: $NEW_ADDRESS"
    echo "  Path: $NEW_WALLET"
    echo ""
    echo "To use this wallet:"
    echo "  solana address -k $NEW_WALLET"
    echo "  anchor deploy --provider.wallet $NEW_WALLET"
}

# Function to list all wallets
list_wallets() {
    echo -e "${BLUE}📋 All Wallets:${NC}"
    echo ""
    
    # List default wallet
    if [ -f "$DEFAULT_WALLET" ]; then
        ADDRESS=$(solana address 2>/dev/null || echo "N/A")
        echo -e "${GREEN}Default:${NC} $DEFAULT_WALLET"
        echo "  Address: $ADDRESS"
        echo ""
    fi
    
    # List other keypairs
    echo -e "${BLUE}Other Keypairs:${NC}"
    for file in "$WALLET_DIR"/*-keypair.json "$WALLET_DIR"/*.json; do
        if [ -f "$file" ] && [ "$file" != "$DEFAULT_WALLET" ]; then
            ADDRESS=$(solana address -k "$file" 2>/dev/null || echo "N/A")
            echo "  $(basename "$file")"
            echo "    Address: $ADDRESS"
        fi
    done
    
    # List backups
    if [ -d "$BACKUP_DIR" ] && [ "$(ls -A $BACKUP_DIR/*.json 2>/dev/null)" ]; then
        echo ""
        echo -e "${YELLOW}Backups:${NC}"
        for file in "$BACKUP_DIR"/*.json; do
            if [ -f "$file" ]; then
                ADDRESS_FILE="${file%.json}-address.txt"
                if [ -f "$ADDRESS_FILE" ]; then
                    ADDRESS=$(cat "$ADDRESS_FILE")
                else
                    ADDRESS="N/A"
                fi
                echo "  $(basename "$file")"
                echo "    Address: $ADDRESS"
            fi
        done
    fi
}

# Function to restore from backup
restore_wallet() {
    echo -e "${BLUE}Available backups:${NC}"
    ls -1 "$BACKUP_DIR"/*.json 2>/dev/null | nl
    
    read -p "Enter backup number to restore: " BACKUP_NUM
    BACKUP_FILE=$(ls -1 "$BACKUP_DIR"/*.json 2>/dev/null | sed -n "${BACKUP_NUM}p")
    
    if [ -z "$BACKUP_FILE" ] || [ ! -f "$BACKUP_FILE" ]; then
        echo -e "${RED}❌ Invalid backup number${NC}"
        return 1
    fi
    
    echo -e "${YELLOW}⚠️  This will overwrite your current default wallet!${NC}"
    read -p "Continue? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        return 1
    fi
    
    cp "$BACKUP_FILE" "$DEFAULT_WALLET"
    ADDRESS=$(solana address)
    echo -e "${GREEN}✅ Wallet restored!${NC}"
    echo "  Address: $ADDRESS"
}

# Function to airdrop SOL
airdrop_sol() {
    echo -e "${BLUE}Requesting SOL airdrop...${NC}"
    echo "Note: Devnet has rate limits. You may need to run this multiple times."
    echo ""
    
    for i in {1..3}; do
        echo "Attempt $i/3..."
        if solana airdrop 2; then
            sleep 3
        else
            echo -e "${YELLOW}⚠️  Airdrop failed. Wait a few seconds and try again.${NC}"
            sleep 5
        fi
    done
    
    BALANCE=$(solana balance)
    echo ""
    echo -e "${GREEN}Current balance: $BALANCE${NC}"
}

# Main menu
show_menu() {
    echo ""
    echo -e "${BLUE}What would you like to do?${NC}"
    echo "1) Show current wallet info"
    echo "2) Backup current wallet"
    echo "3) Create new default wallet (OVERWRITES current)"
    echo "4) Create additional wallet (keeps current default)"
    echo "5) List all wallets"
    echo "6) Restore wallet from backup"
    echo "7) Airdrop SOL to current wallet"
    echo "8) Exit"
    echo ""
    read -p "Enter choice [1-8]: " choice
    
    case $choice in
        1)
            show_current_wallet
            ;;
        2)
            backup_wallet
            ;;
        3)
            create_new_wallet
            ;;
        4)
            create_additional_wallet
            ;;
        5)
            list_wallets
            ;;
        6)
            restore_wallet
            ;;
        7)
            airdrop_sol
            ;;
        8)
            echo "Goodbye!"
            exit 0
            ;;
        *)
            echo -e "${RED}Invalid choice${NC}"
            ;;
    esac
}

# Check if solana CLI is installed
if ! command -v solana &> /dev/null; then
    echo -e "${RED}❌ Solana CLI not found. Please install it first.${NC}"
    exit 1
fi

# Show current wallet info
show_current_wallet

# Main loop
while true; do
    show_menu
    echo ""
    read -p "Press Enter to continue..."
    clear
    show_current_wallet
done
