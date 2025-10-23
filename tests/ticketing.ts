import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { TicketingProgram } from "../target/types/ticketing_program";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  createAssociatedTokenAccount,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { assert } from "chai";
import { randomUUID } from "crypto";

// PoF Program ID (optional - tests will skip PoF if not available)
const POF_PROGRAM_ID = new PublicKey("E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV");
const USE_POF = false; // Set to true to test PoF integration

// Helper: Get PoF wallet PDA
function getPofWalletPda(wallet: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("wallet_points"), wallet.toBuffer()],
    POF_PROGRAM_ID
  )[0];
}

// Helper: Get PoF global state PDA
function getPofGlobalStatePda(): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("global_state")],
    POF_PROGRAM_ID
  )[0];
}

describe("SportsX Ticketing Program", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TicketingProgram as Program<TicketingProgram>;

  // Keypairs
  const deployer = provider.wallet as anchor.Wallet;
  const organizer = Keypair.generate();
  const buyer = Keypair.generate();
  const buyer2 = Keypair.generate();
  const backendAuthority = Keypair.generate();
  const checkinOperator = Keypair.generate();

  // PDAs
  let platformConfig: PublicKey;
  let nonceTracker: PublicKey;
  let eventPda: PublicKey;
  let ticketPda: PublicKey;
  let listingPda: PublicKey;
  let checkinAuthorityPda: PublicKey;
  let ticketAuthorityPda: PublicKey;
  
  // NFT related
  let ticketMintKeypair: Keypair;
  let buyerTicketAccount: PublicKey;

  // USDC Mint and Accounts
  let usdcMint: PublicKey;
  let platformUsdcAccount: PublicKey;
  let organizerUsdcAccount: PublicKey;
  let buyerUsdcAccount: PublicKey;
  let buyer2UsdcAccount: PublicKey;

  // Test Data
  const EVENT_ID = "test_event_001";
  const TICKET_TYPE_ID = "vip";
  const PLATFORM_FEE = 100_000; // 0.1 USDC
  const TICKET_PRICE = 50_000_000; // 50 USDC

  before(async () => {
    // Airdrop SOL to test accounts
    await Promise.all([
      provider.connection.requestAirdrop(organizer.publicKey, 5 * LAMPORTS_PER_SOL),
      provider.connection.requestAirdrop(buyer.publicKey, 5 * LAMPORTS_PER_SOL),
      provider.connection.requestAirdrop(buyer2.publicKey, 5 * LAMPORTS_PER_SOL),
    ]);

    // Wait for airdrops to confirm
    await new Promise((resolve) => setTimeout(resolve, 2000));

    // Create test USDC mint (we control mint authority for testing)
    // Only create if not already created
    if (!usdcMint) {
      usdcMint = await createMint(
        provider.connection,
        deployer.payer,
        deployer.publicKey,
        null,
        6 // USDC decimals
      );
      console.log("  Created test USDC mint:", usdcMint.toString());
    }

    // Create USDC token accounts using getOrCreate to avoid duplicates
    const platformAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      deployer.payer,
      usdcMint,
      deployer.publicKey
    );
    platformUsdcAccount = platformAta.address;

    // Create ATA for event organizer (deployer is the organizer in tests)
    const organizerAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      deployer.payer,
      usdcMint,
      deployer.publicKey  // deployer is set as event.organizer in create_event
    );
    organizerUsdcAccount = organizerAta.address;

    // Create ATAs for buyers using getOrCreate to avoid duplicates
    const buyerAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      deployer.payer,
      usdcMint,
      buyer.publicKey
    );
    buyerUsdcAccount = buyerAta.address;

    const buyer2Ata = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      deployer.payer,
      usdcMint,
      buyer2.publicKey
    );
    buyer2UsdcAccount = buyer2Ata.address;

    // Mint USDC to buyers
    await mintTo(
      provider.connection,
      deployer.payer,
      usdcMint,
      buyerUsdcAccount,
      deployer.publicKey,
      1000_000_000 // 1000 USDC
    );

    await mintTo(
      provider.connection,
      deployer.payer,
      usdcMint,
      buyer2UsdcAccount,
      deployer.publicKey,
      1000_000_000 // 1000 USDC
    );

    // Derive PDAs
    [platformConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from("platform_config")],
      program.programId
    );

    [nonceTracker] = PublicKey.findProgramAddressSync(
      [Buffer.from("nonce_tracker")],
      program.programId
    );

    [eventPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("event"), Buffer.from(EVENT_ID)],
      program.programId
    );

    [ticketAuthorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("ticket_authority")],
      program.programId
    );

  });

  describe("Platform Management", () => {
    it("Initializes platform", async () => {
      await program.methods
        .initializePlatform(
          deployer.publicKey,
          new BN(PLATFORM_FEE),
          backendAuthority.publicKey,
          deployer.publicKey  // event_admin = deployer for testing
        )
        .accounts({
          platformConfig,
          nonceTracker,
          deployer: deployer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const config = await program.account.platformConfig.fetch(platformConfig);
      assert.equal(config.feeReceiver.toString(), deployer.publicKey.toString());
      assert.equal(config.feeAmountUsdc.toNumber(), PLATFORM_FEE);
      assert.equal(config.updateAuthority.toString(), deployer.publicKey.toString());
      assert.equal(config.backendAuthority.toString(), backendAuthority.publicKey.toString());
      assert.equal(config.isPaused, false);
    });

    it("Updates platform config", async () => {
      const newFee = 150_000; // 0.15 USDC

      await program.methods
        .updatePlatformConfig(null, new BN(newFee), null, null)
        .accounts({
          platformConfig,
          authority: deployer.publicKey,
        })
        .rpc();

      const config = await program.account.platformConfig.fetch(platformConfig);
      assert.equal(config.feeAmountUsdc.toNumber(), newFee);
    });

    it("Toggles pause", async () => {
      await program.methods
        .togglePause()
        .accounts({
          platformConfig,
          authority: deployer.publicKey,
        })
        .rpc();

      let config = await program.account.platformConfig.fetch(platformConfig);
      assert.equal(config.isPaused, true);

      // Toggle back
      await program.methods
        .togglePause()
        .accounts({
          platformConfig,
          authority: deployer.publicKey,
        })
        .rpc();

      config = await program.account.platformConfig.fetch(platformConfig);
      assert.equal(config.isPaused, false);
    });

    it("Initializes ticket authority", async () => {
      try {
        // Check if ticket authority is already initialized
        await program.account.ticketAuthority.fetch(ticketAuthorityPda);
        console.log("  ✓ Ticket authority already initialized");
      } catch (e) {
        // If not initialized, initialize it
        await program.methods
          .initializeTicketAuthority()
          .accounts({
            ticketAuthority: ticketAuthorityPda,
            authority: deployer.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        const ticketAuthority = await program.account.ticketAuthority.fetch(ticketAuthorityPda);
        assert.isDefined(ticketAuthority.bump);
        console.log("  ✓ Ticket authority initialized");
      }
    });
  });

  describe("Event Management", () => {
    const now = Math.floor(Date.now() / 1000);
    const startTime = now + 3600; // 1 hour from now (allow check-in)
    const endTime = startTime + 7200; // 2 hours later

    it("Creates an event (Active status by default)", async () => {
      await program.methods
        .createEvent(
          EVENT_ID,
          "ipfs://test-metadata",
          new BN(startTime),
          new BN(endTime),
          new BN(now), // tickets available now
          new BN(600), // stop sales 10 minutes before
          100, // 1% resale fee
          3 // max 3 resales
        )
        .accounts({
          platformConfig,
          event: eventPda,
          organizer: deployer.publicKey,  // deployer is event_admin
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const event = await program.account.eventAccount.fetch(eventPda);
      assert.equal(event.eventId, EVENT_ID);
      assert.equal(event.organizer.toString(), deployer.publicKey.toString());
      assert.equal(event.status, 1); // Active by default
    });

    it("Adds check-in operator", async () => {
      [checkinAuthorityPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("checkin_auth"),
          Buffer.from(EVENT_ID),
          checkinOperator.publicKey.toBuffer(),
        ],
        program.programId
      );

      await program.methods
        .addCheckinOperator(EVENT_ID, checkinOperator.publicKey)
        .accounts({
          platformConfig,
          event: eventPda,
          checkinAuthority: checkinAuthorityPda,
          admin: deployer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const auth = await program.account.checkInAuthority.fetch(checkinAuthorityPda);
      assert.equal(auth.operator.toString(), checkinOperator.publicKey.toString());
      assert.equal(auth.isActive, true);
    });
  });


  describe("Purchase Flow", () => {
    it("Purchases a ticket with backend authorization", async () => {
      const ticketUuid = randomUUID().replace(/-/g, '');  // 32 bytes (no hyphens)

      // Create ticket NFT mint using keypair
      const ticketMintKeypair = Keypair.generate();
      const ticketMint = ticketMintKeypair.publicKey;
      
      // PDA uses UUID instead of sequence number
      [ticketPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("ticket"),
          Buffer.from(EVENT_ID),
          Buffer.from(ticketUuid),
        ],
        program.programId
      );

      // Debug: Print all account public keys
      console.log("=== Debug Account Public Keys ===");
      console.log("buyer.publicKey:", buyer.publicKey.toString());
      console.log("backendAuthority.publicKey:", backendAuthority.publicKey.toString());
      console.log("ticketMint:", ticketMint.toString());
      console.log("organizer.publicKey:", organizer.publicKey.toString());
      console.log("deployer.publicKey:", deployer.publicKey.toString());
      console.log("eventPda:", eventPda.toString());
      console.log("ticketPda:", ticketPda.toString());
      console.log("nonceTracker:", nonceTracker.toString());
      console.log("platformConfig:", platformConfig.toString());
      console.log("ticketAuthorityPda:", ticketAuthorityPda.toString());
      console.log("usdcMint:", usdcMint.toString());
      console.log("buyerUsdcAccount:", buyerUsdcAccount.toString());
      console.log("platformUsdcAccount:", platformUsdcAccount.toString());
      console.log("organizerUsdcAccount:", organizerUsdcAccount.toString());
      console.log("==================================");

      // Create buyer's ticket NFT token account
      buyerTicketAccount = await getAssociatedTokenAddress(
        ticketMint,
        buyer.publicKey
      );

      // Debug: Print additional account public keys
      console.log("buyerTicketAccount:", buyerTicketAccount.toString());
      console.log("TOKEN_2022_PROGRAM_ID:", TOKEN_2022_PROGRAM_ID.toString());
      console.log("SYSVAR_RENT_PUBKEY:", SYSVAR_RENT_PUBKEY.toString());
      console.log("SystemProgram.programId:", SystemProgram.programId.toString());
      console.log("anchor.utils.token.ASSOCIATED_PROGRAM_ID:", anchor.utils.token.ASSOCIATED_PROGRAM_ID.toString());
      console.log("==================================");

      // Create the mint account (this will be done by the program)
      // The program will handle mint creation and set mint authority
      
      // Token 2022 will handle metadata internally

      const buyerBalanceBefore = (
        await getAccount(provider.connection, buyerUsdcAccount)
      ).amount;
      const organizerBalanceBefore = (
        await getAccount(provider.connection, organizerUsdcAccount)
      ).amount;
      const platformBalanceBefore = (
        await getAccount(provider.connection, platformUsdcAccount)
      ).amount;

      // Debug: Print signers
      console.log("=== Debug Signers ===");
      console.log("buyer:", buyer.publicKey.toString());
      console.log("backendAuthority:", backendAuthority.publicKey.toString());
      console.log("ticketMint:", ticketMint.toString());
      console.log("====================");

      await program.methods
        .purchaseTicket(
          EVENT_ID, 
          TICKET_TYPE_ID, 
          ticketUuid, 
          new BN(TICKET_PRICE),
          5, // row_number
          10 // column_number
        )
        .accounts({
          platformConfig,
          backendAuthority: backendAuthority.publicKey,
          event: eventPda,
          ticket: ticketPda,
          nonceTracker,
          buyer: buyer.publicKey,
          ticketMint: ticketMint,
          buyerTicketAccount,
          rent: SYSVAR_RENT_PUBKEY,
          buyerUsdcAccount,
          platformUsdcAccount,
          organizerUsdcAccount,
          usdcMint,
          usdcTokenProgram: TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          ticketAuthority: ticketAuthorityPda,
        })
        .signers([buyer, backendAuthority, ticketMintKeypair])
        .rpc();

      // Verify ticket created
      const ticket = await program.account.ticketAccount.fetch(ticketPda);
      assert.equal(ticket.eventId, EVENT_ID);
      assert.equal(ticket.owner.toString(), buyer.publicKey.toString());
      assert.equal(ticket.ticketUuid, ticketUuid);
      assert.equal(ticket.isCheckedIn, false);
      assert.equal(ticket.rowNumber, 5);
      assert.equal(ticket.columnNumber, 10);

      // Verify USDC transfers
      const buyerBalanceAfter = (
        await getAccount(provider.connection, buyerUsdcAccount)
      ).amount;
      const organizerBalanceAfter = (
        await getAccount(provider.connection, organizerUsdcAccount)
      ).amount;
      const platformBalanceAfter = (
        await getAccount(provider.connection, platformUsdcAccount)
      ).amount;

      const config = await program.account.platformConfig.fetch(platformConfig);
      const platformFee = config.feeAmountUsdc.toNumber();

      assert.equal(
        Number(buyerBalanceBefore - buyerBalanceAfter),
        TICKET_PRICE
      );
      assert.equal(
        Number(organizerBalanceAfter - organizerBalanceBefore),
        TICKET_PRICE - platformFee
      );
      assert.equal(
        Number(platformBalanceAfter - platformBalanceBefore),
        platformFee
      );

      // No more minted count (removed from TicketTypeAccount)
    });

    it("Purchases ticket with PoF (optional)", async () => {
      if (!USE_POF) {
        console.log("  ⏭  Skipping PoF test");
        return;
      }

      const ticketUuid2 = randomUUID().replace(/-/g, '');
      
      // Create ticket NFT mint for PoF test
      const ticketMint2Keypair = Keypair.generate();
      
      const [ticket2Pda] = PublicKey.findProgramAddressSync(
        [Buffer.from("ticket"), Buffer.from(EVENT_ID), Buffer.from(ticketUuid2)],
        program.programId
      );

      // Create buyer's ticket NFT token account
      const buyerTicket2Account = await getAssociatedTokenAddress(
        ticketMint2Keypair.publicKey,
        buyer.publicKey
      );
      
      // Metadata will be handled via event.metadata_uri
      // Token 2022 extension fields will be used for check-in status

      await program.methods
        .purchaseTicket(
          EVENT_ID, 
          TICKET_TYPE_ID, 
          ticketUuid2, 
          new BN(TICKET_PRICE),
          6, // row_number
          11 // column_number
        )
        .accounts({
          platformConfig, backendAuthority: backendAuthority.publicKey, event: eventPda,
          ticket: ticket2Pda, nonceTracker,
          buyer: buyer.publicKey, 
          ticketMint: ticketMint2Keypair.publicKey,
          buyerTicketAccount: buyerTicket2Account,
          rent: SYSVAR_RENT_PUBKEY,
          buyerUsdcAccount,
          platformUsdcAccount, organizerUsdcAccount,
          usdcMint,
          tokenProgram: TOKEN_2022_PROGRAM_ID, 
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          ticketAuthority: ticketAuthorityPda,
        })
        .remainingAccounts([
          { pubkey: getPofWalletPda(buyer.publicKey), isWritable: true, isSigner: false },
          { pubkey: getPofGlobalStatePda(), isWritable: false, isSigner: false },
          { pubkey: POF_PROGRAM_ID, isWritable: false, isSigner: false },
        ])
        .signers([buyer, backendAuthority, ticketMint2Keypair])
        .rpc();

      console.log("  ✓ PoF integration OK");
    });
  });

  describe("Marketplace", () => {
    const RESALE_PRICE = 60_000_000; // 60 USDC

    it("Lists ticket for resale", async () => {
      [listingPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("listing"), ticketPda.toBuffer()],
        program.programId
      );

      await program.methods
        .listTicket(new BN(RESALE_PRICE))
        .accounts({
          event: eventPda,
          ticket: ticketPda,
          listing: listingPda,
          seller: buyer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([buyer])
        .rpc();

      const listing = await program.account.listingAccount.fetch(listingPda);
      assert.equal(listing.originalSeller.toString(), buyer.publicKey.toString());
      assert.equal(listing.price.toNumber(), RESALE_PRICE);
      assert.equal(listing.isActive, true);
      
      // Verify ticket ownership transferred to program
      const ticket = await program.account.ticketAccount.fetch(ticketPda);
      const [programAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from("program_authority")],
        program.programId
      );
      assert.equal(ticket.owner.toString(), programAuthority.toString());
    });

    it("Buys listed ticket", async () => {
      const buyer2BalanceBefore = (
        await getAccount(provider.connection, buyer2UsdcAccount)
      ).amount;
      const buyerBalanceBefore = (
        await getAccount(provider.connection, buyerUsdcAccount)
      ).amount;

      // Create buyer2's ticket NFT token account
      const buyer2TicketAccount = await getAssociatedTokenAddress(
        ticketMintKeypair.publicKey,
        buyer2.publicKey
      );

      await program.methods
        .buyListedTicket(new BN(RESALE_PRICE))
        .accounts({
          platformConfig,
          event: eventPda,
          listing: listingPda,
          ticket: ticketPda,
          nonceTracker,
          buyer: buyer2.publicKey,
          originalSeller: buyer.publicKey,
          ticketMint: ticketMintPda,
          sellerTicketAccount: buyerTicketAccount,
          buyerTicketAccount: buyer2TicketAccount,
          buyerUsdcAccount: buyer2UsdcAccount,
          sellerUsdcAccount: buyerUsdcAccount,
          platformUsdcAccount,
          organizerUsdcAccount,
          usdcMint,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        })
        .signers([buyer2])
        .rpc();

      // Verify ownership transfer
      const ticket = await program.account.ticketAccount.fetch(ticketPda);
      assert.equal(ticket.owner.toString(), buyer2.publicKey.toString());
      assert.equal(ticket.resaleCount, 1);

      // Verify listing closed
      try {
        await program.account.listingAccount.fetch(listingPda);
        assert.fail("Listing should be closed");
      } catch (e) {
        // Expected - account should be closed
      }

      // Verify USDC transfers
      const buyer2BalanceAfter = (
        await getAccount(provider.connection, buyer2UsdcAccount)
      ).amount;
      const buyerBalanceAfter = (
        await getAccount(provider.connection, buyerUsdcAccount)
      ).amount;

      assert.equal(
        Number(buyer2BalanceBefore - buyer2BalanceAfter),
        RESALE_PRICE
      );
      assert.isTrue(Number(buyerBalanceAfter) > Number(buyerBalanceBefore));
    });

    it("Buys listed ticket with PoF (optional)", async () => {
      if (!USE_POF) {
        console.log("  ⏭  Skipping PoF test");
        return;
      }

      // Create a new listing (buyer2 is current owner after previous test)
      const [newListingPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("listing"), ticketPda.toBuffer()],
        program.programId
      );

      await program.methods
        .listTicket(new BN(RESALE_PRICE))
        .accounts({
          event: eventPda, ticket: ticketPda, listing: newListingPda,
          seller: buyer2.publicKey, systemProgram: SystemProgram.programId,
        })
        .signers([buyer2])
        .rpc();

      // Buy with PoF integration (no backend authorization needed)
      // Create buyer's ticket NFT token account for this resale
      const buyerTicketAccountForResale = await getAssociatedTokenAddress(
        ticketMintKeypair.publicKey,
        buyer.publicKey
      );

      await program.methods
        .buyListedTicket(new BN(RESALE_PRICE))
        .accounts({
          platformConfig, event: eventPda, listing: newListingPda,
          ticket: ticketPda, nonceTracker,
          buyer: buyer.publicKey, originalSeller: buyer2.publicKey,
          ticketMint: ticketMintPda,
          sellerTicketAccount: buyer2TicketAccount,
          buyerTicketAccount: buyerTicketAccountForResale,
          buyerUsdcAccount, sellerUsdcAccount: buyer2UsdcAccount,
          platformUsdcAccount, organizerUsdcAccount,
          usdcMint,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        })
        .remainingAccounts([
          { pubkey: getPofWalletPda(buyer2.publicKey), isWritable: true, isSigner: false }, // seller
          { pubkey: getPofWalletPda(buyer.publicKey), isWritable: true, isSigner: false },  // buyer
          { pubkey: getPofGlobalStatePda(), isWritable: false, isSigner: false },
          { pubkey: POF_PROGRAM_ID, isWritable: false, isSigner: false },
        ])
        .signers([buyer])
        .rpc();

      console.log("  ✓ PoF resale integration OK");
    });

    it("Cancels listing", async () => {
      // Create another listing first
      const [newListingPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("listing"), ticketPda.toBuffer()],
        program.programId
      );

      await program.methods
        .listTicket(new BN(RESALE_PRICE))
        .accounts({
          event: eventPda,
          ticket: ticketPda,
          listing: newListingPda,
          seller: buyer2.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([buyer2])
        .rpc();

      // Cancel it
      await program.methods
        .cancelListing()
        .accounts({
          listing: newListingPda,
          ticket: ticketPda,
          seller: buyer2.publicKey,
          ticketMint: ticketMintPda,
          programTicketAccount: buyerTicketAccount, // This should be program's account in real scenario
          sellerTicketAccount: buyerTicketAccount,
        })
        .signers([buyer2])
        .rpc();

      // Verify listing closed
      try {
        await program.account.listingAccount.fetch(newListingPda);
        assert.fail("Listing should be closed");
      } catch (e) {
        // Expected
      }
      
      // Verify ticket ownership returned to seller
      const ticket = await program.account.ticketAccount.fetch(ticketPda);
      assert.equal(ticket.owner.toString(), buyer2.publicKey.toString());
    });

    it("Cannot check-in a listed ticket", async () => {
      // List the ticket again
      const [testListingPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("listing"), ticketPda.toBuffer()],
        program.programId
      );

      await program.methods
        .listTicket(new BN(RESALE_PRICE))
        .accounts({
          event: eventPda,
          ticket: ticketPda,
          listing: testListingPda,
          seller: buyer2.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([buyer2])
        .rpc();

      // Try to check-in (should fail - owner is program_authority, not user)
      try {
        await program.methods
          .checkInTicket(EVENT_ID)
          .accounts({
            event: eventPda,
            checkinAuthority: checkinAuthorityPda,
            ticket: ticketPda,
            ticketMint: ticketMintPda,
            ticketOwnerTokenAccount: buyerTicketAccount,
            operator: checkinOperator.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .signers([checkinOperator])
          .rpc();
        assert.fail("Should not check in a listed ticket");
      } catch (e) {
        // Expected - ticket is owned by program_authority
      }

      // Cancel listing for cleanup
      await program.methods
        .cancelListing()
        .accounts({
          listing: testListingPda,
          ticket: ticketPda,
          seller: buyer2.publicKey,
        })
        .signers([buyer2])
        .rpc();
    });
  });

  describe("Check-in Flow", () => {
    it("Checks in a ticket", async () => {
      // Reset ticket check-in status for clean test
      const ticket = await program.account.ticketAccount.fetch(ticketPda);
      if (ticket.isCheckedIn) {
        // If ticket is already checked in, we need to create a new ticket for this test
        const ticketUuid4 = randomUUID().replace(/-/g, '');

        const [newTicketPda] = PublicKey.findProgramAddressSync(
          [Buffer.from("ticket"), Buffer.from(EVENT_ID), Buffer.from(ticketUuid4)],
          program.programId
        );

        // Create ticket NFT mint for check-in test
        const ticketMint4Keypair = Keypair.generate();
        const buyer2Ticket4Account = await getAssociatedTokenAddress(
          ticketMint4Keypair.publicKey,
          buyer2.publicKey
        );
        // Metadata will be handled via event.metadata_uri
      // Token 2022 extension fields will be used for check-in status

        await program.methods
          .purchaseTicket(
            EVENT_ID, 
            TICKET_TYPE_ID, 
            ticketUuid4, 
            new BN(TICKET_PRICE),
            8, // row_number
            13 // column_number
          )
          .accounts({
            platformConfig,
            backendAuthority: backendAuthority.publicKey,
            event: eventPda,
            ticket: newTicketPda,
            nonceTracker,
            buyer: buyer2.publicKey,
            ticketMint: ticketMint4Keypair.publicKey,
            buyerTicketAccount: buyer2Ticket4Account,
            buyerUsdcAccount: buyer2UsdcAccount,
            platformUsdcAccount,
            organizerUsdcAccount,
            usdcMint,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            ticketAuthority: ticketAuthorityPda,
          })
          .signers([buyer2, backendAuthority, ticketMint4Keypair])
          .rpc();

        // Use the new ticket for check-in
        await program.methods
          .checkInTicket(EVENT_ID)
          .accounts({
            event: eventPda,
            checkinAuthority: checkinAuthorityPda,
            ticket: newTicketPda,
            ticketMint: ticketMint4Keypair.publicKey,
            ticketOwnerTokenAccount: buyer2Ticket4Account,
            operator: checkinOperator.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .signers([checkinOperator])
          .rpc();

        const newTicket = await program.account.ticketAccount.fetch(newTicketPda);
        assert.equal(newTicket.isCheckedIn, true);
      } else {
        // Original ticket is not checked in, proceed normally
        await program.methods
          .checkInTicket(EVENT_ID)
          .accounts({
            event: eventPda,
            checkinAuthority: checkinAuthorityPda,
            ticket: ticketPda,
            ticketMint: ticketMintPda,
            ticketOwnerTokenAccount: buyerTicketAccount,
            operator: checkinOperator.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .signers([checkinOperator])
          .rpc();

        const ticket = await program.account.ticketAccount.fetch(ticketPda);
        assert.equal(ticket.isCheckedIn, true);
      }
    });

    it("Checks in with PoF (optional)", async () => {
      if (!USE_POF) {
        console.log("  ⏭  Skipping PoF test");
        return;
      }

      // Buy a new ticket first (for PoF test)
      const ticketUuid3 = randomUUID().replace(/-/g, '');

      const [newTicketPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("ticket"), Buffer.from(EVENT_ID), Buffer.from(ticketUuid3)],
        program.programId
      );

      // Create ticket NFT mint for PoF check-in test
      const ticketMint3Keypair = Keypair.generate();
      const buyer2Ticket3Account = await getAssociatedTokenAddress(
        ticketMint3Keypair.publicKey,
        buyer2.publicKey
      );
      // Metadata will be handled via event.metadata_uri
      // Token 2022 extension fields will be used for check-in status

      await program.methods
        .purchaseTicket(
          EVENT_ID, 
          TICKET_TYPE_ID, 
          ticketUuid3, 
          new BN(TICKET_PRICE),
          7, // row_number
          12 // column_number
        )
        .accounts({
          platformConfig, backendAuthority: backendAuthority.publicKey, event: eventPda,
          ticket: newTicketPda, nonceTracker,
          buyer: buyer2.publicKey, 
          ticketMint: ticketMint3Keypair.publicKey,
          buyerTicketAccount: buyer2Ticket3Account,
          buyerUsdcAccount: buyer2UsdcAccount,
          platformUsdcAccount, organizerUsdcAccount,
          usdcMint,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          ticketAuthority: ticketAuthorityPda,
        })
        .signers([buyer2, backendAuthority, ticketMint3Keypair])
        .rpc();

      // Check-in with PoF
      await program.methods
        .checkInTicket(EVENT_ID)
        .accounts({
          event: eventPda, checkinAuthority: checkinAuthorityPda,
          ticket: newTicketPda, 
          ticketMint: ticketMint3Keypair.publicKey,
          ticketOwnerTokenAccount: buyer2Ticket3Account,
          operator: checkinOperator.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .remainingAccounts([
          { pubkey: getPofWalletPda(buyer2.publicKey), isWritable: true, isSigner: false },
          { pubkey: getPofGlobalStatePda(), isWritable: false, isSigner: false },
          { pubkey: POF_PROGRAM_ID, isWritable: false, isSigner: false },
        ])
        .signers([checkinOperator])
        .rpc();

      console.log("  ✓ PoF check-in integration OK");
    });

    it("Fails to check in already checked ticket", async () => {
      try {
        await program.methods
          .checkInTicket(EVENT_ID)
          .accounts({
            event: eventPda,
            checkinAuthority: checkinAuthorityPda,
            ticket: ticketPda,
            ticketMint: ticketMintPda,
            ticketOwnerTokenAccount: buyerTicketAccount,
            operator: checkinOperator.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .signers([checkinOperator])
          .rpc();
        assert.fail("Should not check in already checked ticket");
      } catch (e) {
        assert.include(e.toString(), "AlreadyCheckedIn");
      }
    });
  });
});
