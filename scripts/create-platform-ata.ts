// scripts/create-platform-ata.ts
import * as anchor from "@coral-xyz/anchor";
import { getOrCreateAssociatedTokenAccount, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { PublicKey, Keypair } from "@solana/web3.js";
import * as fs from "fs";

async function createPlatformATA() {
  const connection = new anchor.web3.Connection("https://api.devnet.solana.com" , "confirmed");

  
  // 新的 Token 2022 mint
  const newUsdcMint = new PublicKey("BV84FcnKtu6ZXF7bpLYpA3gGxabngx9b26QSduuPvWR5");
  console.log("newUsdcMint", newUsdcMint.toString());
  
  // 平台钱包地址
  const platformWallet = new PublicKey("F3muW7SHCjRWz9gPE639hkekbW2YxNUPA4r6mefZvzqx");
  console.log("platformWallet", platformWallet.toString());
  // 加载部署者钱包
  const walletPath = process.env.HOME + "/.config/solana/id.json";
  const walletKeypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(walletPath, "utf-8")))
  );
  
  console.log("=== 创建平台 USDC ATA ===");
  console.log("新 USDC Mint:", newUsdcMint.toString());
  console.log("平台钱包:", platformWallet.toString());
  
  try {
    // 创建平台 USDC ATA
    const platformAta = await getOrCreateAssociatedTokenAccount(
      connection,
      walletKeypair,
      newUsdcMint,
      platformWallet,
      false,
      'confirmed',
      {
        commitment: 'confirmed',
        skipPreflight: false,
      },
      TOKEN_2022_PROGRAM_ID
    );
    
    console.log("✅ 平台 USDC ATA 创建成功:", platformAta.address.toString());
    
    // 如果需要，可以 mint 一些测试代币
    // await mintTo(connection, walletKeypair, newUsdcMint, platformAta.address, walletKeypair, 1000_000_000);
    
  } catch (error) {
    console.error("❌ 创建失败:", error);
    throw error;
  }
}