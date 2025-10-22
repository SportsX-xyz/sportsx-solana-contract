import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { TicketingProgram } from "../target/types/ticketing_program";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import * as fs from "fs";

/**
 * Update platform wallets to use separate production wallets
 */
async function main() {
  // Connect to devnet
  const connection = new anchor.web3.Connection(
    "https://api.devnet.solana.com",
    "confirmed"
  );

  // Load deployer wallet (has update authority)
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

  console.log("🔄 Updating Platform Wallets");
  console.log("Deployer (Update Authority):", wallet.publicKey.toString());
  console.log("");

  // Derive PDAs
  const [platformConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from("platform_config")],
    program.programId
  );

  // Create new wallets for production
  const feeReceiverKeypair = Keypair.generate();
  const eventAdminKeypair = Keypair.generate();

  console.log("🔑 Generated Production Wallets:");
  console.log("");
  console.log("1. Fee Receiver (receives platform fees):");
  console.log("   Public Key:", feeReceiverKeypair.publicKey.toString());
  console.log("   Secret Key:", JSON.stringify(Array.from(feeReceiverKeypair.secretKey)));
  console.log("");
  console.log("2. Event Admin (can create events):");
  console.log("   Public Key:", eventAdminKeypair.publicKey.toString());
  console.log("   Secret Key:", JSON.stringify(Array.from(eventAdminKeypair.secretKey)));
  console.log("");

  // Save wallets to files
  const feeReceiverPath = "./fee-receiver-wallet.json";
  const eventAdminPath = "./event-admin-wallet.json";

  fs.writeFileSync(
    feeReceiverPath,
    JSON.stringify(Array.from(feeReceiverKeypair.secretKey))
  );
  fs.writeFileSync(
    eventAdminPath,
    JSON.stringify(Array.from(eventAdminKeypair.secretKey))
  );

  console.log("✅ Wallets saved:");
  console.log("   -", feeReceiverPath);
  console.log("   -", eventAdminPath);
  console.log("");

  // Fetch current config
  const currentConfig = await program.account.platformConfig.fetch(platformConfig);
  console.log("Current Platform Config:");
  console.log("  - Fee Receiver:", currentConfig.feeReceiver.toString());
  console.log("  - Event Admin:", currentConfig.eventAdmin.toString());
  console.log("  - Backend Authority:", currentConfig.backendAuthority.toString());
  console.log("");

  console.log("⏳ Updating platform config...");

  try {
    const tx = await program.methods
      .updatePlatformConfig(
        feeReceiverKeypair.publicKey,  // new_fee_receiver
        null,                           // new_fee_usdc (keep same)
        null,                           // new_backend_authority (keep same)
        eventAdminKeypair.publicKey     // new_event_admin
      )
      .accountsPartial({
        authority: wallet.publicKey,
      })
      .rpc();

    console.log("✅ Platform config updated!");
    console.log("Transaction signature:", tx);
    console.log("");

    // Fetch and display updated config
    const updatedConfig = await program.account.platformConfig.fetch(platformConfig);
    console.log("Updated Platform Config:");
    console.log("  - Fee Receiver:", updatedConfig.feeReceiver.toString());
    console.log("  - Platform Fee:", updatedConfig.feeAmountUsdc.toNumber() / 1_000_000, "USDC");
    console.log("  - Event Admin:", updatedConfig.eventAdmin.toString());
    console.log("  - Backend Authority:", updatedConfig.backendAuthority.toString());
    console.log("  - Update Authority:", updatedConfig.updateAuthority.toString());
    console.log("");

    console.log("🎉 Production wallets configured!");
    console.log("");
    console.log("⚠️  IMPORTANT - Give to Backend Team:");
    console.log("  1. backend-authority.json (for signing purchases)");
    console.log("  2. event-admin-wallet.json (for creating events)");
    console.log("");
    console.log("⚠️  IMPORTANT - Give to Finance Team:");
    console.log("  1. fee-receiver-wallet.json (receives platform fees)");
    console.log("");
    console.log("⚠️  Keep for yourself:");
    console.log("  1. Your deployer wallet (for contract upgrades only)");
    console.log("");
    console.log("💡 These wallets need SOL for transaction fees.");
    console.log("   Airdrop SOL on devnet:");
    console.log(`   solana airdrop 2 ${feeReceiverKeypair.publicKey.toString()} --url devnet`);
    console.log(`   solana airdrop 2 ${eventAdminKeypair.publicKey.toString()} --url devnet`);

  } catch (err) {
    console.error("❌ Update failed:", err);
    throw err;
  }
}

main()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });

