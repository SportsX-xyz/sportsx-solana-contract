import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TicketingProgram } from "../target/types/ticketing_program";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import * as fs from "fs";

/**
 * Initialize ticket authority for PoF integration (for existing deployments)
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

  console.log("🔧 Initializing Ticket Authority for PoF Integration");
  console.log("Program ID:", programId.toString());
  console.log("Authority:", wallet.publicKey.toString());
  console.log("");

  // Derive PDAs
  const [platformConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from("platform_config")],
    program.programId
  );

  const [ticketAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from("ticket_authority")],
    program.programId
  );

  console.log("Platform Config PDA:", platformConfig.toString());
  console.log("Ticket Authority PDA:", ticketAuthority.toString());
  console.log("");

  // Check if already initialized
  try {
    const existingAuthority = await program.account.ticketAuthority.fetch(ticketAuthority);
    console.log("⚠️  Ticket authority already initialized!");
    console.log("Bump:", existingAuthority.bump);
    return;
  } catch (err) {
    // Not initialized yet, continue
    console.log("✅ Ticket authority not yet initialized, proceeding...");
  }

  console.log("⏳ Sending transaction...");

  try {
    const tx = await program.methods
      .initializeTicketAuthority()
      .accountsPartial({
        authority: wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("✅ Ticket authority initialized successfully!");
    console.log("Transaction signature:", tx);
    console.log("");

    // Fetch and display
    const authority = await program.account.ticketAuthority.fetch(ticketAuthority);
    console.log("Ticket Authority:");
    console.log("  - PDA:", ticketAuthority.toString());
    console.log("  - Bump:", authority.bump);
    console.log("");
    
    console.log("🎉 Ticket authority is ready for PoF integration!");
    console.log("");
    console.log("Next steps:");
    console.log("  1. Submit authorization to PoF team:");
    console.log("     - Contract Name: Ticketing Contract");
    console.log("     - Program ID:", programId.toString());
    console.log('     - Authority PDA Seeds: ["ticket_authority"]');

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

