/**
 * Collateral Vault Integration Tests
 * 
 * These tests verify the core functionality of the vault program:
 * 1. Vault initialization
 * 2. Deposits
 * 3. Withdrawals
 * 4. Lock/Unlock collateral
 * 5. Transfer between vaults
 * 6. Security checks
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
} from "@solana/spl-token";
import { expect } from "chai";

describe("collateral-vault", () => {
    // Configure the client to use the local cluster
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.CollateralVault as Program<CollateralVault>;
    
    // Test accounts
    let usdtMint: PublicKey;
    let user: Keypair;
    let userTokenAccount: PublicKey;
    let vaultPda: PublicKey;
    let vaultBump: number;
    let vaultTokenAccount: PublicKey;
    let vaultAuthorityPda: PublicKey;
    let vaultAuthorityBump: number;

    // Second user for transfer tests
    let user2: Keypair;
    let user2TokenAccount: PublicKey;
    let vault2Pda: PublicKey;
    let vault2TokenAccount: PublicKey;

    // Admin for vault authority
    const admin = provider.wallet;

    // Constants
    const USDT_DECIMALS = 6;
    const INITIAL_BALANCE = 1000 * Math.pow(10, USDT_DECIMALS); // 1000 USDT

    before(async () => {
        // Create test user keypairs
        user = Keypair.generate();
        user2 = Keypair.generate();

        // Airdrop SOL to users for transaction fees
        const airdropTx1 = await provider.connection.requestAirdrop(
            user.publicKey,
            2 * LAMPORTS_PER_SOL
        );
        await provider.connection.confirmTransaction(airdropTx1);

        const airdropTx2 = await provider.connection.requestAirdrop(
            user2.publicKey,
            2 * LAMPORTS_PER_SOL
        );
        await provider.connection.confirmTransaction(airdropTx2);

        // Create USDT mock mint
        usdtMint = await createMint(
            provider.connection,
            (provider.wallet as anchor.Wallet).payer,
            admin.publicKey,  // Mint authority
            null,             // Freeze authority
            USDT_DECIMALS
        );

        console.log("USDT Mint:", usdtMint.toBase58());

        // Create user token accounts
        const userAta = await getOrCreateAssociatedTokenAccount(
            provider.connection,
            (provider.wallet as anchor.Wallet).payer,
            usdtMint,
            user.publicKey
        );
        userTokenAccount = userAta.address;

        const user2Ata = await getOrCreateAssociatedTokenAccount(
            provider.connection,
            (provider.wallet as anchor.Wallet).payer,
            usdtMint,
            user2.publicKey
        );
        user2TokenAccount = user2Ata.address;

        // Mint USDT to users
        await mintTo(
            provider.connection,
            (provider.wallet as anchor.Wallet).payer,
            usdtMint,
            userTokenAccount,
            admin.publicKey,
            INITIAL_BALANCE
        );

        await mintTo(
            provider.connection,
            (provider.wallet as anchor.Wallet).payer,
            usdtMint,
            user2TokenAccount,
            admin.publicKey,
            INITIAL_BALANCE
        );

        // Derive PDAs
        [vaultPda, vaultBump] = PublicKey.findProgramAddressSync(
            [Buffer.from("vault"), user.publicKey.toBuffer()],
            program.programId
        );

        [vault2Pda] = PublicKey.findProgramAddressSync(
            [Buffer.from("vault"), user2.publicKey.toBuffer()],
            program.programId
        );

        [vaultAuthorityPda, vaultAuthorityBump] = PublicKey.findProgramAddressSync(
            [Buffer.from("vault_authority")],
            program.programId
        );

        // Get vault token account addresses
        vaultTokenAccount = getAssociatedTokenAddressSync(
            usdtMint,
            vaultPda,
            true  // allowOwnerOffCurve for PDAs
        );

        vault2TokenAccount = getAssociatedTokenAddressSync(
            usdtMint,
            vault2Pda,
            true
        );

        console.log("User:", user.publicKey.toBase58());
        console.log("Vault PDA:", vaultPda.toBase58());
        console.log("Vault Authority:", vaultAuthorityPda.toBase58());
    });

    // ==========================================
    // INITIALIZATION TESTS
    // ==========================================

    describe("Initialization", () => {
        it("initializes vault authority", async () => {
            await program.methods
                .initializeVaultAuthority()
                .accounts({
                    admin: admin.publicKey,
                    vaultAuthority: vaultAuthorityPda,
                    systemProgram: SystemProgram.programId,
                } as any)
                .rpc();

            const authority = await program.account.vaultAuthority.fetch(vaultAuthorityPda);
            expect(authority.admin.toBase58()).to.equal(admin.publicKey.toBase58());
            expect(authority.isPaused).to.equal(false);
            expect(authority.authorizedPrograms.length).to.equal(0);
        });

        it("initializes user vault", async () => {
            await program.methods
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
                .signers([user])
                .rpc();

            const vault = await program.account.collateralVault.fetch(vaultPda);
            expect(vault.owner.toBase58()).to.equal(user.publicKey.toBase58());
            expect(vault.totalBalance.toNumber()).to.equal(0);
            expect(vault.lockedBalance.toNumber()).to.equal(0);
            expect(vault.availableBalance.toNumber()).to.equal(0);
        });

        it("initializes second user vault", async () => {
            await program.methods
                .initializeVault()
                .accounts({
                    user: user2.publicKey,
                    vault: vault2Pda,
                    usdtMint: usdtMint,
                    vaultTokenAccount: vault2TokenAccount,
                    systemProgram: SystemProgram.programId,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                } as any)
                .signers([user2])
                .rpc();

            const vault = await program.account.collateralVault.fetch(vault2Pda);
            expect(vault.owner.toBase58()).to.equal(user2.publicKey.toBase58());
        });

        it("prevents duplicate vault initialization", async () => {
            try {
                await program.methods
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
                    .signers([user])
                    .rpc();
                expect.fail("Should have thrown error");
            } catch (e) {
                // Expected: account already exists
                expect(e).to.exist;
            }
        });
    });

    // ==========================================
    // DEPOSIT TESTS
    // ==========================================

    describe("Deposits", () => {
        const depositAmount = 100 * Math.pow(10, USDT_DECIMALS); // 100 USDT

        it("deposits USDT successfully", async () => {
            const balanceBefore = await provider.connection.getTokenAccountBalance(userTokenAccount);
            
            await program.methods
                .deposit(new anchor.BN(depositAmount))
                .accounts({
                    user: user.publicKey,
                    vault: vaultPda,
                    userTokenAccount: userTokenAccount,
                    vaultTokenAccount: vaultTokenAccount,
                    vaultAuthority: vaultAuthorityPda,
                    tokenProgram: TOKEN_PROGRAM_ID,
                } as any)
                .signers([user])
                .rpc();

            const vault = await program.account.collateralVault.fetch(vaultPda);
            const balanceAfter = await provider.connection.getTokenAccountBalance(userTokenAccount);

            expect(vault.totalBalance.toNumber()).to.equal(depositAmount);
            expect(vault.availableBalance.toNumber()).to.equal(depositAmount);
            expect(vault.lockedBalance.toNumber()).to.equal(0);
            expect(
                Number(balanceBefore.value.amount) - Number(balanceAfter.value.amount)
            ).to.equal(depositAmount);
        });

        it("rejects zero deposit", async () => {
            try {
                await program.methods
                    .deposit(new anchor.BN(0))
                    .accounts({
                        user: user.publicKey,
                        vault: vaultPda,
                        userTokenAccount: userTokenAccount,
                        vaultTokenAccount: vaultTokenAccount,
                        vaultAuthority: vaultAuthorityPda,
                        tokenProgram: TOKEN_PROGRAM_ID,
                    } as any)
                    .signers([user])
                    .rpc();
                expect.fail("Should have thrown");
            } catch (e: any) {
                expect(e.message).to.include("InvalidAmount");
            }
        });

        it("rejects deposit when paused", async () => {
            // Pause the system
            await program.methods
                .setPaused(true)
                .accounts({
                    admin: admin.publicKey,
                    vaultAuthority: vaultAuthorityPda,
                } as any)
                .rpc();

            try {
                await program.methods
                    .deposit(new anchor.BN(depositAmount))
                    .accounts({
                        user: user.publicKey,
                        vault: vaultPda,
                        userTokenAccount: userTokenAccount,
                        vaultTokenAccount: vaultTokenAccount,
                        vaultAuthority: vaultAuthorityPda,
                        tokenProgram: TOKEN_PROGRAM_ID,
                    } as any)
                    .signers([user])
                    .rpc();
                expect.fail("Should have thrown");
            } catch (e: any) {
                expect(e.message).to.include("VaultPaused");
            }

            // Unpause for remaining tests
            await program.methods
                .setPaused(false)
                .accounts({
                    admin: admin.publicKey,
                    vaultAuthority: vaultAuthorityPda,
                } as any)
                .rpc();
        });
    });

    // ==========================================
    // WITHDRAWAL TESTS
    // ==========================================

    describe("Withdrawals", () => {
        const withdrawAmount = 50 * Math.pow(10, USDT_DECIMALS); // 50 USDT

        it("withdraws USDT successfully", async () => {
            const vaultBefore = await program.account.collateralVault.fetch(vaultPda);
            
            await program.methods
                .withdraw(new anchor.BN(withdrawAmount))
                .accounts({
                    user: user.publicKey,
                    vault: vaultPda,
                    userTokenAccount: userTokenAccount,
                    vaultTokenAccount: vaultTokenAccount,
                    vaultAuthority: vaultAuthorityPda,
                    tokenProgram: TOKEN_PROGRAM_ID,
                } as any)
                .signers([user])
                .rpc();

            const vaultAfter = await program.account.collateralVault.fetch(vaultPda);

            expect(vaultAfter.totalBalance.toNumber()).to.equal(
                vaultBefore.totalBalance.toNumber() - withdrawAmount
            );
            expect(vaultAfter.availableBalance.toNumber()).to.equal(
                vaultBefore.availableBalance.toNumber() - withdrawAmount
            );
        });

        it("rejects withdrawal exceeding available balance", async () => {
            const vault = await program.account.collateralVault.fetch(vaultPda);
            const tooMuch = vault.availableBalance.toNumber() + 1;

            try {
                await program.methods
                    .withdraw(new anchor.BN(tooMuch))
                    .accounts({
                        user: user.publicKey,
                        vault: vaultPda,
                        userTokenAccount: userTokenAccount,
                        vaultTokenAccount: vaultTokenAccount,
                        vaultAuthority: vaultAuthorityPda,
                        tokenProgram: TOKEN_PROGRAM_ID,
                    } as any)
                    .signers([user])
                    .rpc();
                expect.fail("Should have thrown");
            } catch (e: any) {
                expect(e.message).to.include("InsufficientBalance");
            }
        });

        it("rejects unauthorized withdrawal", async () => {
            try {
                await program.methods
                    .withdraw(new anchor.BN(withdrawAmount))
                    .accounts({
                        user: user2.publicKey,  // Different user!
                        vault: vaultPda,        // Trying to withdraw from user1's vault
                        userTokenAccount: user2TokenAccount,
                        vaultTokenAccount: vaultTokenAccount,
                        vaultAuthority: vaultAuthorityPda,
                        tokenProgram: TOKEN_PROGRAM_ID,
                    } as any)
                    .signers([user2])
                    .rpc();
                expect.fail("Should have thrown");
            } catch (e) {
                // Should fail due to PDA seeds mismatch
                expect(e).to.exist;
            }
        });
    });

    // ==========================================
    // LOCK/UNLOCK TESTS
    // ==========================================

    describe("Lock/Unlock Collateral", () => {
        const lockAmount = 30 * Math.pow(10, USDT_DECIMALS); // 30 USDT

        it("locks collateral", async () => {
            const vaultBefore = await program.account.collateralVault.fetch(vaultPda);

            await program.methods
                .lockCollateral(new anchor.BN(lockAmount))
                .accounts({
                    authority: admin.publicKey,  // Using admin as authority for testing
                    vault: vaultPda,
                    vaultAuthority: vaultAuthorityPda,
                } as any)
                .rpc();

            const vaultAfter = await program.account.collateralVault.fetch(vaultPda);

            expect(vaultAfter.lockedBalance.toNumber()).to.equal(
                vaultBefore.lockedBalance.toNumber() + lockAmount
            );
            expect(vaultAfter.availableBalance.toNumber()).to.equal(
                vaultBefore.availableBalance.toNumber() - lockAmount
            );
            // Total should remain unchanged
            expect(vaultAfter.totalBalance.toNumber()).to.equal(
                vaultBefore.totalBalance.toNumber()
            );
        });

        it("prevents withdrawal of locked funds", async () => {
            const vault = await program.account.collateralVault.fetch(vaultPda);
            // Try to withdraw more than available (but less than total)
            const attemptAmount = vault.availableBalance.toNumber() + 1;

            try {
                await program.methods
                    .withdraw(new anchor.BN(attemptAmount))
                    .accounts({
                        user: user.publicKey,
                        vault: vaultPda,
                        userTokenAccount: userTokenAccount,
                        vaultTokenAccount: vaultTokenAccount,
                        vaultAuthority: vaultAuthorityPda,
                        tokenProgram: TOKEN_PROGRAM_ID,
                    } as any)
                    .signers([user])
                    .rpc();
                expect.fail("Should have thrown");
            } catch (e: any) {
                expect(e.message).to.include("InsufficientBalance");
            }
        });

        it("unlocks collateral", async () => {
            const vaultBefore = await program.account.collateralVault.fetch(vaultPda);

            await program.methods
                .unlockCollateral(new anchor.BN(lockAmount))
                .accounts({
                    authority: admin.publicKey,
                    vault: vaultPda,
                    vaultAuthority: vaultAuthorityPda,
                } as any)
                .rpc();

            const vaultAfter = await program.account.collateralVault.fetch(vaultPda);

            expect(vaultAfter.lockedBalance.toNumber()).to.equal(
                vaultBefore.lockedBalance.toNumber() - lockAmount
            );
            expect(vaultAfter.availableBalance.toNumber()).to.equal(
                vaultBefore.availableBalance.toNumber() + lockAmount
            );
        });

        it("rejects unlocking more than locked", async () => {
            const vault = await program.account.collateralVault.fetch(vaultPda);
            const tooMuch = vault.lockedBalance.toNumber() + 1;

            try {
                await program.methods
                    .unlockCollateral(new anchor.BN(tooMuch))
                    .accounts({
                        authority: admin.publicKey,
                        vault: vaultPda,
                        vaultAuthority: vaultAuthorityPda,
                    } as any)
                    .rpc();
                expect.fail("Should have thrown");
            } catch (e: any) {
                expect(e.message).to.include("InsufficientLockedBalance");
            }
        });
    });

    // ==========================================
    // TRANSFER TESTS
    // ==========================================

    describe("Transfer Collateral", () => {
        const transferAmount = 10 * Math.pow(10, USDT_DECIMALS); // 10 USDT

        before(async () => {
            // Deposit to user2's vault first
            await program.methods
                .deposit(new anchor.BN(100 * Math.pow(10, USDT_DECIMALS)))
                .accounts({
                    user: user2.publicKey,
                    vault: vault2Pda,
                    userTokenAccount: user2TokenAccount,
                    vaultTokenAccount: vault2TokenAccount,
                    vaultAuthority: vaultAuthorityPda,
                    tokenProgram: TOKEN_PROGRAM_ID,
                } as any)
                .signers([user2])
                .rpc();
        });

        it("transfers between vaults", async () => {
            const fromBefore = await program.account.collateralVault.fetch(vaultPda);
            const toBefore = await program.account.collateralVault.fetch(vault2Pda);

            await program.methods
                .transferCollateral(
                    new anchor.BN(transferAmount),
                    { settlement: {} }  // TransferReason::Settlement
                )
                .accounts({
                    authority: admin.publicKey,
                    fromVault: vaultPda,
                    fromTokenAccount: vaultTokenAccount,
                    toVault: vault2Pda,
                    toTokenAccount: vault2TokenAccount,
                    vaultAuthority: vaultAuthorityPda,
                    tokenProgram: TOKEN_PROGRAM_ID,
                } as any)
                .rpc();

            const fromAfter = await program.account.collateralVault.fetch(vaultPda);
            const toAfter = await program.account.collateralVault.fetch(vault2Pda);

            expect(fromAfter.totalBalance.toNumber()).to.equal(
                fromBefore.totalBalance.toNumber() - transferAmount
            );
            expect(toAfter.totalBalance.toNumber()).to.equal(
                toBefore.totalBalance.toNumber() + transferAmount
            );
        });
    });

    // ==========================================
    // ADMIN TESTS
    // ==========================================

    describe("Admin Functions", () => {
        it("adds authorized program", async () => {
            const programToAdd = Keypair.generate().publicKey;

            await program.methods
                .addAuthorizedProgram(programToAdd)
                .accounts({
                    admin: admin.publicKey,
                    vaultAuthority: vaultAuthorityPda,
                } as any)
                .rpc();

            const authority = await program.account.vaultAuthority.fetch(vaultAuthorityPda);
            expect(authority.authorizedPrograms.some(
                p => p.toBase58() === programToAdd.toBase58()
            )).to.be.true;
        });

        it("removes authorized program", async () => {
            const programToRemove = Keypair.generate().publicKey;

            // Add first
            await program.methods
                .addAuthorizedProgram(programToRemove)
                .accounts({
                    admin: admin.publicKey,
                    vaultAuthority: vaultAuthorityPda,
                } as any)
                .rpc();

            // Remove
            await program.methods
                .removeAuthorizedProgram(programToRemove)
                .accounts({
                    admin: admin.publicKey,
                    vaultAuthority: vaultAuthorityPda,
                } as any)
                .rpc();

            const authority = await program.account.vaultAuthority.fetch(vaultAuthorityPda);
            expect(authority.authorizedPrograms.some(
                p => p.toBase58() === programToRemove.toBase58()
            )).to.be.false;
        });

        it("rejects non-admin from modifying authority", async () => {
            try {
                await program.methods
                    .setPaused(true)
                    .accounts({
                        admin: user.publicKey,  // Not the admin!
                        vaultAuthority: vaultAuthorityPda,
                    } as any)
                    .signers([user])
                    .rpc();
                expect.fail("Should have thrown");
            } catch (e: any) {
                expect(e.message).to.include("NotAdmin");
            }
        });
    });
});

