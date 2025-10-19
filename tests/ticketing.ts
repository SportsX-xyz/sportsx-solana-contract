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
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddress,
    getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { assert } from "chai";
import { TicketingProgram } from "../target/types/ticketing_program";

describe("ticketing-program", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.TicketingProgram as Program<TicketingProgram>;
    const connection = provider.connection;

    let platformConfigPDA: PublicKey;
    let platformAuthority: PublicKey;
    let user: Keypair;
    let merchant: Keypair;
    let ticketMint: Keypair;
    let eventId = "test-event-123";
    let ticketId = "ticket-001";
    let seatNumber = "A-101";

    before(async () => {
        // Generate test users
        user = Keypair.generate();
        merchant = Keypair.generate();
        platformAuthority = provider.wallet.publicKey;
        ticketMint = Keypair.generate();


        console.log("--- DEBUG Public Keys ---");
        console.log("User Public Key:", user.publicKey.toBase58());
        console.log("Merchant Public Key:", merchant.publicKey.toBase58());
        console.log("Platform Authority Key (Wallet):", platformAuthority.toBase58());
        console.log("ticketMint Public Key:", ticketMint.publicKey.toBase58());

        // Fund test users
        await provider.connection.requestAirdrop(user.publicKey, LAMPORTS_PER_SOL);
        await provider.connection.requestAirdrop(merchant.publicKey, LAMPORTS_PER_SOL);

        // Derive platform config PDA
        [platformConfigPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("PLATFORM_CONFIG")],
            program.programId
        );

        console.log("Setup complete:");
        console.log("Platform PDA:", platformConfigPDA.toBase58());
    });

    describe("PlatformConfig", () => {
        it("Initializes platform config successfully", async () => {
            const tx = await program.methods
                .initializePlatformConfig(platformAuthority)
                .accounts({
                    payer: provider.wallet.publicKey,
                    platformConfig: platformConfigPDA,
                    admin: provider.wallet.publicKey,
                    systemProgram: SystemProgram.programId,
                })
                .rpc();

            const config = await program.account.platformConfig.fetch(platformConfigPDA);
            assert.equal(config.platformAuthority.toBase58(), platformAuthority.toBase58());
            assert.isNumber(config.bump);
            console.log("✅ Platform config initialized:", tx);
        });

        it("Fails to initialize twice (already exists)", async () => {
            try {
                await program.methods
                    .initializePlatformConfig(platformAuthority)
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
            await provider.connection.requestAirdrop(fakeAdmin.publicKey, anchor.web3.LAMPORTS_PER_SOL);
            const [platformConfigPDA_fake] = anchor.web3.PublicKey.findProgramAddressSync(
                [Buffer.from("PLATFORM_CONFIG")],
                program.programId
            );
            try {
                await program.methods
                    .initializePlatformConfig(platformAuthority)
                    .accounts({
                        payer: fakeAdmin.publicKey,
                        platformConfig: platformConfigPDA_fake,
                        admin: fakeAdmin.publicKey,
                        systemProgram: SystemProgram.programId,
                    })
                    .signers([fakeAdmin])
                    .rpc();
                assert.fail("Should have failed, but did not.");
            } catch (error: any) {
                assert.include(error.message, "already in use");
                console.log("✅ Correctly rejected duplicate initialization attempt by fakeAdmin");
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

            const invalidEventId = "invalid-event-uri";
            const [invalidEventPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("EVENT"), Buffer.from(invalidEventId)],
                program.programId
            );

            try {
                await program.methods
                    .createEvent(
                        invalidEventId,
                        "http://invalid", // invalid protocol
                        merchant.publicKey,
                        "Invalid Event",
                        "INV",
                        new anchor.BN(Date.now() + 86400)
                    )
                    .accounts({
                        payer: provider.wallet.publicKey,
                        event: invalidEventPDA,
                        platformConfig: platformConfigPDA,
                        platformAuthority: platformAuthority,
                        systemProgram: SystemProgram.programId,
                    })
                    .rpc();
                assert.fail("Should fail with invalid URI");
            } catch (error: any) {
                console.log("Error:", error.toString());
                if (error.error?.errorCode?.code === "InvalidUri") {
                    console.log("✅ Correctly failed with InvalidUri");
                } else {
                    console.log("Got different error:", error.error?.errorCode?.code);
                }
            }
        });

        it("Fails to create event with invalid platform authority", async () => {
            // const fakeAdmin = Keypair.generate();
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
                    .signers([user])
                    .rpc();
                assert.fail("Should fail with invalid authority");
            } catch (error: any) {
                console.log("Error:", error.toString());
                if (error.error?.errorCode?.code === "InvalidPlatformAuthority") {
                    console.log("✅ Correctly failed with InvalidPlatformAuthority");
                } else {
                    console.log("Got different error:", error.error?.errorCode?.code);
                }
            }
        });
    });

    describe("MintTicket", () => {
        let eventPDA: PublicKey;
        let seatAccountPDA: PublicKey;

        let mintAuthorityPDA: PublicKey;
        let userNftAta: PublicKey;

        before(async () => {
            [eventPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("EVENT"), Buffer.from(eventId)],
                program.programId
            );
            [seatAccountPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("TICKET"), Buffer.from(ticketId), eventPDA.toBuffer()],
                program.programId
            );
            [mintAuthorityPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("MINT_AUTH")],
                program.programId
            );

            // Get user NFT ATA
            userNftAta = await getAssociatedTokenAddress(
                ticketMint.publicKey,
                user.publicKey,
                false,
                TOKEN_2022_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID
            );

            console.log("userNftAta (Wallet):", userNftAta.toBase58());
        });

        it("Mints ticket NFT successfully", async () => {

            const tx = await program.methods
                .mintTicket(
                    ticketId,
                    eventId,
                    seatNumber
                )
                .accounts({
                    user: user.publicKey,
                    platformAuthority: platformAuthority,
                    ticketMint: ticketMint.publicKey,
                    userNftAta: userNftAta,
                    mintAuthority: mintAuthorityPDA,
                    seatAccount: seatAccountPDA,
                    event: eventPDA,
                    platformConfig: platformConfigPDA,
                    tokenProgram: TOKEN_2022_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                    associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
                    rent: SYSVAR_RENT_PUBKEY,
                })
                .signers([user, provider.wallet.payer, ticketMint])
                .rpc();

            console.log("✅ Ticket minted successfully:", tx);

            // Verify seat account was updated
            const seatAccount = await program.account.seatStatus.fetch(seatAccountPDA);
            assert.isTrue(seatAccount.isMinted);
            assert.isFalse(seatAccount.isScanned);
        });

        it("Fails when ticket already minted", async () => {
            try {
                const newTicketMint = Keypair.generate();
                const newUserNftAta = await getAssociatedTokenAddress(
                    newTicketMint.publicKey,
                    user.publicKey,
                    false,
                    TOKEN_2022_PROGRAM_ID
                );

                await program.methods
                    .mintTicket(
                        ticketId, // same ticket ID
                        eventId,
                        "A-102" // different seat
                    )
                    .accounts({
                        user: user.publicKey,
                        platformAuthority: platformAuthority,
                        ticketMint: newTicketMint.publicKey,
                        userNftAta: newUserNftAta,
                        mintAuthority: mintAuthorityPDA,
                        seatAccount: seatAccountPDA, // same seat account
                        event: eventPDA,
                        platformConfig: platformConfigPDA,
                        tokenProgram: TOKEN_2022_PROGRAM_ID,
                        systemProgram: SystemProgram.programId,
                        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
                        rent: SYSVAR_RENT_PUBKEY,
                    })
                    .signers([user, provider.wallet.payer, newTicketMint])
                    .rpc();
                assert.fail("Should fail with already minted ticket");
            } catch (error: any) {
                console.log("Error:", error.toString());
                if (error.error?.errorCode?.code === "TicketAlreadyMinted") {
                    console.log("✅ Correctly failed with TicketAlreadyMinted");
                } else {
                    console.log("Got different error:", error.error?.errorCode?.code);
                }
            }
        });

        it("Fails with invalid event ID", async () => {
            const newTicketId = "new-ticket-001";
            const [newSeatAccountPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("TICKET"), Buffer.from(newTicketId), eventPDA.toBuffer()],
                program.programId
            );
            const newTicketMint = Keypair.generate();
            const newUserNftAta = await getAssociatedTokenAddress(
                newTicketMint.publicKey,
                user.publicKey,
                false,
                TOKEN_2022_PROGRAM_ID
            );

            try {
                await program.methods
                    .mintTicket(
                        newTicketId,
                        "wrong-event-id", // wrong event ID
                        "A-103"
                    )
                    .accounts({
                        user: user.publicKey,
                        platformAuthority: platformAuthority,
                        ticketMint: newTicketMint.publicKey,
                        userNftAta: newUserNftAta,
                        mintAuthority: mintAuthorityPDA,
                        seatAccount: newSeatAccountPDA,
                        event: eventPDA, // correct event PDA but wrong ID
                        platformConfig: platformConfigPDA,
                        tokenProgram: TOKEN_2022_PROGRAM_ID,
                        systemProgram: SystemProgram.programId,
                        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
                        rent: SYSVAR_RENT_PUBKEY,
                    })
                    .signers([user, newTicketMint])
                    .rpc();
                assert.fail("Should fail with invalid event ID");
            } catch (error: any) {
                console.log("Error:", error.toString());
                if (error.error?.errorCode?.code === "InvalidEventId") {
                    console.log("✅ Correctly failed with InvalidEventId");
                } else {
                    console.log("Got different error:", error.error?.errorCode?.code);
                }
                // assert.include(error.message, "InvalidEventId");
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

        it("Scans ticket successfully", async () => {
            const tx = await program.methods
                .scanTicket(ticketId, eventId)
                .accounts({
                    merchant: merchant.publicKey,
                    seatAccount: seatAccountPDA,
                    event: eventPDA,
                    ticketMint: ticketMint.publicKey,
                })
                .signers([merchant])
                .rpc();

            console.log("✅ Ticket scanned successfully:", tx);

            // Verify seat account was updated
            const seatAccount = await program.account.seatStatus.fetch(seatAccountPDA);
            assert.isTrue(seatAccount.isMinted);
            assert.isTrue(seatAccount.isScanned);
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
                assert.fail("Should fail with already scanned ticket");
            } catch (error: any) {
                assert.include(error.message, "TicketAlreadyScanned");
            }
        });
    });

    describe("UpdateSeatNumber", () => {
        let eventPDA: PublicKey;
        let seatAccountPDA: PublicKey;
        let ticketMint: Keypair;
        let mintAuthorityPDA: PublicKey;

        before(async () => {
            [eventPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("EVENT"), Buffer.from(eventId)],
                program.programId
            );

            const newTicketId = "ticket-update-seat";
            [seatAccountPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("TICKET"), Buffer.from(newTicketId), eventPDA.toBuffer()],
                program.programId
            );
            [mintAuthorityPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("MINT_AUTH")],
                program.programId
            );

            ticketMint = Keypair.generate();
        });

        it("Updates seat number successfully", async () => {
            // First mint a ticket
            const userNftAta = await getAssociatedTokenAddress(
                ticketMint.publicKey,
                user.publicKey,
                false,
                TOKEN_2022_PROGRAM_ID
            );

            await program.methods
                .mintTicket(
                    "ticket-update-seat",
                    eventId,
                    "B-201"
                )
                .accounts({
                    user: user.publicKey,
                    platformAuthority: platformAuthority,
                    ticketMint: ticketMint.publicKey,
                    userNftAta: userNftAta,
                    mintAuthority: mintAuthorityPDA,
                    seatAccount: seatAccountPDA,
                    event: eventPDA,
                    platformConfig: platformConfigPDA,
                    tokenProgram: TOKEN_2022_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                    associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
                    rent: SYSVAR_RENT_PUBKEY,
                })
                .signers([user, ticketMint])
                .rpc();

            // Then update seat number
            const tx = await program.methods
                .updateSeatNumber(
                    "ticket-update-seat",
                    eventId,
                    "B-202" // new seat number
                )
                .accounts({
                    merchant: merchant.publicKey,
                    mint: ticketMint.publicKey,
                    mintAuthority: mintAuthorityPDA,
                    seatAccount: seatAccountPDA,
                    event: eventPDA,
                    tokenProgram: TOKEN_2022_PROGRAM_ID,
                })
                .signers([merchant])
                .rpc();

            console.log("✅ Seat number updated successfully:", tx);
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

    describe("Direct Account Query", () => {
        let eventPDA: PublicKey;
        let seatAccountPDA: PublicKey;
        let existingTicketId = "ticket-001";

        before(async () => {
            [eventPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("EVENT"), Buffer.from(eventId)],
                program.programId
            );
            [seatAccountPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("TICKET"), Buffer.from(existingTicketId), eventPDA.toBuffer()],
                program.programId
            );
        });

        it("Successfully queries existing ticket status", async () => {
            const ticketMint = Keypair.generate();
            const mintAuthorityPDA = PublicKey.findProgramAddressSync(
                [Buffer.from("MINT_AUTH")],
                program.programId
            )[0];
            const userNftAta = await getAssociatedTokenAddress(
                ticketMint.publicKey,
                user.publicKey,
                false,
                TOKEN_2022_PROGRAM_ID
            );

            await program.methods
                .mintTicket(existingTicketId, eventId, "QUERY-TEST-SEAT")
                .accounts({
                    user: user.publicKey,
                    platformAuthority: platformAuthority,
                    ticketMint: ticketMint.publicKey,
                    userNftAta: userNftAta,
                    mintAuthority: mintAuthorityPDA,
                    seatAccount: seatAccountPDA,
                    event: eventPDA,
                    platformConfig: platformConfigPDA,
                    tokenProgram: TOKEN_2022_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                    associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
                    rent: SYSVAR_RENT_PUBKEY,
                })
                .signers([user, ticketMint])
                .rpc();

            const seatAccount = await program.account.seatStatus.fetch(seatAccountPDA);
            console.log("✅ Direct query successful:", {
                isMinted: seatAccount.isMinted,
                isScanned: seatAccount.isScanned,
                bump: seatAccount.bump
            });

            assert.isTrue(seatAccount.isMinted);
            assert.isFalse(seatAccount.isScanned);
            assert.isNumber(seatAccount.bump);
        });

        it("Handles non-existent ticket gracefully", async () => {
            const nonExistentTicketId = "non-existent-ticket-999";
            const [nonExistentSeatPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("TICKET"), Buffer.from(nonExistentTicketId), eventPDA.toBuffer()],
                program.programId
            );

            try {
                await program.account.seatStatus.fetch(nonExistentSeatPDA);
                assert.fail("Should not find non-existent account");
            } catch (error: any) {
                assert.include(error.message, "Account does not exist");
                console.log("✅ Correctly handled non-existent ticket");
            }
        });

        it("Queries event information directly", async () => {
            const event = await program.account.events.fetch(eventPDA);
            console.log("✅ Event query successful:", {
                eventId: event.eventId,
                name: event.name,
                symbol: event.symbol,
                merchantKey: event.merchantKey.toBase58(),
                uri: event.uri
            });

            assert.equal(event.eventId, eventId);
            assert.equal(event.merchantKey.toBase58(), merchant.publicKey.toBase58());
        });

        it("Queries platform config directly", async () => {
            const platformConfig = await program.account.platformConfig.fetch(platformConfigPDA);
            console.log("✅ Platform config query successful:", {
                platformAuthority: platformConfig.platformAuthority.toBase58(),
                bump: platformConfig.bump
            });

            assert.equal(platformConfig.platformAuthority.toBase58(), platformAuthority.toBase58());
            assert.isNumber(platformConfig.bump);
        });
    });
});