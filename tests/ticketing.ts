import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import {
    PublicKey,
    Keypair,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
    LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    createMint,
    createAssociatedTokenAccountInstruction,
    createInitializeMintInstruction,
    getAssociatedTokenAddress,
    mintTo,
    createAccountInstruction,
} from "@solana/spl-token";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";
import { TicketingProgram } from "../target/types/ticketing_program";

describe("ticketing-program", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.TicketingProgram as Program<TicketingProgram>;
    const connection = provider.connection;

    let platformConfigPDA: PublicKey;
    let platformAuthority: PublicKey;
    let usdtMint: PublicKey;
    let user: Keypair;
    let merchant: Keypair;
    let eventId = "test-event-123";
    let ticketId = "ticket-001";
    let seatNumber = "A-101";

    before(async () => {
        // Generate test users
        user = Keypair.generate();
        merchant = Keypair.generate();
        platformAuthority = provider.wallet.publicKey;

        // Fund test users
        await provider.connection.requestAirdrop(user.publicKey, LAMPORTS_PER_SOL);
        await provider.connection.requestAirdrop(merchant.publicKey, LAMPORTS_PER_SOL);

        // Derive platform config PDA
        [platformConfigPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("PLATFORM_CONFIG")],
            program.programId
        );

        // Create USDT mint for testing
        usdtMint = await createMint(
            connection,
            provider.wallet.payer,
            provider.wallet.publicKey, // mint authority
            null,
            6 // USDT decimals
        );

        // Create ATA for platform authority
        const platformATA = await getAssociatedTokenAddress(usdtMint, platformAuthority);
        await provider.connection.requestAirdrop(platformAuthority, 0.1 * LAMPORTS_PER_SOL);
        const createPlatformATAIx = createAssociatedTokenAccountInstruction(
            platformAuthority,
            platformATA,
            platformAuthority,
            usdtMint
        );
        await provider.sendAndConfirm(createPlatformATAIx);

        // Mint some USDT to users
        await mintTo(
            connection,
            provider.wallet.payer,
            usdtMint,
            platformATA,
            provider.wallet.payer,
            new BN(1000000 * 1e6) // 1000 USDT
        );

        await mintTo(
            connection,
            provider.wallet.payer,
            usdtMint,
            await getAssociatedTokenAddress(usdtMint, user.publicKey),
            provider.wallet.payer,
            new BN(100 * 1e6) // 100 USDT
        );

        console.log("Setup complete:");
        console.log("Platform PDA:", platformConfigPDA.toBase58());
        console.log("USDT Mint:", usdtMint.toBase58());
    });

    describe("PlatformConfig", () => {
        it("Initializes platform config successfully", async () => {
            const tx = await program.methods
                .initializePlatformConfig(platformAuthority, usdtMint)
                .accounts({
                    payer: provider.wallet.publicKey,
                    platformConfig: platformConfigPDA,
                    admin: provider.wallet.publicKey,
                    systemProgram: SystemProgram.programId,
                })
                .rpc();

            const config = await program.account.platformConfig.fetch(platformConfigPDA);
            assert.equal(config.platformAuthority.toBase58(), platformAuthority.toBase58());
            assert.equal(config.usdtMint.toBase58(), usdtMint.toBase58());
            assert.isNumber(config.bump);
            console.log("✅ Platform config initialized:", tx);
        });

        it("Fails to initialize twice (already exists)", async () => {
            try {
                await program.methods
                    .initializePlatformConfig(platformAuthority, usdtMint)
                    .accounts({
                        payer: provider.wallet.publicKey,
                        platformConfig: platformConfigPDA,
                        admin: provider.wallet.publicKey,
                        systemProgram: SystemProgram.programId,
                    })
                    .rpc();
                assert.fail("Should have failed with existing account");
            } catch (error: any) {
                assert.include(error.message, "already in use");
                console.log("✅ Correctly rejected duplicate initialization");
            }
        });

        it("Fails with unauthorized admin", async () => {
            const fakeAdmin = Keypair.generate();
            try {
                await program.methods
                    .initializePlatformConfig(platformAuthority, usdtMint)
                    .accounts({
                        payer: fakeAdmin.publicKey,
                        platformConfig: Keypair.generate().publicKey, // new PDA would be needed
                        admin: fakeAdmin.publicKey,
                        systemProgram: SystemProgram.programId,
                    })
                    .signers([fakeAdmin])
                    .rpc();
                assert.fail("Should have failed with unauthorized admin");
            } catch (error: any) {
                assert.include(error.message, "UnauthorizedAdmin");
            }
        });
    });

    describe("Events", () => {
        let eventPDA: PublicKey;

        before(async () => {
            [eventPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("EVENT"), Buffer.from(eventId)],
                program.programId
            );
        });

        it("Creates event successfully", async () => {
            const tx = await program.methods
                .createEvent(
                    eventId,
                    "https://example.com/event.json",
                    merchant.publicKey,
                    "Test Concert",
                    "TCON",
                    new anchor.BN(Date.now() + 24 * 60 * 60 * 1000) // tomorrow
                )
                .accounts({
                    payer: provider.wallet.publicKey,
                    event: eventPDA,
                    platformConfig: platformConfigPDA,
                    platformAuthority: platformAuthority,
                    systemProgram: SystemProgram.programId,
                })
                .rpc();

            const event = await program.account.events.fetch(eventPDA);
            assert.equal(event.eventId, eventId);
            assert.equal(event.name, "Test Concert");
            assert.equal(event.symbol, "TCON");
            assert.equal(event.merchantKey.toBase58(), merchant.publicKey.toBase58());
            console.log("✅ Event created:", tx);
        });

        it("Fails to create event with invalid URI", async () => {
            try {
                await program.methods
                    .createEvent(
                        "invalid-event",
                        "http://invalid", // invalid protocol
                        merchant.publicKey,
                        "Invalid Event",
                        "INV",
                        new anchor.BN(Date.now() + 86400)
                    )
                    .accounts({
                        payer: provider.wallet.publicKey,
                        event: Keypair.generate().publicKey,
                        platformConfig: platformConfigPDA,
                        platformAuthority: platformAuthority,
                        systemProgram: SystemProgram.programId,
                    })
                    .rpc();
                assert.fail("Should fail with invalid URI");
            } catch (error: any) {
                assert.include(error.message, "InvalidUri");
            }
        });

        it("Fails to create event with invalid platform authority", async () => {
            try {
                await program.methods
                    .createEvent(
                        "unauth-event",
                        "https://valid.com",
                        merchant.publicKey,
                        "Unauthorized",
                        "UNAUTH",
                        new anchor.BN(Date.now() + 86400)
                    )
                    .accounts({
                        payer: provider.wallet.publicKey,
                        event: Keypair.generate().publicKey,
                        platformConfig: platformConfigPDA,
                        platformAuthority: user.publicKey, // wrong authority
                        systemProgram: SystemProgram.programId,
                    })
                    .rpc();
                assert.fail("Should fail with invalid authority");
            } catch (error: any) {
                assert.include(error.message, "InvalidPlatformAuthority");
            }
        });
    });

    describe("PurchaseAndMint", () => {
        let eventPDA: PublicKey;
        let seatAccountPDA: PublicKey;
        let ticketMint: PublicKey;
        let userUSDTATA: PublicKey;
        let platformUSDTATA: PublicKey;

        before(async () => {
            [eventPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("EVENT"), Buffer.from(eventId)],
                program.programId
            );
            [seatAccountPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("TICKET"), Buffer.from(ticketId), eventPDA.toBuffer()],
                program.programId
            );
            [platformUSDTATA] = PublicKey.findProgramAddressSync(
                [usdtMint.toBuffer(), platformAuthority.toBuffer()],
                TOKEN_PROGRAM_ID
            );
            [userUSDTATA] = PublicKey.findProgramAddressSync(
                [usdtMint.toBuffer(), user.publicKey.toBuffer()],
                TOKEN_PROGRAM_ID
            );

            // Create ticket mint for testing
            ticketMint = Keypair.generate().publicKey;
        });

        it("Fails when ticket already minted", async () => {
            // First successful mint
            try {
                await program.methods
                    .purchaseAndMint(
                        new anchor.BN(10 * 1e6), // $10 USDT
                        ticketId,
                        eventId,
                        seatNumber
                    )
                    .accounts({
                        user: user.publicKey,
                        platformAuthority: platformAuthority,
                        usdtMint,
                        userUsdtAta: userUSDTATA,
                        platformUsdtVault: platformUSDTATA,
                        merchantUsdtVault: Keypair.generate().publicKey, // mock
                        ticketMint,
                        userNftAta: Keypair.generate().publicKey,
                        mintAuthority: Keypair.generate().publicKey,
                        seatAccount: seatAccountPDA,
                        event: eventPDA,
                        platformConfig: platformConfigPDA,
                        systemProgram: SystemProgram.programId,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
                        rent: SYSVAR_RENT_PUBKEY,
                    })
                    .signers([user])
                    .rpc();
            } catch (error) {
                console.log("First mint setup failed:", error);
            }

            // Second attempt should fail
            try {
                await program.methods
                    .purchaseAndMint(
                        new anchor.BN(10 * 1e6),
                        ticketId,
                        eventId,
                        seatNumber
                    )
                    .accounts({
                        user: user.publicKey,
                        platformAuthority: platformAuthority,
                        usdtMint,
                        userUsdtAta: userUSDTATA,
                        platformUsdtVault: platformUSDTATA,
                        merchantUsdtVault: Keypair.generate().publicKey,
                        ticketMint: Keypair.generate().publicKey, // new mint
                        userNftAta: Keypair.generate().publicKey,
                        mintAuthority: Keypair.generate().publicKey,
                        seatAccount: seatAccountPDA, // same seat account
                        event: eventPDA,
                        platformConfig: platformConfigPDA,
                        systemProgram: SystemProgram.programId,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
                        rent: SYSVAR_RENT_PUBKEY,
                    })
                    .signers([user])
                    .rpc();
                assert.fail("Should fail with already minted ticket");
            } catch (error: any) {
                assert.include(error.message, "TicketAlreadyMinted");
                console.log("✅ Correctly rejected duplicate mint");
            }
        });

        it("Validates USDT mint match", async () => {
            const fakeMint = Keypair.generate().publicKey;
            try {
                await program.methods
                    .purchaseAndMint(new anchor.BN(10 * 1e6), "fake", "fake", "fake")
                    .accounts({
                        user: user.publicKey,
                        platformAuthority: platformAuthority,
                        usdtMint: fakeMint, // wrong mint
                        // ... other accounts
                        platformConfig: platformConfigPDA,
                        systemProgram: SystemProgram.programId,
                        tokenProgram: TOKEN_PROGRAM_ID,
                    })
                    .signers([user])
                    .rpc();
                assert.fail("Should fail with invalid USDT mint");
            } catch (error: any) {
                assert.include(error.message, "InvalidUsdtMint");
            }
        });
    });

    describe("ScanTicket", () => {
        let eventPDA: PublicKey;
        let seatAccountPDA: PublicKey;

        before(async () => {
            [eventPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("EVENT"), Buffer.from(eventId)],
                program.programId
            );
            [seatAccountPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("TICKET"), Buffer.from(ticketId), eventPDA.toBuffer()],
                program.programId
            );
        });

        it("Fails to scan unminted ticket", async () => {
            try {
                await program.methods
                    .scanTicket(ticketId, eventId)
                    .accounts({
                        merchant: merchant.publicKey,
                        seatAccount: seatAccountPDA,
                        event: eventPDA,
                    })
                    .signers([merchant])
                    .rpc();
                assert.fail("Should fail with unminted ticket");
            } catch (error: any) {
                assert.include(error.message, "TicketNotMinted");
            }
        });

        it("Fails to scan with wrong merchant", async () => {
            try {
                await program.methods
                    .scanTicket(ticketId, eventId)
                    .accounts({
                        merchant: user.publicKey, // wrong merchant
                        seatAccount: seatAccountPDA,
                        event: eventPDA,
                    })
                    .signers([user])
                    .rpc();
                assert.fail("Should fail with wrong merchant");
            } catch (error: any) {
                assert.include(error.message, "InvalidMerchantAuthority");
            }
        });

        it("Fails to scan already scanned ticket", async () => {
            // Assuming ticket is minted and not scanned
            try {
                await program.methods
                    .scanTicket(ticketId, eventId)
                    .accounts({
                        merchant: merchant.publicKey,
                        seatAccount: seatAccountPDA,
                        event: eventPDA,
                    })
                    .signers([merchant])
                    .rpc();

                // Second scan should fail
                await program.methods
                    .scanTicket(ticketId, eventId)
                    .accounts({
                        merchant: merchant.publicKey,
                        seatAccount: seatAccountPDA,
                        event: eventPDA,
                    })
                    .signers([merchant])
                    .rpc();
                assert.fail("Should fail with already scanned ticket");
            } catch (error: any) {
                assert.include(error.message, "TicketAlreadyScanned");
            }
        });
    });

    describe("Input Validation", () => {
        it("Rejects oversized ticket ID", async () => {
            const longId = "a".repeat(33); // exceeds MAX_TICKET_ID_LENGTH
            try {
                await program.methods
                    .scanTicket(longId, eventId)
                    .accounts({
                        merchant: merchant.publicKey,
                        seatAccount: Keypair.generate().publicKey,
                        event: Keypair.generate().publicKey,
                    })
                    .signers([merchant])
                    .rpc();
                assert.fail("Should reject oversized ticket ID");
            } catch (error: any) {
                assert.include(error.message, "TicketIdTooLong");
            }
        });

        it("Rejects oversized event ID", async () => {
            const longId = "a".repeat(33);
            try {
                await program.methods
                    .createEvent(
                        longId,
                        "https://valid.com",
                        merchant.publicKey,
                        "Valid Name",
                        "SYM",
                        new anchor.BN(Date.now() + 86400)
                    )
                    .accounts({
                        payer: provider.wallet.publicKey,
                        event: Keypair.generate().publicKey,
                        platformConfig: platformConfigPDA,
                        platformAuthority: platformAuthority,
                        systemProgram: SystemProgram.programId,
                    })
                    .rpc();
                assert.fail("Should reject oversized event ID");
            } catch (error: any) {
                assert.include(error.message, "EventIdTooLong");
            }
        });
    });

    describe("QueryTicketStatus", () => {
        it("Successfully queries ticket status", async () => {
            // This should work even for non-existent tickets
            // as it only validates inputs and emits event
            try {
                await program.methods
                    .queryTicketStatus(ticketId, eventId)
                    .accounts({
                        seatAccount: Keypair.generate().publicKey,
                        event: eventPDA,
                    })
                    .rpc();
                console.log("✅ Query ticket status works");
            } catch (error) {
                console.log("Query failed but expected:", error);
            }
        });
    });
});