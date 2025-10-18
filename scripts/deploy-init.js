const anchor = require("@coral-xyz/anchor");
const { PublicKey, SystemProgram } = require("@solana/web3.js");

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const pofProgram = anchor.workspace.SportsxPof;
  const checkinProgram = anchor.workspace.SportsxCheckin;

  console.log("🚀 Starting initialization on Devnet...");
  console.log("Admin wallet:", provider.wallet.publicKey.toString());
  console.log("PoF Program ID:", pofProgram.programId.toString());
  console.log("Check-in Program ID:", checkinProgram.programId.toString());

  // Step 1: Initialize PoF global state
  const [globalStatePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_state")],
    pofProgram.programId
  );

  console.log("\n📋 Step 1: Initialize PoF Global State");
  console.log("Global State PDA:", globalStatePda.toString());

  try {
    const existingState = await pofProgram.account.globalState.fetch(globalStatePda);
    console.log("✅ Global state already initialized");
    console.log("   Admin:", existingState.admin.toString());
    console.log("   Authorized contracts:", existingState.authorizedContracts.length);
  } catch (e) {
    console.log("Initializing global state...");
    const tx = await pofProgram.methods
      .initialize()
      .accounts({
        globalState: globalStatePda,
        admin: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    
    console.log("✅ Global state initialized!");
    console.log("   Transaction:", tx);
  }

  // Step 2: Get unified checkin authority PDA
  const [checkinAuthorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("checkin_authority")],
    checkinProgram.programId
  );

  console.log("\n📋 Step 2: Authorize Check-in Contract");
  console.log("Check-in Authority PDA:", checkinAuthorityPda.toString());

  // Check if already authorized
  const globalState = await pofProgram.account.globalState.fetch(globalStatePda);
  const isAuthorized = globalState.authorizedContracts.some(
    (contract) => contract.toString() === checkinAuthorityPda.toString()
  );

  if (isAuthorized) {
    console.log("✅ Check-in contract already authorized");
  } else {
    console.log("Authorizing check-in contract...");
    const tx = await pofProgram.methods
      .authorizeContract(checkinAuthorityPda)
      .accounts({
        globalState: globalStatePda,
        admin: provider.wallet.publicKey,
      })
      .rpc();
    
    console.log("✅ Check-in contract authorized!");
    console.log("   Transaction:", tx);
  }

  // Final status
  const finalState = await pofProgram.account.globalState.fetch(globalStatePda);
  console.log("\n🎉 Deployment Complete!");
  console.log("=====================================");
  console.log("Network: Devnet");
  console.log("PoF Program ID:", pofProgram.programId.toString());
  console.log("Check-in Program ID:", checkinProgram.programId.toString());
  console.log("Global State PDA:", globalStatePda.toString());
  console.log("Check-in Authority PDA:", checkinAuthorityPda.toString());
  console.log("Admin:", finalState.admin.toString());
  console.log("Authorized Contracts:", finalState.authorizedContracts.length);
  console.log("=====================================");
  
  console.log("\n📝 Save these addresses:");
  console.log(`POF_PROGRAM_ID=${pofProgram.programId.toString()}`);
  console.log(`CHECKIN_PROGRAM_ID=${checkinProgram.programId.toString()}`);
  console.log(`GLOBAL_STATE_PDA=${globalStatePda.toString()}`);
  console.log(`CHECKIN_AUTHORITY_PDA=${checkinAuthorityPda.toString()}`);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error("❌ Error:", error);
    process.exit(1);
  });

