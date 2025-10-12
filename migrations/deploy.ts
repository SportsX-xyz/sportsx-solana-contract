const anchor = require('@project-serum/anchor');
const { Program, Wallet } = anchor;
const { PublicKey, Keypair, SystemProgram, Transaction } = require('@solana/web3.js');
const { TOKEN_2022_PROGRAM_ID, createMint, createAccount } = require('@solana/spl-token');
const fs = require('fs');
const path = require('path');

async function deployProgram() {
  // Set up provider
  const connection = new anchor.web3.Connection('http://127.0.0.1:8899', 'confirmed');
  const wallet = new Wallet(Keypair.generate()); // Replace with your wallet keypair
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: 'confirmed' });
  anchor.setProvider(provider);

  // Load program IDL and keypair
  const idl = JSON.parse(fs.readFileSync(path.resolve(__dirname, '../target/idl/ticketing_program.json'), 'utf8'));
  const programKeypair = Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(fs.readFileSync(path.resolve(__dirname, '../target/deploy/ticketing_program-keypair.json'))))
  );
  const programId = programKeypair.publicKey;

  // Initialize program
  const program = new Program(idl, programId, provider);

  // Airdrop SOL to wallet for fees
  await provider.connection.requestAirdrop(wallet.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
  await new Promise(resolve => setTimeout(resolve, 1000)); // Wait for airdrop confirmation

  // Create USDT mint (mock for testing)
  const usdtMint = await createMint(
    provider.connection,
    wallet.payer,
    wallet.publicKey,
    null,
    6, // USDT decimals
    undefined,
    { commitment: 'confirmed' },
    TOKEN_2022_PROGRAM_ID
  );

  // Derive platform config PDA
  const [platformConfigPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from('PLATFORM_CONFIG')],
    programId
  );

  // Initialize platform config
  const tx = new Transaction().add(
    program.instruction.initializePlatformConfig(
      wallet.publicKey, // platform_authority
      usdtMint, // usdt_mint
      {
        accounts: {
          payer: wallet.publicKey,
          platformConfig: platformConfigPDA,
          admin: wallet.publicKey,
          systemProgram: SystemProgram.programId,
        },
      }
    )
  );

  await provider.sendAndConfirm(tx);
  console.log('Program deployed and platform config initialized!');
  console.log('Program ID:', programId.toBase58());
  console.log('Platform Config PDA:', platformConfigPDA.toBase58());
  console.log('USDT Mint:', usdtMint.toBase58());
}

deployProgram().catch(err => {
  console.error('Deployment failed:', err);
  process.exit(1);
});
