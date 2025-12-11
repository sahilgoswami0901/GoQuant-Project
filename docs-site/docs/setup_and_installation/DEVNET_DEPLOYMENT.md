# Collateral Vault - Devnet Deployment Guide

Deploy your Collateral Vault to Solana Devnet and view on [Solana Explorer](https://explorer.solana.com/?cluster=devnet).

---

## 🌐 Why Devnet?

| Feature | Localhost | Devnet |
|---------|-----------|--------|
| Persistence | Resets when validator stops | Permanent |
| Explorer | No | Yes ✅ |
| Free SOL | Unlimited airdrops | Limited airdrops |
| Speed | Instant | ~400ms |
| Good for | Development | Testing, Demo |

---

## Step 1: Configure Solana for Devnet

```bash
# Switch to devnet
solana config set --url devnet

# Verify configuration
solana config get

# Expected output:
# Config File: ~/.config/solana/cli/config.yml
# RPC URL: https://api.devnet.solana.com
# WebSocket URL: wss://api.devnet.solana.com
# Keypair Path: ~/.config/solana/id.json
```

---

## Step 2: Get Devnet SOL

You need SOL for deployment and testing. Devnet SOL is free but limited.

```bash
# Airdrop 2 SOL (devnet limit per request)
solana airdrop 2

# Wait a few seconds, then airdrop more
solana airdrop 2

# Check balance
solana balance

# You need at least 4-5 SOL for deployment
```

**If airdrops fail** (rate limited), use the faucet:
- Go to: https://faucet.solana.com/
- Select "Devnet"
- Paste your wallet address: `solana address`
- Request SOL

---

## Step 3: Update Anchor Configuration

Edit `Anchor.toml` to use devnet:

```bash
cd "/Users/sahilgoswami/Desktop/GoQuant Project/collateral-vault"
```

Update `Anchor.toml`:

```toml
[toolchain]

[features]
resolution = true
skip-lint = false

[programs.devnet]
collateral_vault = "YOUR_PROGRAM_ID"

[programs.localnet]
collateral_vault = "YOUR_PROGRAM_ID"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "devnet"
wallet = "~/.config/solana/id.json"

[scripts]
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
```

---

## Step 4: Build the Program

```bash
# Clean previous builds
anchor clean

# Build fresh
anchor build

# Get your program ID
solana address -k target/deploy/collateral_vault-keypair.json
```

**Copy the program ID** - you'll need it!

---

## Step 5: Update Program ID in Code

Update the program ID in two places:

### 5.1 Update `lib.rs`

```bash
# Edit the file
nano programs/collateral-vault/src/lib.rs
```

Find and update:
```rust
declare_id!("YOUR_NEW_PROGRAM_ID_HERE");
```

### 5.2 Update `Anchor.toml`

```toml
[programs.devnet]
collateral_vault = "YOUR_NEW_PROGRAM_ID_HERE"

[programs.localnet]
collateral_vault = "YOUR_NEW_PROGRAM_ID_HERE"
```

### 5.3 Rebuild with new ID

```bash
anchor build
```

---

## Step 6: Deploy to Devnet

```bash
# Deploy the program
anchor deploy --provider.cluster devnet

# Expected output:
# Deploying cluster: https://api.devnet.solana.com
# Upgrade authority: <your-wallet>
# Deploying program "collateral_vault"...
# Program Id: <YOUR_PROGRAM_ID>
# Deploy success
```

**🎉 Your program is now live on Devnet!**

---

## Step 7: View on Solana Explorer

Open your program on Solana Explorer:

```
https://explorer.solana.com/address/YOUR_PROGRAM_ID?cluster=devnet
```

Replace `YOUR_PROGRAM_ID` with your actual program ID.

**Example:**
```
https://explorer.solana.com/address/AVRBwuFHdU51wxP3a8brB95KL1VT7PCrFAVt1zzmjde?cluster=devnet
```

---

## Step 8: Create a Test Token (Mock USDT)

Since real USDT isn't on devnet, create a mock token:

```bash
# Create a new token mint
spl-token create-token --decimals 6

# Note the token mint address (e.g., "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr")

# Create a token account for yourself
spl-token create-account <TOKEN_MINT_ADDRESS>

# Mint some tokens (1 million = 1,000,000 USDT with 6 decimals)
spl-token mint <TOKEN_MINT_ADDRESS> 1000000000

# Check balance
spl-token balance <TOKEN_MINT_ADDRESS>
```

**Save your token mint address** - this is your "test USDT"!

---

## Step 9: Run Tests on Devnet

```bash
# Run tests against devnet
anchor test --provider.cluster devnet --skip-deploy

# Or with more verbose output
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com anchor test --skip-deploy
```

---

## Step 10: Update Backend for Devnet

Update the backend `.env` file:

```bash
cd backend

cat > .env << 'EOF'
# Database
DATABASE_URL=postgres://YOUR_USERNAME:@localhost:5432/collateral_vault

# Solana DEVNET
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_WS_URL=wss://api.devnet.solana.com
VAULT_PROGRAM_ID=YOUR_PROGRAM_ID_HERE
USDT_MINT=YOUR_TEST_TOKEN_MINT_HERE

# Server
SERVER_HOST=127.0.0.1
SERVER_PORT=8080

# Monitoring
BALANCE_CHECK_INTERVAL=30
RECONCILIATION_INTERVAL=300
LOW_BALANCE_THRESHOLD=100

# Logging
RUST_LOG=info,collateral_vault_backend=debug
EOF
```

Replace:
- `YOUR_PROGRAM_ID_HERE` with your deployed program ID
- `YOUR_TEST_TOKEN_MINT_HERE` with the token mint from Step 8

---

## Step 11: Test Everything

### Initialize Vault Authority (One Time)

```typescript
// Run this once to set up the vault authority
anchor run initialize-authority --provider.cluster devnet
```

Or use a script:

```bash
cd "/Users/sahilgoswami/Desktop/GoQuant Project/collateral-vault"

cat > scripts/init-devnet.ts << 'EOF'
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";

async function main() {
    // Connect to devnet
    const connection = new anchor.web3.Connection(
        "https://api.devnet.solana.com",
        "confirmed"
    );
    
    const wallet = anchor.Wallet.local();
    const provider = new anchor.AnchorProvider(connection, wallet, {});
    anchor.setProvider(provider);
    
    // Your program ID
    const programId = new PublicKey("YOUR_PROGRAM_ID_HERE");
    
    console.log("Wallet:", wallet.publicKey.toString());
    console.log("Program:", programId.toString());
    
    // Derive vault authority PDA
    const [vaultAuthorityPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault_authority")],
        programId
    );
    console.log("Vault Authority PDA:", vaultAuthorityPda.toString());
    
    // Check if already initialized
    const account = await connection.getAccountInfo(vaultAuthorityPda);
    if (account) {
        console.log("Vault authority already initialized!");
        return;
    }
    
    console.log("Initializing vault authority...");
    // Add initialization code here using your IDL
    
    console.log("Done!");
}

main().catch(console.error);
EOF

npx ts-node scripts/init-devnet.ts
```

### View Transactions on Explorer

Every transaction you make will be visible:

1. **Initialize Vault:**
   ```
   https://explorer.solana.com/tx/SIGNATURE?cluster=devnet
   ```

2. **View Vault Account:**
   ```
   https://explorer.solana.com/address/VAULT_PDA?cluster=devnet
   ```

3. **View Token Account:**
   ```
   https://explorer.solana.com/address/TOKEN_ACCOUNT?cluster=devnet
   ```

---

## 📊 Useful Devnet Links

| Resource | Link |
|----------|------|
| Explorer | https://explorer.solana.com/?cluster=devnet |
| Faucet | https://faucet.solana.com/ |
| RPC Status | https://status.solana.com/ |
| Your Program | `https://explorer.solana.com/address/YOUR_PROGRAM_ID?cluster=devnet` |
| Your Wallet | `https://explorer.solana.com/address/YOUR_WALLET?cluster=devnet` |

---

## ⚠️ Devnet Limitations

| Limitation | Details |
|------------|---------|
| Airdrop limit | ~2 SOL per request, rate limited |
| No real tokens | Use mock tokens |
| Can be slow | ~400ms confirmation |
| May reset | Devnet occasionally resets |
| Rate limits | RPC may throttle requests |

---

## 🔄 Quick Commands Reference

```bash
# Switch to devnet
solana config set --url devnet

# Check balance
solana balance

# Airdrop SOL
solana airdrop 2

# Deploy program
anchor deploy --provider.cluster devnet

# Run tests
anchor test --provider.cluster devnet --skip-deploy

# View your wallet on explorer
echo "https://explorer.solana.com/address/$(solana address)?cluster=devnet"

# View your program on explorer
echo "https://explorer.solana.com/address/YOUR_PROGRAM_ID?cluster=devnet"
```

---

## 🎬 Recording Demo for Submission

Since you can now see everything on Solana Explorer, you can:

1. **Record screen** while performing operations
2. **Show Explorer** after each transaction
3. **Demonstrate:**
   - Initialize vault → Show on Explorer
   - Deposit → Show token transfer on Explorer
   - Withdraw → Show token transfer on Explorer
   - Lock/Unlock → Show account state changes

This makes a great video demo for your assignment!

---

## Troubleshooting

### "Airdrop failed"
```bash
# Use the web faucet instead
open https://faucet.solana.com/
```

### "Transaction failed"
```bash
# Check your SOL balance
solana balance

# Might need more SOL for fees
solana airdrop 2
```

### "Account not found"
Make sure you're looking at the right cluster:
```
?cluster=devnet
```

### "Program not found"
Verify deployment:
```bash
solana program show YOUR_PROGRAM_ID --url devnet
```

