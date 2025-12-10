# Collateral Vault Management System

A Solana smart contract (Anchor program) for managing user collateral in a decentralized perpetual futures exchange.

## 📋 Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Key Concepts](#key-concepts)
4. [Project Structure](#project-structure)
5. [Account Structures](#account-structures)
6. [Instructions Reference](#instructions-reference)
7. [PDA Derivation](#pda-derivation)
8. [Security Model](#security-model)
9. [Events](#events)
10. [Error Handling](#error-handling)
11. [Setup & Installation](#setup--installation)
12. [Testing](#testing)
13. [Deployment](#deployment)

---

## Overview

### What is this?

This is the **custody layer** for a perpetual futures DEX on Solana. It manages:

- **User Deposits**: Users deposit USDT collateral into secure vaults
- **Balance Tracking**: Tracks total, locked, and available balances
- **Position Margin**: Locks collateral when users open trading positions
- **Settlements**: Transfers funds between users for trade settlement
- **Withdrawals**: Allows users to withdraw available funds

### Why is it needed?

In a perpetual futures exchange:

```
User deposits $1000 USDT
    ↓
Opens 10x leveraged $5000 position
    ↓
System locks $500 as margin
    ↓
Position moves in profit (+$200)
    ↓
User closes position
    ↓
$500 unlocked + $200 profit = $700 available
    ↓
User can withdraw $1700 total
```

This program manages all of the above securely on-chain.

---

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           CLIENT (Web/Mobile)                            │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         BACKEND SERVICE (Rust)                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │   Vault     │  │  Balance    │  │ Transaction │  │    Vault    │    │
│  │  Manager    │  │  Tracker    │  │   Builder   │  │   Monitor   │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            SOLANA BLOCKCHAIN                             │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                    COLLATERAL VAULT PROGRAM                         │ │
│  │                                                                      │ │
│  │   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐              │ │
│  │   │ User Vault  │   │ User Vault  │   │ User Vault  │    ...       │ │
│  │   │   (PDA)     │   │   (PDA)     │   │   (PDA)     │              │ │
│  │   └─────────────┘   └─────────────┘   └─────────────┘              │ │
│  │                                                                      │ │
│  │   ┌──────────────────────────────────────────────────────────────┐ │ │
│  │   │                    Vault Authority (PDA)                      │ │ │
│  │   │  • Admin control  • Authorized programs  • Pause state       │ │ │
│  │   └──────────────────────────────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                    │                                     │
│                                    │ CPI                                 │
│                                    ▼                                     │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                       SPL TOKEN PROGRAM                             │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Interactions

```
Position Manager Program              Vault Program                 SPL Token Program
        │                                  │                              │
        │ 1. User opens position           │                              │
        │──────────────────────────────────>                              │
        │   lock_collateral(100 USDT)      │                              │
        │                                  │                              │
        │                                  │ 2. Validate & update state   │
        │                                  │                              │
        │<─────────────────────────────────│                              │
        │   Success                        │                              │
        │                                  │                              │
        │ 3. User closes position          │                              │
        │──────────────────────────────────>                              │
        │   unlock_collateral(100 USDT)    │                              │
        │                                  │                              │
        │                                  │ 4. Update state              │
        │<─────────────────────────────────│                              │
        │   Success                        │                              │
```

---

## Key Concepts

### 1. Program Derived Addresses (PDAs)

PDAs are special Solana addresses that:
- Have no private key
- Can only be "signed" by the program
- Are derived deterministically from seeds

```rust
// Vault PDA derivation
seeds = ["vault", user_public_key]
bump = 255, 254, 253... (until valid PDA found)
```

**Why PDAs?**
- Trustless: No one can steal funds, even developers
- Deterministic: Anyone can calculate vault address from user's pubkey
- Secure: Only the program can authorize vault transactions

### 2. SPL Token Program

The standard token program on Solana. Our program uses it via CPI (Cross-Program Invocation):

```rust
// Deposit: User → Vault
token::transfer(
    CpiContext::new(...),
    Transfer {
        from: user_token_account,
        to: vault_token_account,
        authority: user,  // User signs
    }
)?;

// Withdraw: Vault → User
token::transfer(
    CpiContext::new_with_signer(..., signer_seeds),  // PDA signs
    Transfer {
        from: vault_token_account,
        to: user_token_account,
        authority: vault_pda,  // Program provides seeds
    }
)?;
```

### 3. Balance Types

| Balance Type | Description | Can Withdraw? |
|--------------|-------------|---------------|
| `total_balance` | All USDT in vault | - |
| `locked_balance` | Reserved for positions | ❌ |
| `available_balance` | Free to use | ✅ |

**Invariant**: `total_balance = locked_balance + available_balance`

### 4. Cross-Program Invocation (CPI)

When one Solana program calls another:

```
Your Vault Program                    SPL Token Program
       │                                     │
       │  CPI: transfer(100 USDT)            │
       │─────────────────────────────────────>
       │                                     │
       │  Result: Ok(())                     │
       │<─────────────────────────────────────
```

---

## Project Structure

```
collateral-vault/
├── Anchor.toml                 # Anchor configuration
├── Cargo.toml                  # Rust workspace config
├── programs/
│   └── collateral-vault/
│       ├── Cargo.toml          # Program dependencies
│       └── src/
│           ├── lib.rs          # Main program entry point
│           ├── errors/
│           │   └── mod.rs      # Custom error definitions
│           ├── events/
│           │   └── mod.rs      # Event definitions for indexing
│           ├── instructions/
│           │   ├── mod.rs      # Instruction exports
│           │   ├── initialize_vault.rs
│           │   ├── initialize_vault_authority.rs
│           │   ├── deposit.rs
│           │   ├── withdraw.rs
│           │   ├── lock_collateral.rs
│           │   ├── unlock_collateral.rs
│           │   └── transfer_collateral.rs
│           └── state/
│               ├── mod.rs      # State exports
│               ├── vault.rs    # CollateralVault account
│               └── vault_authority.rs  # VaultAuthority account
├── tests/                      # Integration tests
└── README.md                   # This file
```

---

## Account Structures

### CollateralVault

The main vault account for each user.

```rust
#[account]
pub struct CollateralVault {
    pub owner: Pubkey,              // 32 bytes - Vault owner
    pub token_account: Pubkey,      // 32 bytes - USDT token account
    pub total_balance: u64,         // 8 bytes  - Total USDT
    pub locked_balance: u64,        // 8 bytes  - Locked for positions
    pub available_balance: u64,     // 8 bytes  - Free to withdraw
    pub total_deposited: u64,       // 8 bytes  - Lifetime deposits
    pub total_withdrawn: u64,       // 8 bytes  - Lifetime withdrawals
    pub created_at: i64,            // 8 bytes  - Creation timestamp
    pub bump: u8,                   // 1 byte   - PDA bump
}
// Total: 8 (discriminator) + 121 = 129 bytes
```

**PDA Seeds**: `["vault", user_pubkey]`

### VaultAuthority

Global configuration for the program.

```rust
#[account]
pub struct VaultAuthority {
    pub admin: Pubkey,                      // 32 bytes
    pub authorized_programs: Vec<Pubkey>,   // 4 + (32 * 10) bytes max
    pub bump: u8,                           // 1 byte
    pub is_paused: bool,                    // 1 byte
    pub last_updated: i64,                  // 8 bytes
}
```

**PDA Seeds**: `["vault_authority"]` (singleton)

---

## Instructions Reference

### 1. initialize_vault

Creates a new vault for a user.

```typescript
await program.methods
    .initializeVault()
    .accounts({
        user: wallet.publicKey,
        vault: vaultPda,
        usdtMint: USDT_MINT,
        vaultTokenAccount: vaultTokenAccountPda,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    })
    .rpc();
```

### 2. deposit

Deposits USDT into the vault.

```typescript
const amount = new BN(100 * 1_000_000); // 100 USDT

await program.methods
    .deposit(amount)
    .accounts({
        user: wallet.publicKey,
        vault: vaultPda,
        userTokenAccount: userUsdtAccount,
        vaultTokenAccount: vaultTokenAccountPda,
        vaultAuthority: vaultAuthorityPda,
        tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();
```

### 3. withdraw

Withdraws USDT from the vault.

```typescript
const amount = new BN(50 * 1_000_000); // 50 USDT

await program.methods
    .withdraw(amount)
    .accounts({
        user: wallet.publicKey,
        vault: vaultPda,
        userTokenAccount: userUsdtAccount,
        vaultTokenAccount: vaultTokenAccountPda,
        vaultAuthority: vaultAuthorityPda,
        tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();
```

### 4. lock_collateral

Locks collateral for trading (called by authorized programs).

```typescript
await program.methods
    .lockCollateral(new BN(100 * 1_000_000))
    .accounts({
        authority: positionManagerPda,
        vault: userVaultPda,
        vaultAuthority: vaultAuthorityPda,
    })
    .rpc();
```

### 5. unlock_collateral

Unlocks previously locked collateral.

```typescript
await program.methods
    .unlockCollateral(new BN(100 * 1_000_000))
    .accounts({
        authority: positionManagerPda,
        vault: userVaultPda,
        vaultAuthority: vaultAuthorityPda,
    })
    .rpc();
```

### 6. transfer_collateral

Transfers between vaults (settlements/liquidations).

```typescript
await program.methods
    .transferCollateral(
        new BN(50 * 1_000_000),
        { settlement: {} }  // TransferReason enum
    )
    .accounts({
        authority: settlementRelayerPda,
        fromVault: loserVaultPda,
        fromTokenAccount: loserTokenAccountPda,
        toVault: winnerVaultPda,
        toTokenAccount: winnerTokenAccountPda,
        vaultAuthority: vaultAuthorityPda,
        tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();
```

---

## PDA Derivation

### Vault PDA

```typescript
const [vaultPda, vaultBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), userPublicKey.toBuffer()],
    PROGRAM_ID
);
```

### Vault Authority PDA

```typescript
const [vaultAuthorityPda, authorityBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority")],
    PROGRAM_ID
);
```

### Vault Token Account

```typescript
const vaultTokenAccount = getAssociatedTokenAddressSync(
    USDT_MINT,
    vaultPda,
    true  // allowOwnerOffCurve = true for PDAs
);
```

---

## Security Model

### 1. Access Control

| Action | Who Can Do It |
|--------|---------------|
| Create vault | Any user (for themselves) |
| Deposit | Vault owner only |
| Withdraw | Vault owner only |
| Lock/Unlock | Authorized programs only |
| Transfer | Authorized programs only |
| Add/Remove programs | Admin only |
| Pause system | Admin only |

### 2. Balance Protection

```rust
// Cannot withdraw locked funds
require!(
    vault.available_balance >= amount,
    VaultError::InsufficientBalance
);

// Cannot unlock more than locked
require!(
    vault.locked_balance >= amount,
    VaultError::InsufficientLockedBalance
);
```

### 3. Overflow Prevention

```rust
// All arithmetic uses checked operations
vault.total_balance = vault
    .total_balance
    .checked_add(amount)
    .ok_or(VaultError::Overflow)?;
```

### 4. PDA Security

- No private keys exist for vault PDAs
- Only the program can sign for PDAs
- Funds cannot be stolen even if backend is compromised

---

## Events

Events are emitted for off-chain indexing and real-time updates.

### DepositEvent
```rust
#[event]
pub struct DepositEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}
```

### WithdrawEvent
```rust
#[event]
pub struct WithdrawEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub remaining_balance: u64,
    pub timestamp: i64,
}
```

### LockCollateralEvent
```rust
#[event]
pub struct LockCollateralEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub new_locked_balance: u64,
    pub new_available_balance: u64,
    pub locked_by: Pubkey,
    pub timestamp: i64,
}
```

### Listening to Events (TypeScript)

```typescript
program.addEventListener("DepositEvent", (event, slot) => {
    console.log(`Deposit: ${event.amount} to ${event.vault}`);
    // Update database, send notification, etc.
});
```

---

## Error Handling

### Error Codes

| Code | Name | Description |
|------|------|-------------|
| 6000 | InvalidAmount | Amount must be > 0 |
| 6001 | BelowMinimumDeposit | Below minimum deposit |
| 6002 | InvalidTokenMint | Wrong token type |
| 6010 | InsufficientBalance | Not enough available |
| 6011 | HasOpenPositions | Cannot withdraw with positions |
| 6012 | InsufficientLockedBalance | Cannot unlock that much |
| 6020 | Unauthorized | Not vault owner |
| 6021 | UnauthorizedProgram | Program not in whitelist |
| 6022 | NotAdmin | Only admin can do this |
| 6030 | VaultAlreadyExists | Vault already initialized |
| 6031 | VaultNotFound | Vault doesn't exist |
| 6032 | VaultPaused | System is paused |
| 6040 | Overflow | Arithmetic overflow |
| 6041 | Underflow | Arithmetic underflow |

---

## Setup & Installation

### Prerequisites

```bash
# Rust 1.75+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"

# Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install 0.29.0
avm use 0.29.0

# Node.js (for testing)
nvm install 18
nvm use 18
```

### Build

```bash
cd collateral-vault
anchor build
```

### Test

```bash
anchor test
```

### Deploy

```bash
# Configure for devnet
solana config set --url devnet

# Get some devnet SOL
solana airdrop 2

# Deploy
anchor deploy
```

---

## Testing

### Unit Tests

Located in `programs/collateral-vault/src/` as Rust doc tests.

### Integration Tests

Located in `tests/`:

```typescript
describe("collateral-vault", () => {
    it("initializes vault", async () => {
        await program.methods.initializeVault().rpc();
        const vault = await program.account.collateralVault.fetch(vaultPda);
        expect(vault.owner.toString()).to.equal(user.publicKey.toString());
    });

    it("deposits USDT", async () => {
        const amount = new BN(100 * 1_000_000);
        await program.methods.deposit(amount).rpc();
        const vault = await program.account.collateralVault.fetch(vaultPda);
        expect(vault.totalBalance.toNumber()).to.equal(amount.toNumber());
    });

    it("prevents unauthorized withdrawal", async () => {
        try {
            await program.methods.withdraw(amount)
                .accounts({ user: attacker.publicKey })
                .signers([attacker])
                .rpc();
            expect.fail("Should have thrown");
        } catch (e) {
            expect(e.message).to.include("Unauthorized");
        }
    });
});
```

---

## Deployment

### 1. Build for Production

```bash
anchor build --verifiable
```

### 2. Deploy to Devnet

```bash
anchor deploy --provider.cluster devnet
```

### 3. Initialize VaultAuthority

```typescript
const [vaultAuthorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority")],
    PROGRAM_ID
);

await program.methods
    .initializeVaultAuthority()
    .accounts({
        admin: adminKeypair.publicKey,
        vaultAuthority: vaultAuthorityPda,
        systemProgram: SystemProgram.programId,
    })
    .signers([adminKeypair])
    .rpc();
```

### 4. Add Authorized Programs

```typescript
await program.methods
    .addAuthorizedProgram(POSITION_MANAGER_PROGRAM_ID)
    .accounts({
        admin: adminKeypair.publicKey,
        vaultAuthority: vaultAuthorityPda,
    })
    .signers([adminKeypair])
    .rpc();
```

---

## License

Confidential - GoQuant Assignment

---

## Author

Built as part of the GoQuant recruitment process.

