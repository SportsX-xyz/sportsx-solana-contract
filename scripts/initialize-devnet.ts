import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { TicketingProgram } from "../target/types/ticketing_program";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import * as fs from "fs";

/**
 * Initialize platform on devnet
 */
async function main() {
  // Connect to devnet
  const connection = new anchor.web3.Connection(
    "https://api.devnet.solana.com",
    "confirmed"
  );

  // Load wallet
  const walletPath = process.env.HOME + "/.config/solana/id.json";
  const walletKeypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(walletPath, "utf-8")))
  );

  const wallet = new anchor.Wallet(walletKeypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  // Load program
  const programId = new PublicKey("EFuMNTn1zfn6Zhvdq1Vjaxs83sz2gTWvDgjuJcKDYjhw");
  const idl = JSON.parse(
    fs.readFileSync("./target/idl/ticketing_program.json", "utf-8")
  );
  const program = new Program(idl, provider) as Program<TicketingProgram>;

  console.log("🚀 Initializing SportsX Ticketing Platform on Devnet");
  console.log("Program ID:", programId.toString());
  console.log("Deployer:", wallet.publicKey.toString());
  console.log("");

  // Derive PDAs
  const [platformConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from("platform_config")],
    program.programId
  );

  const [nonceTracker] = PublicKey.findProgramAddressSync(
    [Buffer.from("nonce_tracker")],
    program.programId
  );

  console.log("Platform Config PDA:", platformConfig.toString());
  console.log("Nonce Tracker PDA:", nonceTracker.toString());
  console.log("");

  // Check if already initialized
  try {
    const existingConfig = await program.account.platformConfig.fetch(platformConfig);
    console.log("⚠️  Platform already initialized!");
    console.log("Current config:");
    console.log("  - Fee Receiver:", existingConfig.feeReceiver.toString());
    console.log("  - Platform Fee:", existingConfig.feeAmountUsdc.toNumber() / 1_000_000, "USDC");
    console.log("  - Backend Authority:", existingConfig.backendAuthority.toString());
    console.log("  - Event Admin:", existingConfig.eventAdmin.toString());
    console.log("  - Update Authority:", existingConfig.updateAuthority.toString());
    console.log("  - Is Paused:", existingConfig.isPaused);
    return;
  } catch (err) {
    // Not initialized yet, continue
    console.log("✅ Platform not yet initialized, proceeding...");
  }

  // Generate backend authority keypair (YOU MUST SAVE THIS!)
  const backendAuthority = Keypair.generate();
  
  console.log("");
  console.log("🔑 IMPORTANT: Backend Authority Keypair Generated!");
  console.log("Public Key:", backendAuthority.publicKey.toString());
  console.log("Secret Key:", JSON.stringify(Array.from(backendAuthority.secretKey)));
  console.log("");
  console.log("⚠️  SAVE THE SECRET KEY ABOVE! Backend needs it to sign purchase authorizations.");
  console.log("");

  // Save backend authority to file
  const backendKeyPath = "./backend-authority.json";
  fs.writeFileSync(
    backendKeyPath,
    JSON.stringify(Array.from(backendAuthority.secretKey))
  );
  console.log(`✅ Backend authority saved to: ${backendKeyPath}`);
  console.log("");

  // Initialize parameters
  const initialFeeReceiver = wallet.publicKey; // Platform fee goes to deployer
  const initialFeeUsdc = 100_000; // 0.1 USDC platform fee
  const eventAdmin = wallet.publicKey; // Deployer can create events

  console.log("Initialization parameters:");
  console.log("  - Fee Receiver:", initialFeeReceiver.toString());
  console.log("  - Platform Fee:", initialFeeUsdc / 1_000_000, "USDC");
  console.log("  - Backend Authority:", backendAuthority.publicKey.toString());
  console.log("  - Event Admin:", eventAdmin.toString());
  console.log("");

  // Confirm with user
  console.log("⏳ Sending transaction...");

  try {
    const tx = await program.methods
      .initializePlatform(
        initialFeeReceiver,
        new BN(initialFeeUsdc),
        backendAuthority.publicKey,
        eventAdmin
      )
      .accountsPartial({
        deployer: wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("✅ Platform initialized successfully!");
    console.log("Transaction signature:", tx);
    console.log("");

    // Fetch and display config
    const config = await program.account.platformConfig.fetch(platformConfig);
    console.log("Platform Config:");
    console.log("  - Fee Receiver:", config.feeReceiver.toString());
    console.log("  - Platform Fee:", config.feeAmountUsdc.toNumber() / 1_000_000, "USDC");
    console.log("  - Backend Authority:", config.backendAuthority.toString());
    console.log("  - Event Admin:", config.eventAdmin.toString());
    console.log("  - Update Authority:", config.updateAuthority.toString());
    console.log("  - Is Paused:", config.isPaused);
    console.log("");
    
    console.log("🎉 Platform is ready to use!");
    console.log("");
    console.log("Next steps:");
    console.log("  1. Save backend-authority.json securely");
    console.log("  2. Share backend-authority.json with backend team");
    console.log("  3. Use scripts/create-event.ts to create your first event");

  } catch (err) {
    console.error("❌ Initialization failed:", err);
    throw err;
  }
}

main()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });

