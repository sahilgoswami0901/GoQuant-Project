/**
 * Collateral Vault - Devnet Tests
 * 
 * This test file is designed for DEVNET where airdrops are rate-limited.
 * It uses your existing funded wallet instead of creating new ones.
 * 
 * Prerequisites:
 * 1. Your wallet should have at least 1 SOL (check: solana balance)
 * 2. Program should be deployed to devnet (anchor deploy --provider.cluster devnet)
 * 
 * Run with:
 *   anchor test --provider.cluster devnet --skip-deploy
 */

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CollateralVault } from "../target/types/collateral_vault";
import {
    PublicKey,
    Keypair,
    SystemProgram,
    LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createMint,
    getAssociatedTokenAddressSync,
    getOrCreateAssociatedTokenAccount,
    mintTo,
    getAccount,
} from "@solana/spl-token";
import { expect } from "chai";
import * as fs from "fs";
import * as os from "os";

describe("collateral-vault-devnet", () => {
    // Configure the client
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.CollateralVault as Program<CollateralVault>;
    
    // Use the provider wallet (your funded wallet) as the test user
    // This avoids needing airdrops!
    const user = (provider.wallet as anchor.Wallet).payer;
    const admin = provider.wallet;

    // Test accounts
    let usdtMint: PublicKey;
    let userTokenAccount: PublicKey;
    let vaultPda: PublicKey;
    let vaultBump: number;
    let vaultTokenAccount: PublicKey;
    let vaultAuthorityPda: PublicKey;
    let vaultAuthorityBump: number;

    // Constants
    const USDT_DECIMALS = 6;
    const INITIAL_BALANCE = 1000 * Math.pow(10, USDT_DECIMALS); // 1000 USDT

    before(async () => {
        console.log("\n===========================================");
        console.log("  Collateral Vault - Devnet Test Suite");
        console.log("===========================================\n");

        // Check wallet balance
        const balance = await provider.connection.getBalance(user.publicKey);
        console.log("Wallet:", user.publicKey.toBase58());
        console.log("Balance:", balance / LAMPORTS_PER_SOL, "SOL");
        
        if (balance < 0.5 * LAMPORTS_PER_SOL) {
            throw new Error("Insufficient balance! Need at least 0.5 SOL. Get more from https://faucet.solana.com/");
        }

        console.log("\n--- Setting up test environment ---\n");

        // Create a test USDT mint (mock token)
        console.log("Creating mock USDT token...");
        usdtMint = await createMint(
            provider.connection,
            user,
            user.publicKey,  // Mint authority
            null,            // Freeze authority
            USDT_DECIMALS
        );
        console.log("✓ USDT Mint created:", usdtMint.toBase58());

        // Create user's token account
        console.log("Creating user token account...");
        const userAta = await getOrCreateAssociatedTokenAccount(
            provider.connection,
            user,
            usdtMint,
            user.publicKey
        );
        
        // Check if account exists, if not create it
        try {
            await getAccount(provider.connection, userAta.address);
        } catch {
            await getOrCreateAssociatedTokenAccount(
                provider.connection,
                user,
                usdtMint,
                user.publicKey
            );
        }
        console.log("✓ User token account:", userAta.address.toBase58());

        // Mint test USDT to user
        console.log("Minting 1000 test USDT...");
        await mintTo(
            provider.connection,
            user,
            usdtMint,
            userAta.address,
            user.publicKey,
            INITIAL_BALANCE
        );
        console.log("✓ Minted 1000 USDT to user");

        // Derive PDAs
        [vaultPda, vaultBump] = PublicKey.findProgramAddressSync(
            [Buffer.from("vault"), user.publicKey.toBuffer()],
            program.programId
        );
        console.log("✓ Vault PDA:", vaultPda.toBase58());

        [vaultAuthorityPda, vaultAuthorityBump] = PublicKey.findProgramAddressSync(
            [Buffer.from("vault_authority")],
            program.programId
        );
        console.log("✓ Vault Authority PDA:", vaultAuthorityPda.toBase58());

        // Get vault token account address
        vaultTokenAccount = getAssociatedTokenAddressSync(
            usdtMint,
            vaultPda,
            true  // allowOwnerOffCurve for PDAs
        );
        console.log("✓ Vault Token Account:", vaultTokenAccount.toBase58());

        console.log("\n--- Setup complete! ---\n");
    });

    // ==========================================
    // TEST 1: Initialize Vault Authority
    // ==========================================
    describe("1. Initialize Vault Authority", () => {
        it("should initialize vault authority", async () => {
            console.log("\nInitializing vault authority...");

            // Check if already initialized
            const existingAccount = await provider.connection.getAccountInfo(vaultAuthorityPda);
            if (existingAccount) {
                console.log("✓ Vault authority already initialized, skipping...");
                return;
            }

            const tx = await program.methods
                .initializeVaultAuthority()
                .accounts({
                    admin: admin.publicKey,
                    vaultAuthority: vaultAuthorityPda,
                    systemProgram: SystemProgram.programId,
                } as any)
                .rpc();

            console.log("✓ Transaction:", tx);
            console.log("  View on Explorer: https://explorer.solana.com/tx/" + tx + "?cluster=devnet");

            const authority = await program.account.vaultAuthority.fetch(vaultAuthorityPda);
            expect(authority.admin.toBase58()).to.equal(admin.publicKey.toBase58());
            console.log("✓ Vault authority initialized!");

            // Add authorized programs (Position Manager and Liquidation Engine)
            console.log("\nAdding authorized programs...");
            
            // Load position manager keypair if it exists
            const positionManagerKeypairPath = os.homedir() + '/.config/solana/position-manager.json';
            let positionManagerPubkey: PublicKey | null = null;
            
            if (fs.existsSync(positionManagerKeypairPath)) {
                try {
                    const keypairData = JSON.parse(fs.readFileSync(positionManagerKeypairPath, 'utf-8'));
                    const positionManagerKeypair = Keypair.fromSecretKey(
                        Uint8Array.from(keypairData)
                    );
                    positionManagerPubkey = positionManagerKeypair.publicKey;
                    console.log("✓ Found Position Manager:", positionManagerPubkey.toBase58());
                } catch (e) {
                    console.log("⚠️  Failed to load Position Manager keypair:", e);
                }
            } else {
                console.log("⚠️  Position Manager keypair not found at:", positionManagerKeypairPath);
                console.log("   Create it with: solana-keygen new --outfile ~/.config/solana/position-manager.json --no-bip39-passphrase");
            }

            // Load liquidation engine keypair if it exists
            const liquidationEngineKeypairPath = os.homedir() + '/.config/solana/liquidation-engine.json';
            let liquidationEnginePubkey: PublicKey | null = null;
            
            if (fs.existsSync(liquidationEngineKeypairPath)) {
                try {
                    const keypairData = JSON.parse(fs.readFileSync(liquidationEngineKeypairPath, 'utf-8'));
                    const liquidationEngineKeypair = Keypair.fromSecretKey(
                        Uint8Array.from(keypairData)
                    );
                    liquidationEnginePubkey = liquidationEngineKeypair.publicKey;
                    console.log("✓ Found Liquidation Engine:", liquidationEnginePubkey.toBase58());
                } catch (e) {
                    console.log("⚠️  Failed to load Liquidation Engine keypair:", e);
                }
            } else {
                console.log("⚠️  Liquidation Engine keypair not found at:", liquidationEngineKeypairPath);
                console.log("   Create it with: solana-keygen new --outfile ~/.config/solana/liquidation-engine.json --no-bip39-passphrase");
            }

            // Add position manager if keypair exists
            if (positionManagerPubkey) {
                try {
                    // Check if already authorized
                    const currentAuthority = await program.account.vaultAuthority.fetch(vaultAuthorityPda);
                    const isAlreadyAuthorized = currentAuthority.authorizedPrograms.some(
                        p => p.toBase58() === positionManagerPubkey!.toBase58()
                    );
                    
                    if (isAlreadyAuthorized) {
                        console.log("✓ Position Manager already authorized");
                    } else {
                        const addPosMgrTx = await program.methods
                            .addAuthorizedProgram(positionManagerPubkey)
                            .accounts({
                                admin: admin.publicKey,
                                vaultAuthority: vaultAuthorityPda,
                            } as any)
                            .rpc();
                        
                        console.log("✓ Added Position Manager as authorized program");
                        console.log("  Transaction:", addPosMgrTx);
                        console.log("  View on Explorer: https://explorer.solana.com/tx/" + addPosMgrTx + "?cluster=devnet");
                    }
                } catch (e: any) {
                    console.log("⚠️  Failed to add Position Manager:", e.message || e);
                }
            }

            // Add liquidation engine if keypair exists
            if (liquidationEnginePubkey) {
                try {
                    // Check if already authorized
                    const currentAuthority = await program.account.vaultAuthority.fetch(vaultAuthorityPda);
                    const isAlreadyAuthorized = currentAuthority.authorizedPrograms.some(
                        p => p.toBase58() === liquidationEnginePubkey!.toBase58()
                    );
                    
                    if (isAlreadyAuthorized) {
                        console.log("✓ Liquidation Engine already authorized");
                    } else {
                        const addLiqEngTx = await program.methods
                            .addAuthorizedProgram(liquidationEnginePubkey)
                            .accounts({
                                admin: admin.publicKey,
                                vaultAuthority: vaultAuthorityPda,
                            } as any)
                            .rpc();
                        
                        console.log("✓ Added Liquidation Engine as authorized program");
                        console.log("  Transaction:", addLiqEngTx);
                        console.log("  View on Explorer: https://explorer.solana.com/tx/" + addLiqEngTx + "?cluster=devnet");
                    }
                } catch (e: any) {
                    console.log("⚠️  Failed to add Liquidation Engine:", e.message || e);
                }
            }

            // Display final authorized programs list
            const finalAuthority = await program.account.vaultAuthority.fetch(vaultAuthorityPda);
            console.log("\n📋 Authorized Programs List:");
            if (finalAuthority.authorizedPrograms.length === 0) {
                console.log("  (none)");
                console.log("\n💡 Tip: Create the keypairs first, then re-run this test:");
                console.log("   solana-keygen new --outfile ~/.config/solana/position-manager.json --no-bip39-passphrase");
                console.log("   solana-keygen new --outfile ~/.config/solana/liquidation-engine.json --no-bip39-passphrase");
            } else {
                finalAuthority.authorizedPrograms.forEach((prog, idx) => {
                    console.log(`  ${idx + 1}. ${prog.toBase58()}`);
                });
            }
        });
    });

//     // ==========================================
//     // TEST 2: Initialize User Vault
//     // ==========================================
    describe("2. Initialize User Vault", () => {
        it("should create a vault for the user", async () => {
            console.log("\nInitializing user vault...");

            // Check if already initialized
            const existingAccount = await provider.connection.getAccountInfo(vaultPda);
            if (existingAccount) {
                console.log("✓ User vault already initialized, skipping...");
                return;
            }

            const tx = await program.methods
                .initializeVault()
                .accounts({
                    user: user.publicKey,
                    vault: vaultPda,
                    usdtMint: usdtMint,
                    vaultTokenAccount: vaultTokenAccount,
                    systemProgram: SystemProgram.programId,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                } as any)
                .rpc();

            console.log("✓ Transaction:", tx);
            console.log("  View on Explorer: https://explorer.solana.com/tx/" + tx + "?cluster=devnet");

            const vault = await program.account.collateralVault.fetch(vaultPda);
            expect(vault.owner.toBase58()).to.equal(user.publicKey.toBase58());
            expect(vault.totalBalance.toNumber()).to.equal(0);
            console.log("✓ User vault created!");
        });
    });

//     // ==========================================
//     // TEST 3: Deposit USDT
//     // ==========================================
//     describe("3. Deposit Collateral", () => {
//         const depositAmount = 100 * Math.pow(10, USDT_DECIMALS); // 100 USDT

//         it("should deposit 100 USDT into vault", async () => {
//             console.log("\nDepositing 100 USDT...");

//             const balanceBefore = await provider.connection.getTokenAccountBalance(userTokenAccount);
//             console.log("  User balance before:", Number(balanceBefore.value.amount) / 1_000_000, "USDT");

//             const tx = await program.methods
//                 .deposit(new anchor.BN(depositAmount))
//                 .accounts({
//                     user: user.publicKey,
//                     vault: vaultPda,
//                     userTokenAccount: userTokenAccount,
//                     vaultTokenAccount: vaultTokenAccount,
//                     vaultAuthority: vaultAuthorityPda,
//                     tokenProgram: TOKEN_PROGRAM_ID,
//                 } as any)
//                 .rpc();

//             console.log("✓ Transaction:", tx);
//             console.log("  View on Explorer: https://explorer.solana.com/tx/" + tx + "?cluster=devnet");

//             const vault = await program.account.collateralVault.fetch(vaultPda);
//             const balanceAfter = await provider.connection.getTokenAccountBalance(userTokenAccount);

//             console.log("  User balance after:", Number(balanceAfter.value.amount) / 1_000_000, "USDT");
//             console.log("  Vault balance:", vault.totalBalance.toNumber() / 1_000_000, "USDT");

//             expect(vault.totalBalance.toNumber()).to.equal(depositAmount);
//             console.log("✓ Deposit successful!");
//         });
//     });

//     // ==========================================
//     // TEST 4: Withdraw USDT
//     // ==========================================
//     describe("4. Withdraw Collateral", () => {
//         const withdrawAmount = 50 * Math.pow(10, USDT_DECIMALS); // 50 USDT

//         it("should withdraw 50 USDT from vault", async () => {
//             console.log("\nWithdrawing 50 USDT...");

//             const vaultBefore = await program.account.collateralVault.fetch(vaultPda);
//             console.log("  Vault balance before:", vaultBefore.totalBalance.toNumber() / 1_000_000, "USDT");

//             const tx = await program.methods
//                 .withdraw(new anchor.BN(withdrawAmount))
//                 .accounts({
//                     user: user.publicKey,
//                     vault: vaultPda,
//                     userTokenAccount: userTokenAccount,
//                     vaultTokenAccount: vaultTokenAccount,
//                     vaultAuthority: vaultAuthorityPda,
//                     tokenProgram: TOKEN_PROGRAM_ID,
//                 } as any)
//                 .rpc();

//             console.log("✓ Transaction:", tx);
//             console.log("  View on Explorer: https://explorer.solana.com/tx/" + tx + "?cluster=devnet");

//             const vaultAfter = await program.account.collateralVault.fetch(vaultPda);
//             console.log("  Vault balance after:", vaultAfter.totalBalance.toNumber() / 1_000_000, "USDT");

//             expect(vaultAfter.totalBalance.toNumber()).to.equal(
//                 vaultBefore.totalBalance.toNumber() - withdrawAmount
//             );
//             console.log("✓ Withdrawal successful!");
//         });
//     });

//     // ==========================================
//     // TEST 5: Lock Collateral
//     // ==========================================
//     describe("5. Lock Collateral", () => {
//         const lockAmount = 20 * Math.pow(10, USDT_DECIMALS); // 20 USDT

//         it("should lock 20 USDT as margin", async () => {
//             console.log("\nLocking 20 USDT as collateral...");

//             const vaultBefore = await program.account.collateralVault.fetch(vaultPda);
//             console.log("  Available before:", vaultBefore.availableBalance.toNumber() / 1_000_000, "USDT");
//             console.log("  Locked before:", vaultBefore.lockedBalance.toNumber() / 1_000_000, "USDT");

//             const tx = await program.methods
//                 .lockCollateral(new anchor.BN(lockAmount))
//                 .accounts({
//                     authority: admin.publicKey,
//                     vault: vaultPda,
//                     vaultAuthority: vaultAuthorityPda,
//                 } as any)
//                 .rpc();

//             console.log("✓ Transaction:", tx);
//             console.log("  View on Explorer: https://explorer.solana.com/tx/" + tx + "?cluster=devnet");

//             const vaultAfter = await program.account.collateralVault.fetch(vaultPda);
//             console.log("  Available after:", vaultAfter.availableBalance.toNumber() / 1_000_000, "USDT");
//             console.log("  Locked after:", vaultAfter.lockedBalance.toNumber() / 1_000_000, "USDT");

//             expect(vaultAfter.lockedBalance.toNumber()).to.equal(
//                 vaultBefore.lockedBalance.toNumber() + lockAmount
//             );
//             console.log("✓ Lock successful!");
//         });
//     });

//     // ==========================================
//     // TEST 6: Unlock Collateral
//     // ==========================================
//     describe("6. Unlock Collateral", () => {
//         const unlockAmount = 20 * Math.pow(10, USDT_DECIMALS); // 20 USDT

//         it("should unlock 20 USDT", async () => {
//             console.log("\nUnlocking 20 USDT...");

//             const vaultBefore = await program.account.collateralVault.fetch(vaultPda);
//             console.log("  Available before:", vaultBefore.availableBalance.toNumber() / 1_000_000, "USDT");
//             console.log("  Locked before:", vaultBefore.lockedBalance.toNumber() / 1_000_000, "USDT");

//             const tx = await program.methods
//                 .unlockCollateral(new anchor.BN(unlockAmount))
//                 .accounts({
//                     authority: admin.publicKey,
//                     vault: vaultPda,
//                     vaultAuthority: vaultAuthorityPda,
//                 } as any)
//                 .rpc();

//             console.log("✓ Transaction:", tx);
//             console.log("  View on Explorer: https://explorer.solana.com/tx/" + tx + "?cluster=devnet");

//             const vaultAfter = await program.account.collateralVault.fetch(vaultPda);
//             console.log("  Available after:", vaultAfter.availableBalance.toNumber() / 1_000_000, "USDT");
//             console.log("  Locked after:", vaultAfter.lockedBalance.toNumber() / 1_000_000, "USDT");

//             expect(vaultAfter.lockedBalance.toNumber()).to.equal(
//                 vaultBefore.lockedBalance.toNumber() - unlockAmount
//             );
//             console.log("✓ Unlock successful!");
//         });
//     });

//     // ==========================================
//     // TEST 7: Final State Check
//     // ==========================================
//     describe("7. Final State", () => {
//         it("should show final vault state", async () => {
//             console.log("\n===========================================");
//             console.log("           FINAL VAULT STATE");
//             console.log("===========================================\n");

//             const vault = await program.account.collateralVault.fetch(vaultPda);
//             const userBalance = await provider.connection.getTokenAccountBalance(userTokenAccount);

//             console.log("Vault Address:", vaultPda.toBase58());
//             console.log("Owner:", vault.owner.toBase58());
//             console.log("");
//             console.log("Balances:");
//             console.log("  Total Balance:", vault.totalBalance.toNumber() / 1_000_000, "USDT");
//             console.log("  Locked Balance:", vault.lockedBalance.toNumber() / 1_000_000, "USDT");
//             console.log("  Available Balance:", vault.availableBalance.toNumber() / 1_000_000, "USDT");
//             console.log("");
//             console.log("Lifetime Stats:");
//             console.log("  Total Deposited:", vault.totalDeposited.toNumber() / 1_000_000, "USDT");
//             console.log("  Total Withdrawn:", vault.totalWithdrawn.toNumber() / 1_000_000, "USDT");
//             console.log("");
//             console.log("User's Remaining USDT:", Number(userBalance.value.amount) / 1_000_000, "USDT");
//             console.log("");
//             console.log("View on Explorer:");
//             console.log("  Vault: https://explorer.solana.com/address/" + vaultPda.toBase58() + "?cluster=devnet");
//             console.log("  Program: https://explorer.solana.com/address/" + program.programId.toBase58() + "?cluster=devnet");
//             console.log("\n===========================================\n");
//         });
//     });
});

