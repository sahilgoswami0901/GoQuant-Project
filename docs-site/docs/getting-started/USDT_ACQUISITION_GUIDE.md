# 💰 USDT Acquisition Guide

**Question:** Who puts USDT into the user's token account before they can deposit it into the vault?

**Answer:** **The user themselves** must acquire USDT before depositing. The vault system does NOT provide USDT - it only manages deposits and withdrawals of USDT that users already own.

---

## 🔄 Complete Deposit Flow

```text
┌─────────────────────────────────────────────────────────────┐
│                    DEPOSIT FLOW                             │
└─────────────────────────────────────────────────────────────┘

Step 1: User Acquires USDT
    │
    ├─ Option A: Buy USDT on DEX (Jupiter, Raydium, Orca)
    ├─ Option B: Receive USDT from another user
    ├─ Option C: Bridge USDT from another blockchain
    └─ Option D: (Testing only) Mint test USDT
    │
    ↓
    User's Token Account now has USDT ✅
    │
    ↓
Step 2: User Calls Deposit Endpoint
    │
    POST /vault/deposit
    {
      "userPubkey": "...",
      "amount": 100000000  // 100 USDT
    }
    │
    ↓
Step 3: Backend Builds Transaction
    │
    Transfers USDT from:
    - user_token_account (source)
    - vault_token_account (destination)
    │
    ↓
Step 4: User Signs & Submits Transaction
    │
    ↓
Step 5: USDT Moves to Vault ✅
```

---

## 📍 Where USDT Comes From

### Option 1: Buy USDT on a DEX (Decentralized Exchange)

**Most common method for users:**

1. **Jupiter Aggregator** (Recommended)
   - Best rates by aggregating multiple DEXs
   - Website: https://jup.ag/
   - Swap SOL → USDT
   - Automatically creates token account if needed

2. **Raydium**
   - Direct DEX on Solana
   - Website: https://raydium.io/
   - Swap SOL → USDT

3. **Orca**
   - Another popular Solana DEX
   - Website: https://www.orca.so/
   - Swap SOL → USDT

**How it works:**
```text
User Wallet (SOL) → DEX Swap → User Token Account (USDT)
```

### Option 2: Receive USDT from Another User

Users can send USDT to each other using standard SPL Token transfers:

```typescript
// Example: User A sends USDT to User B
import { transfer } from "@solana/spl-token";

await transfer(
  connection,
  payer,              // User A (signer)
  sourceTokenAccount, // User A's USDT token account
  destinationTokenAccount, // User B's USDT token account
  owner,              // User A (authority)
  100000000           // 100 USDT
);
```

### Option 3: Bridge from Another Chain

Users can bridge USDT from Ethereum, BSC, or other chains:

- **Wormhole**: https://wormhole.com/
- **Portal (Wormhole UI)**: https://portalbridge.com/
- **Allbridge**: https://allbridge.io/

### Option 4: Testing Only - Mint Test USDT

**⚠️ Only for development/testing!**

In your test files, you create a mock USDT mint and mint tokens:

```typescript
// From tests/devnet-test.ts
usdtMint = await createMint(
  provider.connection,
  user,
  user.publicKey,  // Mint authority
  null,
  USDT_DECIMALS
);

// Mint USDT to user
await mintTo(
  provider.connection,
  user,
  usdtMint,
  userTokenAccount,
  user.publicKey,  // Must be mint authority
  INITIAL_BALANCE
);
```

**Note:** In production, you cannot mint real USDT - only Tether (the issuer) can mint USDT.

---

## 🏗️ Architecture: Why Users Need USDT First

### Your Vault System's Role

Your Collateral Vault system is a **custodial vault**, not a token provider:

```text
┌─────────────────────────────────────────┐
│     What Your Vault System Does:       │
├─────────────────────────────────────────┤
│ ✅ Accept deposits of USDT             │
│ ✅ Store USDT securely                  │
│ ✅ Track balances (total/locked/avail)  │
│ ✅ Allow withdrawals                    │
│ ✅ Lock/unlock collateral               │
│                                         │
│ ❌ Does NOT mint USDT                   │
│ ❌ Does NOT provide USDT                │
│ ❌ Does NOT buy USDT for users          │
└─────────────────────────────────────────┘
```

### Token Account Creation

When a user first interacts with USDT, their **Associated Token Account (ATA)** is automatically created:

```text
User Wallet Address: 7xKt9Fj2...
    │
    └─ Associated Token Account (ATA)
       Address: Derived from (user_wallet, USDT_mint)
       Balance: 0 USDT initially
```

**ATA is created automatically when:**
- User swaps SOL → USDT on a DEX
- User receives USDT from another user
- User calls `getOrCreateAssociatedTokenAccount()`

---

## 💡 Integration Options for Your System

### Option A: Let Users Handle It (Current Approach)

**Pros:**
- ✅ Simple - no additional code needed
- ✅ Users have full control
- ✅ No regulatory concerns about token distribution

**Cons:**
- ❌ Users need to know how to buy USDT
- ❌ Extra step for users

**Implementation:**
- Document how to buy USDT in your UI/docs
- Provide links to DEXs
- Show wallet balance before allowing deposit

### Option B: Add DEX Integration (Advanced)

You could integrate a DEX aggregator API to let users buy USDT directly in your app:

```typescript
// Example: Integrate Jupiter Swap API
async function buyUSDT(userWallet: PublicKey, solAmount: number) {
  // 1. Get quote from Jupiter
  const quote = await fetch(
    `https://quote-api.jup.ag/v6/quote?inputMint=So11111111111111111111111111111111111111112&outputMint=${USDT_MINT}&amount=${solAmount}`
  );
  
  // 2. Build swap transaction
  const swapTx = await buildSwapTransaction(quote);
  
  // 3. User signs and submits
  // 4. USDT arrives in user's token account
  // 5. User can now deposit to vault
}
```

**Pros:**
- ✅ Better UX - one-click buy + deposit
- ✅ Users don't need to leave your app

**Cons:**
- ❌ More complex implementation
- ❌ Need to handle swap failures
- ❌ Additional fees/risks

### Option C: Provide Test USDT Faucet (Devnet Only)

For testing on devnet, you could create a faucet endpoint:

```rust
// backend/src/api/handlers.rs
pub async fn faucet_usdt(
    state: web::Data<Arc<AppState>>,
    body: web::Json<FaucetRequest>,
) -> HttpResponse {
    // Only allow on devnet!
    // Mint test USDT to user's token account
    // This is ONLY for testing - never on mainnet!
}
```

---

## 📝 Current Implementation Details

### Deposit Endpoint Flow

Looking at your code (`backend/src/services/transaction_builder.rs`):

```rust
pub async fn build_deposit(
    &self,
    user_pubkey: &str,
    amount: u64,
) -> Result<String, TransactionBuilderError> {
    // Gets user's token account
    let user_token_account = spl_associated_token_account::get_associated_token_address(
        &user,
        &self.usdt_mint,
    );
    
    // Gets vault's token account
    let vault_token_account = spl_associated_token_account::get_associated_token_address(
        &vault_pda,
        &self.usdt_mint,
    );
    
    // Builds transfer instruction:
    // FROM: user_token_account (must have USDT!)
    // TO: vault_token_account
    // AUTHORITY: user (signs transaction)
}
```

**Key Point:** The transaction assumes `user_token_account` already has USDT. If it doesn't, the transaction will fail with "Insufficient funds".

### Smart Contract Validation

Your smart contract (`programs/collateral-vault/src/instructions/deposit.rs`) performs a CPI (Cross-Program Invocation) to the SPL Token Program:

```rust
// This will fail if user_token_account doesn't have enough USDT
token::transfer(cpi_context, amount)?;
```

The SPL Token Program checks:
- ✅ `user_token_account` exists
- ✅ `user_token_account` has sufficient balance
- ✅ User signed the transaction (authority check)

---

## 🎯 Recommended User Flow

### For Your Frontend/UI:

```text
┌─────────────────────────────────────────┐
│         User Deposit Flow              │
└─────────────────────────────────────────┘

1. User clicks "Deposit"
   │
   ↓
2. Check user's USDT balance
   │
   ├─ If balance = 0:
   │  └─ Show: "You need USDT first!"
   │     └─ Button: "Buy USDT on Jupiter" (opens DEX)
   │
   └─ If balance > 0:
      └─ Show: "You have X USDT"
         └─ Allow deposit
            │
            ↓
3. User enters deposit amount
   │
   ├─ If amount > balance:
   │  └─ Error: "Insufficient USDT"
   │
   └─ If amount <= balance:
      └─ Proceed with deposit
```

### Example Frontend Code:

```typescript
async function checkAndDeposit(userPubkey: string, amount: number) {
  // 1. Check user's USDT balance
  const userTokenAccount = getAssociatedTokenAddressSync(
    new PublicKey(USDT_MINT),
    new PublicKey(userPubkey)
  );
  
  const balance = await connection.getTokenAccountBalance(userTokenAccount);
  
  if (balance.value.amount === "0") {
    // Show UI: "You need USDT first! Buy on Jupiter"
    window.open(`https://jup.ag/swap/SOL-USDT`);
    return;
  }
  
  if (parseInt(balance.value.amount) < amount) {
    throw new Error("Insufficient USDT balance");
  }
  
  // 2. Proceed with deposit
  const depositTx = await fetch("/vault/deposit", {
    method: "POST",
    body: JSON.stringify({ userPubkey, amount })
  });
  
  // 3. User signs and submits...
}
```

---

## 🔍 Checking User's USDT Balance

### Using Your Backend API:

You could add an endpoint to check user's USDT balance:

```rust
// backend/src/api/handlers.rs
pub async fn get_user_usdt_balance(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> HttpResponse {
    let user = path.into_inner();
    
    // Get user's token account
    let user_token_account = get_associated_token_address(
        &Pubkey::from_str(&user)?,
        &state.solana.usdt_mint(),
    );
    
    // Check balance
    let balance = state.solana.get_token_balance(
        &user_token_account.to_string()
    ).await?;
    
    // Return balance
    Ok(balance)
}
```

### Using Solana RPC Directly:

```typescript
import { getAssociatedTokenAddressSync, getAccount } from "@solana/spl-token";

const userTokenAccount = getAssociatedTokenAddressSync(
  USDT_MINT,
  userPublicKey
);

const accountInfo = await getAccount(connection, userTokenAccount);
console.log("USDT Balance:", accountInfo.amount);
```

---

## 📚 Summary

| Question | Answer |
|----------|--------|
| **Who puts USDT in user's token account?** | **The user themselves** (or someone sending them USDT) |
| **Does your vault provide USDT?** | ❌ No - it only manages deposits/withdrawals |
| **How do users get USDT?** | Buy on DEX (Jupiter, Raydium, Orca), receive from others, or bridge |
| **What happens if user has no USDT?** | Deposit transaction fails with "Insufficient funds" |
| **Can you mint USDT?** | ❌ No - only Tether can mint real USDT (test tokens for devnet only) |

---

## 🚀 Next Steps

1. **Document USDT acquisition** in your user guide
2. **Add balance check** before allowing deposits
3. **Consider DEX integration** for better UX (optional)
4. **Add test faucet** for devnet testing (optional)

---

## 🔗 Useful Links

- **Jupiter Swap**: https://jup.ag/
- **Raydium**: https://raydium.io/
- **Orca**: https://www.orca.so/
- **USDT on Solana**: https://solscan.io/token/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB
- **SPL Token Docs**: https://spl.solana.com/token

---

**Key Takeaway:** Your vault is a **storage system**, not a **token provider**. Users must acquire USDT through external means (DEX, transfers, bridges) before they can deposit it into your vault.
