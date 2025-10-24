import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TicketingProgram } from "../target/types/ticketing_program";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import * as fs from "fs";

/**
 * 更新backend_authority为指定的地址
 */
async function main() {
  // 连接到devnet
  const connection = new anchor.web3.Connection(
    "https://api.devnet.solana.com",
    "confirmed"
  );
  
  // 加载部署者钱包 (有更新权限)
  const walletPath = process.env.HOME + "/.config/solana/id.json";
  const walletKeypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(walletPath, "utf-8")))
  );
  const wallet = new anchor.Wallet(walletKeypair);
  anchor.setProvider(new anchor.AnchorProvider(connection, wallet, {}));
  
  const program = anchor.workspace.TicketingProgram as Program<TicketingProgram>;
  
  // 平台配置PDA
  const [platformConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from("platform_config")],
    program.programId
  );
  
  // 新的backend_authority地址
  const newBackendAuthority = new PublicKey("DsSBRCNzwNTsYeh6zyQqqPJbQVBprPUaVmaPuegaeQBS");
  
  console.log("=== 更新 Backend Authority ===");
  console.log("当前钱包:", wallet.publicKey.toString());
  console.log("平台配置PDA:", platformConfig.toString());
  console.log("新的Backend Authority:", newBackendAuthority.toString());
  console.log("");
  
  try {
    // 先获取当前配置
    console.log("📋 获取当前平台配置...");
    const currentConfig = await program.account.platformConfig.fetch(platformConfig);
    console.log("当前 Backend Authority:", currentConfig.backendAuthority.toString());
    console.log("当前 Update Authority:", currentConfig.updateAuthority.toString());
    console.log("");
    
    // 检查是否有权限更新
    if (currentConfig.updateAuthority.toString() !== wallet.publicKey.toString()) {
      console.error("❌ 错误: 当前钱包没有更新权限");
      console.error("需要 Update Authority:", currentConfig.updateAuthority.toString());
      console.error("当前钱包:", wallet.publicKey.toString());
      return;
    }
    
    console.log("⏳ 更新 Backend Authority...");
    
    // 更新backend_authority
    const tx = await program.methods
      .updatePlatformConfig(
        null,                           // new_fee_receiver (保持不变)
        null,                           // new_fee_usdc (保持不变)
        newBackendAuthority,             // new_backend_authority (更新)
        null                            // new_event_admin (保持不变)
      )
      .accountsPartial({
        authority: wallet.publicKey,
      })
      .rpc();
    
    console.log("✅ Backend Authority 更新成功!");
    console.log("交易签名:", tx);
    console.log("");
    
    // 获取更新后的配置
    console.log("📋 验证更新结果...");
    const updatedConfig = await program.account.platformConfig.fetch(platformConfig);
    console.log("更新后的配置:");
    console.log("  - Fee Receiver:", updatedConfig.feeReceiver.toString());
    console.log("  - Platform Fee:", updatedConfig.feeAmountUsdc.toNumber() / 1_000_000, "USDC");
    console.log("  - Backend Authority:", updatedConfig.backendAuthority.toString());
    console.log("  - Event Admin:", updatedConfig.eventAdmin.toString());
    console.log("  - Update Authority:", updatedConfig.updateAuthority.toString());
    console.log("  - Is Paused:", updatedConfig.isPaused);
    console.log("");
    
    console.log("🎉 Backend Authority 更新完成!");
    console.log("");
    console.log("现在可以使用新的 Backend Authority 进行购票:");
    console.log("  Backend Authority:", updatedConfig.backendAuthority.toString());
    
  } catch (error) {
    console.error("❌ 更新失败:", error);
    throw error;
  }
}

main()
  .then(() => {
    console.log("✅ 脚本执行完成");
    process.exit(0);
  })
  .catch((error) => {
    console.error("❌ 脚本执行失败:", error);
    process.exit(1);
  });
