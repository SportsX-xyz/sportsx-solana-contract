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

const CONTRACT_NAME = "Checkin";  // Display name
const PROGRAM_ID = "2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX";  // Contract's Program ID
const AUTHORITY_SEEDS = ["checkin_authority"];  // PDA seeds used in CPI calls

// ============================================================
// Optional: Use command line arguments to override
// Example: ts-node scripts/authorize-contract.ts Ticket GxYZ...abc ticket_authority
// ============================================================

const contractName = process.argv[2] || CONTRACT_NAME;
const programId = process.argv[3] || PROGRAM_ID;
const authoritySeeds = process.argv[4] ? [process.argv[4]] : AUTHORITY_SEEDS;

// ============================================================

async function authorizeContract() {
  // Configure the client
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Load POF program
  const pofProgram = anchor.workspace.SportsxPof as Program<SportsxPof>;
  const contractProgramId = new PublicKey(programId);

  console.log("POF Program ID:", pofProgram.programId.toString());
  console.log(`${contractName} Program ID:`, contractProgramId.toString());

  // Derive PDAs
  const [globalState] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_state")],
    pofProgram.programId
  );

  const seedBuffers = authoritySeeds.map(seed => 
    typeof seed === "string" ? Buffer.from(seed) : seed
  );
  
  const [contractAuthority] = PublicKey.findProgramAddressSync(
    seedBuffers,
    contractProgramId
  );

  console.log("\n=== Addresses ===");
  console.log("Global State:", globalState.toString());
  console.log(`${contractName} Authority PDA:`, contractAuthority.toString());
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
      console.log(`\n✅ ${contractName} contract is already authorized!`);
      return;
    }

    // Authorize the contract
    console.log(`\n=== Authorizing ${contractName} Contract ===`);
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

authorizeContract()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });

