import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { SportsxPof } from "../target/types/sportsx_pof";

/**
 * Generic authorization script for POF contract
 * 
 * Usage:
 * 1. Set CONTRACT_NAME, PROGRAM_ID, and AUTHORITY_SEEDS below
 * 2. Run: ts-node scripts/authorize-contract.ts
 */

// ============================================================
// CONFIGURATION - Update these for your contract
// ============================================================

const CONTRACT_NAME = "Ticket";  // Display name
const PROGRAM_ID = "EFuMNTn1zfn6Zhvdq1Vjaxs83sz2gTWvDgjuJcKDYjhw";  // Contract's Program ID
const AUTHORITY_SEEDS = ["ticket_authority"];  // PDA seeds used in CPI calls

// ============================================================

async function authorizeContract() {
  // Configure the client
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Load POF program
  const pofProgram = anchor.workspace.SportsxPof as Program<SportsxPof>;
  const contractProgramId = new PublicKey(PROGRAM_ID);

  console.log("POF Program ID:", pofProgram.programId.toString());
  console.log(`${CONTRACT_NAME} Program ID:`, contractProgramId.toString());

  // Derive PDAs
  const [globalState] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_state")],
    pofProgram.programId
  );

  const seedBuffers = AUTHORITY_SEEDS.map(seed => 
    typeof seed === "string" ? Buffer.from(seed) : seed
  );
  
  const [contractAuthority] = PublicKey.findProgramAddressSync(
    seedBuffers,
    contractProgramId
  );

  console.log("\n=== Addresses ===");
  console.log("Global State:", globalState.toString());
  console.log(`${CONTRACT_NAME} Authority PDA:`, contractAuthority.toString());
  console.log("Admin (current wallet):", provider.wallet.publicKey.toString());

  try {
    // Check if already authorized
    const globalStateAccount = await pofProgram.account.globalState.fetch(globalState);
    console.log("\n=== Current Authorized Contracts ===");
    console.log("Admin:", globalStateAccount.admin.toString());
    console.log("Authorized contracts:", globalStateAccount.authorizedContracts.map(c => c.toString()));

    const isAlreadyAuthorized = globalStateAccount.authorizedContracts.some(
      c => c.equals(contractAuthority)
    );

    if (isAlreadyAuthorized) {
      console.log(`\n✅ ${CONTRACT_NAME} contract is already authorized!`);
      return;
    }

    // Authorize the contract
    console.log(`\n=== Authorizing ${CONTRACT_NAME} Contract ===`);
    const tx = await pofProgram.methods
      .authorizeContract(contractAuthority)
      .accounts({
        globalState: globalState,
        admin: provider.wallet.publicKey,
      })
      .rpc();

    console.log("✅ Authorization successful!");
    console.log("Transaction signature:", tx);

    // Verify
    const updatedState = await pofProgram.account.globalState.fetch(globalState);
    console.log("\n=== Updated Authorized Contracts ===");
    console.log(updatedState.authorizedContracts.map(c => c.toString()));

  } catch (error) {
    console.error("❌ Error:", error);
    throw error;
  }
}

authorizeContract().catch((error) => {
  console.error("Fatal error:", error);
  throw error;
});

