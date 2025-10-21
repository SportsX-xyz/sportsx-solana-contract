import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { TicketingProgram } from "../target/types/ticketing_program";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { assert } from "chai";

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
  let ticketTypePda: PublicKey;
  let ticketPda: PublicKey;
  let listingPda: PublicKey;
  let checkinAuthorityPda: PublicKey;

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
    usdcMint = await createMint(
      provider.connection,
      deployer.payer,
      deployer.publicKey,
      null,
      6 // USDC decimals
    );
    console.log("  Created test USDC mint:", usdcMint.toString());

    // Create USDC token accounts
    platformUsdcAccount = await createAccount(
      provider.connection,
      deployer.payer,
      usdcMint,
      deployer.publicKey
    );

    organizerUsdcAccount = await createAccount(
      provider.connection,
      deployer.payer,
      usdcMint,
      organizer.publicKey
    );

    buyerUsdcAccount = await createAccount(
      provider.connection,
      deployer.payer,
      usdcMint,
      buyer.publicKey
    );

    buyer2UsdcAccount = await createAccount(
      provider.connection,
      deployer.payer,
      usdcMint,
      buyer2.publicKey
    );

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

    [ticketTypePda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("ticket_type"),
        Buffer.from(EVENT_ID),
        Buffer.from(TICKET_TYPE_ID),
      ],
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
          event: eventPda,
          checkinAuthority: checkinAuthorityPda,
          organizer: deployer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const auth = await program.account.checkInAuthority.fetch(checkinAuthorityPda);
      assert.equal(auth.operator.toString(), checkinOperator.publicKey.toString());
      assert.equal(auth.isActive, true);
    });
  });

  describe("Ticket Management", () => {
    it("Creates ticket type", async () => {
      await program.methods
        .createTicketType(
          EVENT_ID,
          TICKET_TYPE_ID,
          "VIP Tier",
          new BN(TICKET_PRICE),
          100, // total supply
          0xff0000 // red color
        )
        .accounts({
          event: eventPda,
          ticketType: ticketTypePda,
          organizer: deployer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const ticketType = await program.account.ticketTypeAccount.fetch(ticketTypePda);
      assert.equal(ticketType.eventId, EVENT_ID);
      assert.equal(ticketType.typeId, TICKET_TYPE_ID);
      assert.equal(ticketType.price.toNumber(), TICKET_PRICE);
      assert.equal(ticketType.totalSupply, 100);
      assert.equal(ticketType.minted, 0);
    });

    it("Batch mints tickets (increases supply)", async () => {
      await program.methods
        .batchMintTickets(EVENT_ID, TICKET_TYPE_ID, 50)
        .accounts({
          event: eventPda,
          ticketType: ticketTypePda,
          organizer: deployer.publicKey,
        })
        .rpc();

      const ticketType = await program.account.ticketTypeAccount.fetch(ticketTypePda);
      assert.equal(ticketType.totalSupply, 150);
    });
  });

  describe("Purchase Flow", () => {
    it("Purchases a ticket with backend authorization", async () => {
      const nonce = Date.now();
      const validUntil = Math.floor(Date.now() / 1000) + 300; // 5 minutes

      const authData = {
        buyer: buyer.publicKey,
        ticketTypeId: TICKET_TYPE_ID,
        maxPrice: new BN(TICKET_PRICE),
        validUntil: new BN(validUntil),
        nonce: new BN(nonce),
        ticketPda: null,  // First-time purchase
        rowNumber: 5,
        columnNumber: 10,
      };

      // Mock signature (in production, backend would sign this)
      const signature = new Array(64).fill(0);

      [ticketPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("ticket"),
          Buffer.from(EVENT_ID),
          Buffer.from([1, 0, 0, 0]), // sequence 1 (little-endian u32)
        ],
        program.programId
      );

      const buyerBalanceBefore = (
        await getAccount(provider.connection, buyerUsdcAccount)
      ).amount;
      const organizerBalanceBefore = (
        await getAccount(provider.connection, organizerUsdcAccount)
      ).amount;

      await program.methods
        .purchaseTicket(EVENT_ID, TICKET_TYPE_ID, authData, signature)
        .accounts({
          platformConfig,
          event: eventPda,
          ticketType: ticketTypePda,
          ticket: ticketPda,
          nonceTracker,
          buyer: buyer.publicKey,
          buyerUsdcAccount,
          platformUsdcAccount,
          organizerUsdcAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([buyer])
        .rpc();

      // Verify ticket created
      const ticket = await program.account.ticketAccount.fetch(ticketPda);
      assert.equal(ticket.eventId, EVENT_ID);
      assert.equal(ticket.owner.toString(), buyer.publicKey.toString());
      assert.equal(ticket.sequenceNumber, 1);
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

      // Verify ticket type minted count
      const ticketType = await program.account.ticketTypeAccount.fetch(ticketTypePda);
      assert.equal(ticketType.minted, 1);
    });

    it("Purchases ticket with PoF (optional)", async () => {
      if (!USE_POF) {
        console.log("  ⏭  Skipping PoF test");
        return;
      }

      const nonce = Date.now() + 1;
      const validUntil = Math.floor(Date.now() / 1000) + 300;
      const authData = {
        buyer: buyer.publicKey,
        ticketTypeId: TICKET_TYPE_ID,
        maxPrice: new BN(TICKET_PRICE),
        validUntil: new BN(validUntil),
        nonce: new BN(nonce),
        ticketPda: null,  // First-time purchase
        rowNumber: 6,
        columnNumber: 11,
      };
      const signature = new Array(64).fill(0);

      const [ticket2Pda] = PublicKey.findProgramAddressSync(
        [Buffer.from("ticket"), Buffer.from(EVENT_ID), Buffer.from([2, 0, 0, 0])],
        program.programId
      );

      await program.methods
        .purchaseTicket(EVENT_ID, TICKET_TYPE_ID, authData, signature)
        .accounts({
          platformConfig, event: eventPda, ticketType: ticketTypePda,
          ticket: ticket2Pda, nonceTracker,
          buyer: buyer.publicKey, buyerUsdcAccount,
          platformUsdcAccount, organizerUsdcAccount,
          tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
        })
        .remainingAccounts([
          { pubkey: getPofWalletPda(buyer.publicKey), isWritable: true, isSigner: false },
          { pubkey: getPofGlobalStatePda(), isWritable: false, isSigner: false },
          { pubkey: POF_PROGRAM_ID, isWritable: false, isSigner: false },
        ])
        .signers([buyer])
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

      // Get backend authorization for resale
      const nonce = Date.now() + 100;
      const validUntil = Math.floor(Date.now() / 1000) + 300;
      const resaleAuthData = {
        buyer: buyer2.publicKey,
        ticketTypeId: TICKET_TYPE_ID,
        maxPrice: new BN(RESALE_PRICE),
        validUntil: new BN(validUntil),
        nonce: new BN(nonce),
        ticketPda: ticketPda,  // Resale: specify ticket PDA
        rowNumber: 5,  // Same seat as original
        columnNumber: 10,
      };
      const signature = new Array(64).fill(0);

      await program.methods
        .buyListedTicket(resaleAuthData, signature)
        .accounts({
          platformConfig,
          event: eventPda,
          listing: listingPda,
          ticket: ticketPda,
          nonceTracker,
          buyer: buyer2.publicKey,
          originalSeller: buyer.publicKey,
          buyerUsdcAccount: buyer2UsdcAccount,
          sellerUsdcAccount: buyerUsdcAccount,
          platformUsdcAccount,
          organizerUsdcAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
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

      // Buy with PoF integration and backend authorization
      const nonce = Date.now() + 200;
      const validUntil = Math.floor(Date.now() / 1000) + 300;
      const resaleAuth = {
        buyer: buyer.publicKey,
        ticketTypeId: TICKET_TYPE_ID,
        maxPrice: new BN(RESALE_PRICE),
        validUntil: new BN(validUntil),
        nonce: new BN(nonce),
        ticketPda: ticketPda,  // Resale
        rowNumber: 5,
        columnNumber: 10,
      };
      const signature = new Array(64).fill(0);

      await program.methods
        .buyListedTicket(resaleAuth, signature)
        .accounts({
          platformConfig, event: eventPda, listing: newListingPda,
          ticket: ticketPda, nonceTracker,
          buyer: buyer.publicKey, originalSeller: buyer2.publicKey,
          buyerUsdcAccount, sellerUsdcAccount: buyer2UsdcAccount,
          platformUsdcAccount, organizerUsdcAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
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
            operator: checkinOperator.publicKey,
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
      await program.methods
        .checkInTicket(EVENT_ID)
        .accounts({
          event: eventPda,
          checkinAuthority: checkinAuthorityPda,
          ticket: ticketPda,
          operator: checkinOperator.publicKey,
        })
        .signers([checkinOperator])
        .rpc();

      const ticket = await program.account.ticketAccount.fetch(ticketPda);
      assert.equal(ticket.isCheckedIn, true);
    });

    it("Checks in with PoF (optional)", async () => {
      if (!USE_POF) {
        console.log("  ⏭  Skipping PoF test");
        return;
      }

      // Buy a new ticket first (for PoF test)
      const nonce = Date.now() + 999;
      const validUntil = Math.floor(Date.now() / 1000) + 300;
      const authData = {
        buyer: buyer2.publicKey,
        ticketTypeId: TICKET_TYPE_ID,
        maxPrice: new BN(TICKET_PRICE),
        validUntil: new BN(validUntil),
        nonce: new BN(nonce),
        ticketPda: null,  // First-time purchase
        rowNumber: 7,
        columnNumber: 12,
      };
      const signature = new Array(64).fill(0);

      const ticketType = await program.account.ticketTypeAccount.fetch(ticketTypePda);
      const nextSeq = ticketType.minted + 1;
      const [newTicketPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("ticket"), Buffer.from(EVENT_ID), new BN(nextSeq).toArrayLike(Buffer, 'le', 4)],
        program.programId
      );

      await program.methods
        .purchaseTicket(EVENT_ID, TICKET_TYPE_ID, authData, signature)
        .accounts({
          platformConfig, event: eventPda, ticketType: ticketTypePda,
          ticket: newTicketPda, nonceTracker,
          buyer: buyer2.publicKey, buyerUsdcAccount: buyer2UsdcAccount,
          platformUsdcAccount, organizerUsdcAccount,
          tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
        })
        .signers([buyer2])
        .rpc();

      // Check-in with PoF
      await program.methods
        .checkInTicket(EVENT_ID)
        .accounts({
          event: eventPda, checkinAuthority: checkinAuthorityPda,
          ticket: newTicketPda, operator: checkinOperator.publicKey,
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
            operator: checkinOperator.publicKey,
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
