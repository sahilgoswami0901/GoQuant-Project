#!/bin/bash

# ============================================
# Collateral Vault - Local Testing Script
# ============================================
# This script helps you test the complete system locally.
# Run each section step by step.

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Collateral Vault - Local Test Suite  ${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check prerequisites
check_command() {
    if command -v $1 &> /dev/null; then
        echo -e "${GREEN}✓${NC} $1 is installed"
        return 0
    else
        echo -e "${RED}✗${NC} $1 is NOT installed"
        return 1
    fi
}

echo -e "${YELLOW}Step 1: Checking Prerequisites${NC}"
echo "--------------------------------"
check_command rustc
check_command solana
check_command anchor
check_command node
check_command yarn
check_command psql
echo ""

# Check Solana config
echo -e "${YELLOW}Step 2: Checking Solana Configuration${NC}"
echo "--------------------------------------"
SOLANA_URL=$(solana config get | grep "RPC URL" | awk '{print $3}')
echo "Current RPC URL: $SOLANA_URL"

if [[ "$SOLANA_URL" == *"localhost"* ]] || [[ "$SOLANA_URL" == *"127.0.0.1"* ]]; then
    echo -e "${GREEN}✓${NC} Configured for local development"
else
    echo -e "${YELLOW}!${NC} Not configured for localhost. Run: solana config set --url localhost"
fi
echo ""

# Check if validator is running
echo -e "${YELLOW}Step 3: Checking Local Validator${NC}"
echo "---------------------------------"
if pgrep -x "solana-test-validator" > /dev/null; then
    echo -e "${GREEN}✓${NC} Local validator is running"
else
    echo -e "${RED}✗${NC} Local validator is NOT running"
    echo "  Start it with: solana-test-validator"
    echo ""
    echo -e "${YELLOW}Would you like to start the validator? (y/n)${NC}"
    read -r START_VALIDATOR
    if [[ "$START_VALIDATOR" == "y" ]]; then
        echo "Starting validator in background..."
        solana-test-validator &
        sleep 5
    fi
fi
echo ""

# Check SOL balance
echo -e "${YELLOW}Step 4: Checking SOL Balance${NC}"
echo "-----------------------------"
WALLET=$(solana address 2>/dev/null || echo "NO_WALLET")
if [[ "$WALLET" != "NO_WALLET" ]]; then
    BALANCE=$(solana balance 2>/dev/null | awk '{print $1}')
    echo "Wallet: $WALLET"
    echo "Balance: $BALANCE SOL"
    
    if (( $(echo "$BALANCE < 1" | bc -l) )); then
        echo -e "${YELLOW}!${NC} Low balance. Airdropping 10 SOL..."
        solana airdrop 10 2>/dev/null || echo "Airdrop failed (might already have enough)"
    fi
else
    echo -e "${RED}✗${NC} No wallet found. Generate one with: solana-keygen new"
fi
echo ""

# Check database
echo -e "${YELLOW}Step 5: Checking Database${NC}"
echo "--------------------------"
if psql -lqt | cut -d \| -f 1 | grep -qw collateral_vault; then
    echo -e "${GREEN}✓${NC} Database 'collateral_vault' exists"
else
    echo -e "${YELLOW}!${NC} Database not found. Creating..."
    createdb collateral_vault 2>/dev/null && echo -e "${GREEN}✓${NC} Database created" || echo -e "${RED}✗${NC} Failed to create database"
fi
echo ""

# Test backend health (if running)
echo -e "${YELLOW}Step 6: Checking Backend Service${NC}"
echo "---------------------------------"
BACKEND_RESPONSE=$(curl -s http://localhost:8080/health 2>/dev/null || echo "NOT_RUNNING")
if [[ "$BACKEND_RESPONSE" != "NOT_RUNNING" ]]; then
    echo -e "${GREEN}✓${NC} Backend is running"
    echo "Response: $BACKEND_RESPONSE" | head -c 100
    echo "..."
else
    echo -e "${YELLOW}!${NC} Backend is not running"
    echo "  Start it with: cd backend && cargo run"
fi
echo ""

# Summary
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}            Summary                     ${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "To fully test the system, ensure:"
echo ""
echo "Terminal 1 - Validator:"
echo "  $ solana-test-validator"
echo ""
echo "Terminal 2 - Backend:"
echo "  $ cd backend && cargo run"
echo ""
echo "Terminal 3 - Tests:"
echo "  $ anchor test --skip-local-validator"
echo ""
echo "Or test APIs manually:"
echo "  $ curl http://localhost:8080/health"
echo "  $ curl http://localhost:8080/vault/tvl"
echo ""
echo -e "${GREEN}Happy testing! 🚀${NC}"


