import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SportsxPof } from "../target/types/sportsx_pof";
import { SportsxCheckin } from "../target/types/sportsx_checkin";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";

describe("sportsx-checkin", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const pofProgram = anchor.workspace.SportsxPof as Program<SportsxPof>;
  const checkinProgram = anchor.workspace.SportsxCheckin as Program<SportsxCheckin>;
  const admin = provider.wallet;

  // Test users
  const user1 = Keypair.generate();
  const user2 = Keypair.generate();

  // PDAs for PoF
  let globalStatePda: PublicKey;
  let user1PointsPda: PublicKey;
  let user2PointsPda: PublicKey;

  // PDAs for Check-in
  let user1CheckinPda: PublicKey;
  let user1CheckinAuthorityPda: PublicKey;
  let user2CheckinPda: PublicKey;
  let user2CheckinAuthorityPda: PublicKey;

  before(async () => {
    // Derive PoF PDAs
    [globalStatePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("global_state")],
      pofProgram.programId
    );

    [user1PointsPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("wallet_points"), user1.publicKey.toBuffer()],
      pofProgram.programId
    );

    [user2PointsPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("wallet_points"), user2.publicKey.toBuffer()],
      pofProgram.programId
    );

    // Derive Check-in PDAs
    [user1CheckinPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("checkin_record"), user1.publicKey.toBuffer()],
      checkinProgram.programId
    );

    [user1CheckinAuthorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("checkin_authority"), user1.publicKey.toBuffer()],
      checkinProgram.programId
    );

    [user2CheckinPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("checkin_record"), user2.publicKey.toBuffer()],
      checkinProgram.programId
    );

    [user2CheckinAuthorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("checkin_authority"), user2.publicKey.toBuffer()],
      checkinProgram.programId
    );

    // Airdrop SOL
    const airdropAmount = 2 * anchor.web3.LAMPORTS_PER_SOL;
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(user1.publicKey, airdropAmount)
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(user2.publicKey, airdropAmount)
    );
  });

  describe("Setup", () => {
    it("Initializes PoF global state", async () => {
      await pofProgram.methods
        .initialize()
        .accounts({
          globalState: globalStatePda,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const globalState = await pofProgram.account.globalState.fetch(globalStatePda);
      expect(globalState.admin.toString()).to.equal(admin.publicKey.toString());
    });

    it("Authorizes check-in contract in PoF", async () => {
      await pofProgram.methods
        .authorizeContract(user1CheckinAuthorityPda)
        .accounts({
          globalState: globalStatePda,
          admin: admin.publicKey,
        })
        .rpc();

      await pofProgram.methods
        .authorizeContract(user2CheckinAuthorityPda)
        .accounts({
          globalState: globalStatePda,
          admin: admin.publicKey,
        })
        .rpc();

      const globalState = await pofProgram.account.globalState.fetch(globalStatePda);
      expect(globalState.authorizedContracts).to.have.lengthOf(2);
    });

    it("Initializes user1 PoF points", async () => {
      await pofProgram.methods
        .initializeWallet()
        .accounts({
          walletPoints: user1PointsPda,
          wallet: user1.publicKey,
          payer: user1.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user1])
        .rpc();

      const points = await pofProgram.account.walletPoints.fetch(user1PointsPda);
      expect(points.points.toNumber()).to.equal(0);
    });

    it("Initializes user2 PoF points", async () => {
      await pofProgram.methods
        .initializeWallet()
        .accounts({
          walletPoints: user2PointsPda,
          wallet: user2.publicKey,
          payer: user2.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user2])
        .rpc();

      const points = await pofProgram.account.walletPoints.fetch(user2PointsPda);
      expect(points.points.toNumber()).to.equal(0);
    });
  });

  describe("Check-in Functionality", () => {
    it("Initializes user1 check-in record", async () => {
      await checkinProgram.methods
        .initializeCheckin()
        .accounts({
          checkinRecord: user1CheckinPda,
          wallet: user1.publicKey,
          payer: user1.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user1])
        .rpc();

      const record = await checkinProgram.account.checkinRecord.fetch(user1CheckinPda);
      expect(record.wallet.toString()).to.equal(user1.publicKey.toString());
      expect(record.lastCheckin.toNumber()).to.equal(0);
      expect(record.totalCheckins.toNumber()).to.equal(0);
    });

    it("User1 performs first check-in and receives 10 points", async () => {
      await checkinProgram.methods
        .dailyCheckin()
        .accounts({
          checkinRecord: user1CheckinPda,
          wallet: user1.publicKey,
          checkinAuthority: user1CheckinAuthorityPda,
          walletPoints: user1PointsPda,
          globalState: globalStatePda,
          pofProgram: pofProgram.programId,
          systemProgram: SystemProgram.programId,
        })
        .signers([user1])
        .rpc();

      // Check check-in record
      const record = await checkinProgram.account.checkinRecord.fetch(user1CheckinPda);
      expect(record.totalCheckins.toNumber()).to.equal(1);
      expect(record.lastCheckin.toNumber()).to.be.greaterThan(0);

      // Check points awarded
      const points = await pofProgram.account.walletPoints.fetch(user1PointsPda);
      expect(points.points.toNumber()).to.equal(10);
    });

    it("Cannot check in twice within 24 hours", async () => {
      try {
        await checkinProgram.methods
          .dailyCheckin()
          .accounts({
            checkinRecord: user1CheckinPda,
            wallet: user1.publicKey,
            checkinAuthority: user1CheckinAuthorityPda,
            walletPoints: user1PointsPda,
            globalState: globalStatePda,
            pofProgram: pofProgram.programId,
            systemProgram: SystemProgram.programId,
          })
          .signers([user1])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (error) {
        expect(error.toString()).to.include("CheckinTooSoon");
      }
    });

    it("Get check-in info returns correct status", async () => {
      const info = await checkinProgram.methods
        .getCheckinInfo()
        .accounts({
          checkinRecord: user1CheckinPda,
        })
        .view();

      expect(info.totalCheckins.toNumber()).to.equal(1);
      expect(info.lastCheckin.toNumber()).to.be.greaterThan(0);
      expect(info.canCheckin).to.be.false;
      expect(info.timeUntilNextCheckin.toNumber()).to.be.greaterThan(0);
    });

    it("Initializes and performs check-in for user2", async () => {
      // Initialize check-in record
      await checkinProgram.methods
        .initializeCheckin()
        .accounts({
          checkinRecord: user2CheckinPda,
          wallet: user2.publicKey,
          payer: user2.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user2])
        .rpc();

      // Perform check-in
      await checkinProgram.methods
        .dailyCheckin()
        .accounts({
          checkinRecord: user2CheckinPda,
          wallet: user2.publicKey,
          checkinAuthority: user2CheckinAuthorityPda,
          walletPoints: user2PointsPda,
          globalState: globalStatePda,
          pofProgram: pofProgram.programId,
          systemProgram: SystemProgram.programId,
        })
        .signers([user2])
        .rpc();

      // Verify
      const record = await checkinProgram.account.checkinRecord.fetch(user2CheckinPda);
      expect(record.totalCheckins.toNumber()).to.equal(1);

      const points = await pofProgram.account.walletPoints.fetch(user2PointsPda);
      expect(points.points.toNumber()).to.equal(10);
    });

    it("User1 info shows cannot check in yet", async () => {
      const info = await checkinProgram.methods
        .getCheckinInfo()
        .accounts({
          checkinRecord: user1CheckinPda,
        })
        .view();

      expect(info.canCheckin).to.be.false;
      // Should have ~24 hours remaining
      expect(info.timeUntilNextCheckin.toNumber()).to.be.greaterThan(86300);
    });
  });

  describe("Edge Cases", () => {
    it("Cannot initialize check-in record twice", async () => {
      try {
        await checkinProgram.methods
          .initializeCheckin()
          .accounts({
            checkinRecord: user1CheckinPda,
            wallet: user1.publicKey,
            payer: user1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([user1])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (error) {
        // Should fail because account already exists
        expect(error).to.exist;
      }
    });
  });
});

