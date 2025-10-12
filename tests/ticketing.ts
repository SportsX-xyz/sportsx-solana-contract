import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TicketingProgram } from "../target/types/ticketing_program"; // Adjust path if needed
import { PublicKey, Keypair, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token-2022"; // If using Token-2022
import { expect } from "chai";

describe("ticketing_program", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const program = anchor.workspace.TicketingProgram as Program<TicketingProgram>;

    // Generate keypairs for testing
    const admin = provider.wallet; // Use default wallet as admin
    const platformAuthority = Keypair.generate();
    const merchant = Keypair.generate();
    const user = Keypair.generate();

    // PDAs
    let platformConfigPDA: PublicKey;
    let eventPDA: PublicKey;
    let seatAccountPDA: PublicKey;
    let mintAuthorityPDA: PublicKey;

    // Constants for testing
    const usdtDecimals = 6;
    const platformMintFee = 100000; // 0.1 USDT
    const ticketPrice = 200000; // 0.2 USDT
    const eventId = "event123";
    const ticketId = "ticket123";
    const seatNumber = "A1";
    const uri = "https://example.com/metadata.json";
    const name = "Event Name";
    const symbol = "EVT";
    const expiryTimestamp = Math.floor(Date.now() / 1000) + 3600; // 1 hour from now

    // Mints and ATAs
    let usdtMint: PublicKey;
    let userUsdtATA: PublicKey;
    let platformUsdtVault: PublicKey;
    let merchantUsdtVault: PublicKey;
    let ticketMint: Keypair;
    let userNftATA: PublicKey;

    before(async () => {
        // Fund accounts
        await provider.connection.requestAirdrop(platformAuthority.publicKey, anchor.web3.LAMPORTS_PER_SOL * 10);
        await provider.connection.requestAirdrop(merchant.publicKey, anchor.web3.LAMPORTS_PER_SOL * 10);
        await provider.connection.requestAirdrop(user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 10);

        // Derive PDAs
        [platformConfigPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("PLATFORM_CONFIG")],
            program.programId
        );
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

        // Create USDT Mint (simulate with SPL Token for testing; adjust for Token-2022 if needed)
        usdtMint = await anchor.spl.createMint(
            provider.connection,
            admin,
            admin.publicKey,
            null,
            usdtDecimals,
            undefined,
            undefined,
            TOKEN_PROGRAM_ID // Use TOKEN_2022_PROGRAM_ID if Token-2022
        );

        // Create ATAs and mint USDT to user
        userUsdtATA = getAssociatedTokenAddressSync(usdtMint, user.publicKey);
        await anchor.spl.createAssociatedTokenAccount(
            provider.connection,
            user,
            usdtMint,
            user.publicKey,
            undefined,
            TOKEN_PROGRAM_ID
        );
        await anchor.spl.mintTo(
            provider.connection,
            admin,
            usdtMint,
            userUsdtATA,
            admin.publicKey,
            ticketPrice * 2 // Mint enough for tests
        );

        // Create platform and merchant USDT vaults
        platformUsdtVault = getAssociatedTokenAddressSync(usdtMint, platformAuthority.publicKey);
        await anchor.spl.createAssociatedTokenAccount(
            provider.connection,
            platformAuthority,
            usdtMint,
            platformAuthority.publicKey,
            undefined,
            TOKEN_PROGRAM_ID
        );

        merchantUsdtVault = getAssociatedTokenAddressSync(usdtMint, merchant.publicKey);
        await anchor.spl.createAssociatedTokenAccount(
            provider.connection,
            merchant,
            usdtMint,
            merchant.publicKey,
            undefined,
            TOKEN_PROGRAM_ID
        );

        // Generate ticket mint keypair
        ticketMint = Keypair.generate();
    });

    it("Initializes platform config", async () => {
        await program.methods
            .initializePlatformConfig(platformAuthority.publicKey, usdtMint)
            .accounts({
                payer: admin.publicKey,
                platformConfig: platformConfigPDA,
                admin: admin.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .signers([admin.payer]) // If payer is separate; here assuming admin is payer
            .rpc();

        const config = await program.account.platformConfig.fetch(platformConfigPDA);
        expect(config.platformAuthority.toBase58()).to.equal(platformAuthority.publicKey.toBase58());
        expect(config.usdtMint.toBase58()).to.equal(usdtMint.toBase58());
    });

    it("Creates an event", async () => {
        await program.methods
            .createEvent(eventId, uri, merchant.publicKey, name, symbol, new anchor.BN(expiryTimestamp))
            .accounts({
                payer: platformAuthority.publicKey,
                event: eventPDA,
                platformConfig: platformConfigPDA,
                platformAuthority: platformAuthority.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .signers([platformAuthority])
            .rpc();

        const eventAccount = await program.account.events.fetch(eventPDA);
        expect(eventAccount.eventId).to.equal(eventId);
        expect(eventAccount.uri).to.equal(uri);
        expect(eventAccount.merchantKey.toBase58()).to.equal(merchant.publicKey.toBase58());
        expect(eventAccount.name).to.equal(name);
        expect(eventAccount.symbol).to.equal(symbol);
        expect(eventAccount.expiryTimestamp.toNumber()).to.equal(expiryTimestamp);
    });

    it("Purchases and mints a ticket", async () => {
        // Derive user NFT ATA
        userNftATA = getAssociatedTokenAddressSync(ticketMint.publicKey, user.publicKey, false, TOKEN_2022_PROGRAM_ID); // Use Token-2022

        const preUserBalance = await anchor.spl.getBalance(provider.connection, userUsdtATA);
        const prePlatformBalance = await anchor.spl.getBalance(provider.connection, platformUsdtVault);
        const preMerchantBalance = await anchor.spl.getBalance(provider.connection, merchantUsdtVault);

        await program.methods
            .purchaseAndMint(new anchor.BN(ticketPrice), ticketId, eventId, seatNumber)
            .accounts({
                user: user.publicKey,
                platformAuthority: platformAuthority.publicKey,
                usdtMint: usdtMint,
                userUsdtAta: userUsdtATA,
                platformUsdtVault: platformUsdtVault,
                merchantUsdtVault: merchantUsdtVault,
                ticketMint: ticketMint.publicKey,
                userNftAta: userNftATA,
                mintAuthority: mintAuthorityPDA,
                seatAccount: seatAccountPDA,
                event: eventPDA,
                platformConfig: platformConfigPDA,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_2022_PROGRAM_ID, // Adjust if mixed
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                rent: SYSVAR_RENT_PUBKEY,
            })
            .signers([user, platformAuthority])
            .rpc();

        // Assert seat account
        const seatAccount = await program.account.seatStatus.fetch(seatAccountPDA);
        expect(seatAccount.isMinted).to.be.true;
        expect(seatAccount.isScanned).to.be.false;

        // Assert balances
        const postUserBalance = await anchor.spl.getBalance(provider.connection, userUsdtATA);
        const postPlatformBalance = await anchor.spl.getBalance(provider.connection, platformUsdtVault);
        const postMerchantBalance = await anchor.spl.getBalance(provider.connection, merchantUsdtVault);

        expect(postUserBalance).to.equal(preUserBalance - ticketPrice);
        expect(postPlatformBalance).to.equal(prePlatformBalance + platformMintFee);
        expect(postMerchantBalance).to.equal(preMerchantBalance + (ticketPrice - platformMintFee));

        // Assert NFT minted (supply == 1)
        const mintInfo = await anchor.spl.getMint(provider.connection, ticketMint.publicKey, undefined, TOKEN_2022_PROGRAM_ID);
        expect(mintInfo.supply).to.equal(1n);
    });

    it("Scans a ticket", async () => {
        await program.methods
            .scanTicket(ticketId, eventId)
            .accounts({
                merchant: merchant.publicKey,
                seatAccount: seatAccountPDA,
                event: eventPDA,
            })
            .signers([merchant])
            .rpc();

        const seatAccount = await program.account.seatStatus.fetch(seatAccountPDA);
        expect(seatAccount.isScanned).to.be.true;
    });

    it("Updates seat number", async () => {
        const newSeatNumber = "A2";

        await program.methods
            .updateSeatNumber(ticketId, eventId, newSeatNumber)
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

        // Note: To assert metadata update, you may need to fetch metadata using spl-token-2022 libraries
        // For simplicity, assume success if no error
    });

    it("Queries ticket status", async () => {
        await program.methods
            .queryTicketStatus(ticketId, eventId)
            .accounts({
                seatAccount: seatAccountPDA,
                event: eventPDA,
            })
            .rpc();

        // This instruction emits an event; in tests, you can capture logs or just check no error
        // For advanced, use program.addEventListener to capture emits
    });
});