// 修正后的更新脚本
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TicketingProgram } from "../target/types/ticketing_program";
import { PublicKey, Keypair } from "@solana/web3.js";
import * as fs from "fs";

async function main() {
  const connection = new anchor.web3.Connection(
    "https://api.devnet.solana.com",
    "confirmed"
  );
  
  const walletPath = process.env.HOME + "/.config/solana/id.json";
  const walletKeypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(walletPath, "utf-8")))
  );
  const wallet = new anchor.Wallet(walletKeypair);
  anchor.setProvider(new anchor.AnchorProvider(connection, wallet, {}));
  
  const program = anchor.workspace.TicketingProgram as Program<TicketingProgram>;
  
  const [platformConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from("platform_config")],
    program.programId
  );
  
  // 正确的平台钱包地址（不是 token account）
  const correctFeeReceiver = new PublicKey("F3muW7SHCjRWz9gPE639hkekbW2YxNUPA4r6mefZvzqx");
  
  console.log("=== 修正 Platform Fee Receiver ===");
  console.log("当前钱包:", wallet.publicKey.toString());
  console.log("平台配置PDA:", platformConfig.toString());
  console.log("正确的 Fee Receiver:", correctFeeReceiver.toString());
  console.log("");
  
  try {
    // 获取当前配置
    console.log("📋 获取当前平台配置...");
    const currentConfig = await program.account.platformConfig.fetch(platformConfig);
    console.log("当前 Fee Receiver:", currentConfig.feeReceiver.toString());
    console.log("当前 Update Authority:", currentConfig.updateAuthority.toString());
    console.log("");
    
    // 检查权限
    if (currentConfig.updateAuthority.toString() !== wallet.publicKey.toString()) {
      console.error("❌ 错误: 当前钱包没有更新权限");
      return;
    }
    
    console.log("⏳ 修正 Fee Receiver...");
    
    // 更新为正确的钱包地址
    const tx = await program.methods
      .updatePlatformConfig(
        correctFeeReceiver,              // 正确的钱包地址
        null,                            // new_fee_usdc (保持不变)
        null,                            // new_backend_authority (保持不变)
        null                             // new_event_admin (保持不变)
      )
      .accountsPartial({
        authority: wallet.publicKey,
      })
      .rpc();
    
    console.log("✅ Fee Receiver 修正成功!");
    console.log("交易签名:", tx);
    console.log("");
    
    // 验证更新
    const updatedConfig = await program.account.platformConfig.fetch(platformConfig);
    console.log("修正后的配置:");
    console.log("  - Fee Receiver:", updatedConfig.feeReceiver.toString());
    console.log("  - Platform Fee:", updatedConfig.feeAmountUsdc.toNumber() / 1_000_000, "USDC");
    console.log("");
    
    console.log("🎉 Fee Receiver 修正完成!");
    console.log("现在程序会基于钱包地址 F3muW7SHCjRWz9gPE639hkekbW2YxNUPA4r6mefZvzqx 计算 USDC ATA");
    
  } catch (error) {
    console.error("❌ 修正失败:", error);
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