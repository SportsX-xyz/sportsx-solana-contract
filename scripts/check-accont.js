const { PublicKey, Connection } = require('@solana/web3.js');
const PROGRAM_ID = new PublicKey('EFuMNTn1zfn6Zhvdq1Vjaxs83sz2gTWvDgjuJcKDYjhw');

// 尝试不同的网络
const networks = [
  'https://api.mainnet-beta.solana.com',
  'https://api.devnet.solana.com',
  'https://api.testnet.solana.com'
];

async function checkProgramAccounts() {
  for (const network of networks) {
    try {
      console.log(\`\\n🌐 Checking network: \${network}\`);
      const connection = new Connection(network, 'confirmed');
      
      const accounts = await connection.getProgramAccounts(PROGRAM_ID);
      console.log(\`Found \${accounts.length} accounts owned by program\`);
      
      if (accounts.length > 0) {
        console.log('📋 Program accounts:');
        accounts.forEach((account, index) => {
          console.log(\`  \${index + 1}. \${account.pubkey.toString()}\`);
        });
        
        // 检查平台配置
        const [platformConfigPda] = PublicKey.findProgramAddressSync(
          [Buffer.from('platform_config')],
          PROGRAM_ID
        );
        
        const hasPlatformConfig = accounts.some(acc => acc.pubkey.equals(platformConfigPda));
        console.log(\`\\n🔧 Platform Config (\${platformConfigPda.toString()}): \${hasPlatformConfig ? '✅' : '❌'}\`);
        
        return; // 找到账户就退出
      }
    } catch (error) {
      console.log(\`❌ Error on \${network}: \${error.message}\`);
    }
  }
  
  console.log('\\n❌ No accounts found on any network');
}

checkProgramAccounts();

const { PublicKey, Connection } = require('@solana/web3.js');
const PROGRAM_ID = new PublicKey('EFuMNTn1zfn6Zhvdq1Vjaxs83sz2gTWvDgjuJcKDYjhw');

const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

const accounts = [
  'gfWzPqLG24zSgVmnLdNzQRCoYjfRGWTyDnCQ4RZDe33', // 平台配置
  'BsLpp2Rkcwvth9RNUXTTJte2rQZjns7Qb56XRj3GLpMB',
  '5qSt1JKvdo5zwx8FZCd74ENz3hwHsU1xdeiDPkKBh8Hy',
  'GDABnmK9vusTVE46eDRZ5zHR8BXNhS5dvNNw2PeNMn24',
  '7M89JDEvhPPAjrgQ5xfWdbmT3ec1KZAQUFk3HqsrVYMw',
  '6mL6gPPdq2k7LYyAi5majZ6diYeJC5RCiyiF4EzPzGNW',
  'Ckd32TtGFweYhkVVnybeKsvdgVDK78Wssgesy6sRgx61',
  'BqtCoZyjEr5ZfrmnVp4PRiNzsMQnr4ur6pQbTcBPNEAq'
];

async function checkAccounts() {
  console.log('🔍 Checking account types...');
  
  for (const accountAddress of accounts) {
    try {
      const account = await connection.getAccountInfo(new PublicKey(accountAddress));
      if (account) {
        console.log(\`\\n📋 \${accountAddress}:\`);
        console.log(\`   Owner: \${account.owner.toString()}\`);
        console.log(\`   Data length: \${account.data.length} bytes\`);
        console.log(\`   Executable: \${account.executable}\`);
        
        // 检查数据前几个字节来识别账户类型
        const data = account.data;
        if (data.length > 8) {
          const discriminator = Array.from(data.slice(0, 8)).map(b => b.toString().padStart(3, '0')).join(' ');
          console.log(\`   Discriminator: [\${discriminator}]\`);
          
          // 尝试解析为字符串（活动ID）
          if (data.length > 40) {
            try {
              const eventIdLength = data.readUInt32LE(8);
              if (eventIdLength > 0 && eventIdLength < 50) {
                const eventId = data.slice(12, 12 + eventIdLength).toString();
                console.log(\`   Event ID: \${eventId}\`);
              }
            } catch (e) {
              // 忽略解析错误
            }
          }
        }
      }
    } catch (error) {
      console.log(\`❌ Error checking \${accountAddress}: \${error.message}\`);
    }
  }
}

checkAccounts();
