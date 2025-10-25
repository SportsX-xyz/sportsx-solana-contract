import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { TicketingProgram } from "../target/types/ticketing_program";
import * as fs from "fs";

// 配置参数 - 请根据你的需求修改这些地址
const CONFIG = {
  // 从 DEVNET_ADDRESSES.txt 获取的地址
  FEE_RECEIVER: "F3muW7SHCjRWz9gPE639hkekbW2YxNUPA4r6mefZvzqx",
  BACKEND_AUTHORITY: "DsSBRCNzwNTsYeh6zyQqqPJbQVBprPUaVmaPuegaeQBS", 
  EVENT_ADMIN: "DsSBRCNzwNTsYeh6zyQqqPJbQVBprPUaVmaPuegaeQBS",
  INITIAL_FEE_USDC: 100000, // 0.1 USDC (6 decimals)
};

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

    
  const ticketingProgram = anchor.workspace.TicketingProgram as Program<TicketingProgram>;

  console.log("🚀 Starting Ticketing Program Initialization...");
  console.log("Admin wallet:", provider.wallet.publicKey.toString());
  console.log("Ticketing Program ID:", ticketingProgram.programId.toString());

  // 验证管理员钱包
  if (provider.wallet.publicKey.toString() !== CONFIG.BACKEND_AUTHORITY) {
    console.warn("⚠️  Warning: Current wallet is not the configured backend authority");
    console.warn("   Current:", provider.wallet.publicKey.toString());
    console.warn("   Expected:", CONFIG.BACKEND_AUTHORITY);
  }

  // Step 1: 检查平台是否已初始化
  const [platformConfigPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("platform_config")],
    ticketingProgram.programId
  );

  const [nonceTrackerPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("nonce_tracker")],
    ticketingProgram.programId
  );

  const [ticketAuthorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("ticket_authority")],
    ticketingProgram.programId
  );

  console.log("\n📋 Step 1: Check Platform Initialization");
  console.log("Platform Config PDA:", platformConfigPda.toString());
  console.log("Nonce Tracker PDA:", nonceTrackerPda.toString());
  console.log("Ticket Authority PDA:", ticketAuthorityPda.toString());

  try {
    const existingConfig = await ticketingProgram.account.platformConfig.fetch(platformConfigPda);
    console.log("✅ Platform already initialized");
    console.log("   Update Authority:", existingConfig.updateAuthority.toString());
    console.log("   Backend Authority:", existingConfig.backendAuthority.toString());
    console.log("   Event Admin:", existingConfig.eventAdmin.toString());
    console.log("   Fee Receiver:", existingConfig.feeReceiver.toString());
    console.log("   Fee Amount:", existingConfig.feeAmountUsdc.toString());
    console.log("   Is Paused:", existingConfig.isPaused);
    
    // 检查是否需要更新配置
    if (existingConfig.backendAuthority.toString() !== CONFIG.BACKEND_AUTHORITY ||
        existingConfig.eventAdmin.toString() !== CONFIG.EVENT_ADMIN ||
        existingConfig.feeReceiver.toString() !== CONFIG.FEE_RECEIVER) {
      console.log("\n📋 Step 2: Update Platform Configuration");
      await updatePlatformConfig(ticketingProgram, platformConfigPda);
    } else {
      console.log("✅ Platform configuration is up to date");
    }
  } catch (e) {
    console.log("Initializing platform...");
    await initializePlatform(ticketingProgram, platformConfigPda, nonceTrackerPda, ticketAuthorityPda);
  }

  // Step 3: 验证最终状态
  await verifyFinalState(ticketingProgram, platformConfigPda, ticketAuthorityPda);
}

async function initializePlatform(
  program: Program<TicketingProgram>,
  platformConfigPda: PublicKey,
  nonceTrackerPda: PublicKey,
  ticketAuthorityPda: PublicKey
) {
  console.log("\n📋 Initializing Platform Configuration");
  
  const tx = await program.methods
    .initializePlatform(
      new PublicKey(CONFIG.FEE_RECEIVER),
      new anchor.BN(CONFIG.INITIAL_FEE_USDC),
      new PublicKey(CONFIG.BACKEND_AUTHORITY),
      new PublicKey(CONFIG.EVENT_ADMIN)
    )
    // .accountsPartial({
    //   platformConfig: platformConfigPda,
    //   nonceTracker: nonceTrackerPda,
    //   ticketAuthority: ticketAuthorityPda,
    //   deployer: program.provider.wallet.publicKey,
    //   systemProgram: SystemProgram.programId,
    // })
    .rpc();

  console.log("✅ Platform initialized!");
  console.log("   Transaction:", tx);
}

async function updatePlatformConfig(
  program: Program<TicketingProgram>,
  platformConfigPda: PublicKey
) {
  console.log("Updating platform configuration...");
  
  const tx = await program.methods
    .updatePlatformConfig(
      new PublicKey(CONFIG.FEE_RECEIVER),
      new anchor.BN(CONFIG.INITIAL_FEE_USDC),
      new PublicKey(CONFIG.BACKEND_AUTHORITY),
      new PublicKey(CONFIG.EVENT_ADMIN)
    )
    // .accounts({
    //   platformConfig: platformConfigPda,
    //   authority: program.provider.wallet.publicKey,
    // })
    .rpc();

  console.log("✅ Platform configuration updated!");
  console.log("   Transaction:", tx);
}

async function verifyFinalState(
  program: Program<TicketingProgram>,
  platformConfigPda: PublicKey,
  ticketAuthorityPda: PublicKey
) {
  console.log("\n🎉 Verification Complete!");
  console.log("=====================================");
  
  try {
    const config = await program.account.platformConfig.fetch(platformConfigPda);
    console.log("Ticketing Program ID:", program.programId.toString());
    console.log("Platform Config PDA:", platformConfigPda.toString());
    console.log("Ticket Authority PDA:", ticketAuthorityPda.toString());
    console.log("Update Authority:", config.updateAuthority.toString());
    console.log("Backend Authority:", config.backendAuthority.toString());
    console.log("Event Admin:", config.eventAdmin.toString());
    console.log("Fee Receiver:", config.feeReceiver.toString());
    console.log("Fee Amount (USDC):", config.feeAmountUsdc.toString());
    console.log("Is Paused:", config.isPaused);
    console.log("=====================================");
    
    // 验证配置是否正确
    const isConfigCorrect = 
      config.backendAuthority.toString() === CONFIG.BACKEND_AUTHORITY &&
      config.eventAdmin.toString() === CONFIG.EVENT_ADMIN &&
      config.feeReceiver.toString() === CONFIG.FEE_RECEIVER;
    
    if (isConfigCorrect) {
      console.log("✅ All configurations are correct!");
    } else {
      console.log("❌ Configuration mismatch detected!");
    }
  } catch (error) {
    console.error("❌ Error verifying final state:", error);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error("❌ Error:", error);
    process.exit(1);
  });