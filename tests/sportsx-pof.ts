import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SportsxPof } from "../target/types/sportsx_pof";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";

describe("sportsx-pof", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SportsxPof as Program<SportsxPof>;
  const admin = provider.wallet;

  // Test wallets
  const wallet1 = Keypair.generate();
  const wallet2 = Keypair.generate();
  const unauthorizedUser = Keypair.generate();
  const mockContract = Keypair.generate();

  // PDAs
  let globalStatePda: PublicKey;
  let globalStateBump: number;
  let wallet1PointsPda: PublicKey;
  let wallet1PointsBump: number;
  let wallet2PointsPda: PublicKey;
  let wallet2PointsBump: number;

  before(async () => {
    // Derive PDAs
    [globalStatePda, globalStateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("global_state")],
      program.programId
    );

    [wallet1PointsPda, wallet1PointsBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("wallet_points"), wallet1.publicKey.toBuffer()],
      program.programId
    );

    [wallet2PointsPda, wallet2PointsBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("wallet_points"), wallet2.publicKey.toBuffer()],
      program.programId
    );

    // Airdrop SOL to test accounts
    const airdropAmount = 2 * anchor.web3.LAMPORTS_PER_SOL;
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(wallet1.publicKey, airdropAmount)
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(wallet2.publicKey, airdropAmount)
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(unauthorizedUser.publicKey, airdropAmount)
    );
  });

  it("Initializes global state", async () => {
    await program.methods
      .initialize()
      .accounts({
        globalState: globalStatePda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const globalState = await program.account.globalState.fetch(globalStatePda);
    expect(globalState.admin.toString()).to.equal(admin.publicKey.toString());
    expect(globalState.authorizedContracts).to.be.empty;
  });

  it("Initializes wallet1 points account", async () => {
    await program.methods
      .initializeWallet()
      .accounts({
        walletPoints: wallet1PointsPda,
        wallet: wallet1.publicKey,
        payer: wallet1.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([wallet1])
      .rpc();

    const walletPoints = await program.account.walletPoints.fetch(wallet1PointsPda);
    expect(walletPoints.wallet.toString()).to.equal(wallet1.publicKey.toString());
    expect(walletPoints.points.toNumber()).to.equal(0);
  });

  it("Initializes wallet2 points account", async () => {
    await program.methods
      .initializeWallet()
      .accounts({
        walletPoints: wallet2PointsPda,
        wallet: wallet2.publicKey,
        payer: wallet2.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([wallet2])
      .rpc();

    const walletPoints = await program.account.walletPoints.fetch(wallet2PointsPda);
    expect(walletPoints.wallet.toString()).to.equal(wallet2.publicKey.toString());
    expect(walletPoints.points.toNumber()).to.equal(0);
  });

  it("Admin can update wallet points (increase)", async () => {
    await program.methods
      .updatePoints(new anchor.BN(100))
      .accounts({
        walletPoints: wallet1PointsPda,
        globalState: globalStatePda,
        authority: admin.publicKey,
      })
      .rpc();

    const walletPoints = await program.account.walletPoints.fetch(wallet1PointsPda);
    expect(walletPoints.points.toNumber()).to.equal(100);
  });

  it("Admin can update wallet points (decrease)", async () => {
    await program.methods
      .updatePoints(new anchor.BN(-30))
      .accounts({
        walletPoints: wallet1PointsPda,
        globalState: globalStatePda,
        authority: admin.publicKey,
      })
      .rpc();

    const walletPoints = await program.account.walletPoints.fetch(wallet1PointsPda);
    expect(walletPoints.points.toNumber()).to.equal(70);
  });

  it("Query wallet points", async () => {
    const result = await program.methods
      .getPoints()
      .accounts({
        walletPoints: wallet1PointsPda,
      })
      .view();

    expect(result.toNumber()).to.equal(70);
  });

  it("Fails when unauthorized user tries to update points", async () => {
    try {
      await program.methods
        .updatePoints(new anchor.BN(50))
        .accounts({
          walletPoints: wallet1PointsPda,
          globalState: globalStatePda,
          authority: unauthorizedUser.publicKey,
        })
        .signers([unauthorizedUser])
        .rpc();
      expect.fail("Should have thrown an error");
    } catch (error) {
      expect(error.toString()).to.include("Unauthorized");
    }
  });

  it("Fails when trying to decrease points below zero", async () => {
    try {
      await program.methods
        .updatePoints(new anchor.BN(-100))
        .accounts({
          walletPoints: wallet1PointsPda,
          globalState: globalStatePda,
          authority: admin.publicKey,
        })
        .rpc();
      expect.fail("Should have thrown an error");
    } catch (error) {
      expect(error.toString()).to.include("InsufficientPoints");
    }
  });

  it("Admin can authorize a contract", async () => {
    await program.methods
      .authorizeContract(mockContract.publicKey)
      .accounts({
        globalState: globalStatePda,
        admin: admin.publicKey,
      })
      .rpc();

    const globalState = await program.account.globalState.fetch(globalStatePda);
    expect(globalState.authorizedContracts).to.have.lengthOf(1);
    expect(globalState.authorizedContracts[0].toString()).to.equal(
      mockContract.publicKey.toString()
    );
  });

  it("Authorized contract can update points", async () => {
    await program.methods
      .updatePoints(new anchor.BN(30))
      .accounts({
        walletPoints: wallet2PointsPda,
        globalState: globalStatePda,
        authority: mockContract.publicKey,
      })
      .signers([mockContract])
      .rpc();

    const walletPoints = await program.account.walletPoints.fetch(wallet2PointsPda);
    expect(walletPoints.points.toNumber()).to.equal(30);
  });

  it("Admin can revoke contract authorization", async () => {
    await program.methods
      .revokeContract(mockContract.publicKey)
      .accounts({
        globalState: globalStatePda,
        admin: admin.publicKey,
      })
      .rpc();

    const globalState = await program.account.globalState.fetch(globalStatePda);
    expect(globalState.authorizedContracts).to.be.empty;
  });

  it("Revoked contract cannot update points", async () => {
    try {
      await program.methods
        .updatePoints(new anchor.BN(50))
        .accounts({
          walletPoints: wallet2PointsPda,
          globalState: globalStatePda,
          authority: mockContract.publicKey,
        })
        .signers([mockContract])
        .rpc();
      expect.fail("Should have thrown an error");
    } catch (error) {
      expect(error.toString()).to.include("Unauthorized");
    }
  });

  it("Fails when non-admin tries to authorize contract", async () => {
    try {
      await program.methods
        .authorizeContract(mockContract.publicKey)
        .accounts({
          globalState: globalStatePda,
          admin: unauthorizedUser.publicKey,
        })
        .signers([unauthorizedUser])
        .rpc();
      expect.fail("Should have thrown an error");
    } catch (error) {
      expect(error.toString()).to.include("AnchorError");
    }
  });

  it("Fails when trying to authorize the same contract twice", async () => {
    // First authorization
    await program.methods
      .authorizeContract(mockContract.publicKey)
      .accounts({
        globalState: globalStatePda,
        admin: admin.publicKey,
      })
      .rpc();

    // Try to authorize again
    try {
      await program.methods
        .authorizeContract(mockContract.publicKey)
        .accounts({
          globalState: globalStatePda,
          admin: admin.publicKey,
        })
        .rpc();
      expect.fail("Should have thrown an error");
    } catch (error) {
      expect(error.toString()).to.include("ContractAlreadyAuthorized");
    }
  });
});

