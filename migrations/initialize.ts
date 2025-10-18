import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

// const USDT_DEVNET_MINT = new PublicKey("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");

export async function main() {
    // Setup provider
    anchor.setProvider(anchor.AnchorProvider.env());
    const program = anchor.workspace.TicketingProgram;
    const provider = anchor.getProvider();

    console.log("Admin:", provider.wallet.publicKey.toString());

    // Check balance
    const balance = await provider.connection.getBalance(provider.wallet.publicKey);
    console.log("SOL Balance:", balance / anchor.web3.LAMPORTS_PER_SOL, "SOL");

    // Derive PDA
    const [platformConfigPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("PLATFORM_CONFIG")],
        program.programId
    );

    console.log("Platform Config PDA:", platformConfigPDA.toString());

    // Check if exists
    const accountInfo = await provider.connection.getAccountInfo(platformConfigPDA);
    if (accountInfo && accountInfo.data.length > 0) {
        console.log("⚠️ Platform Config already initialized, skipping...");

        // Fetch existing data
        try {
            const config = await program.account.platformConfig.fetch(platformConfigPDA);
            console.log("Existing config:");
            console.log("  Authority:", config.platformAuthority.toString());
        } catch (e) {
            console.log("Could not fetch config:", e);
        }
        return;
    }

    console.log("\n🚀 Initializing Platform Config...");

    // Initialize
    const tx = await program.methods
        .initializePlatformConfig(
            provider.wallet.publicKey, // platform_authority
        )
        .accounts({
            payer: provider.wallet.publicKey,
            platformConfig: platformConfigPDA,
            admin: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

    console.log("✅ Success! Tx:", tx);
    console.log("Explorer:", `https://explorer.solana.com/tx/${tx}?cluster=devnet`);

    // Verify
    const config = await program.account.platformConfig.fetch(platformConfigPDA);
    console.log("Verified:");
    console.log("  Authority:", config.platformAuthority.toString());
}

anchor.setProvider(anchor.AnchorProvider.env());
main()
    .then(() => {
        console.log("Migration completed!");
        process.exit(0);
    })
    .catch((err) => {
        console.error("Migration failed:", err);
        process.exit(1);
    });