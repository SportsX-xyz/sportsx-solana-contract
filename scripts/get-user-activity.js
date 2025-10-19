const anchor = require("@coral-xyz/anchor");
const { PublicKey, Connection } = require("@solana/web3.js");

/**
 * 获取用户的合约活动历史
 * 通过查询交易签名和解析程序日志
 */
async function getUserActivity(userWallet, options = {}) {
  const {
    limit = 20,
    network = "devnet",
  } = options;

  const rpcUrl = network === "devnet" 
    ? "https://api.devnet.solana.com"
    : "https://api.mainnet-beta.solana.com";
  
  const connection = new Connection(rpcUrl, "confirmed");
  
  const pofProgramId = new PublicKey("E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV");
  const checkinProgramId = new PublicKey("2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX");

  console.log(`🔍 Fetching activity for wallet: ${userWallet.toString()}`);
  console.log(`Network: ${network}\n`);

  // 获取用户的交易签名
  const signatures = await connection.getSignaturesForAddress(
    userWallet,
    { limit }
  );

  console.log(`Found ${signatures.length} transactions\n`);

  const activities = [];

  // 解析每个交易
  for (const sigInfo of signatures) {
    try {
      const tx = await connection.getParsedTransaction(
        sigInfo.signature,
        {
          maxSupportedTransactionVersion: 0,
        }
      );

      if (!tx || !tx.meta) continue;

      // 检查是否涉及我们的合约
      const accountKeys = tx.transaction.message.accountKeys.map(
        k => k.pubkey.toString()
      );

      const involvesPof = accountKeys.includes(pofProgramId.toString());
      const involvesCheckin = accountKeys.includes(checkinProgramId.toString());

      if (!involvesPof && !involvesCheckin) continue;

      // 解析日志获取详细信息
      const logs = tx.meta.logMessages || [];
      const activity = parseActivity(logs, sigInfo, involvesPof, involvesCheckin);
      
      if (activity) {
        activities.push(activity);
      }
    } catch (err) {
      console.error(`Error parsing tx ${sigInfo.signature}:`, err.message);
    }
  }

  return activities;
}

/**
 * 解析交易日志，提取活动信息
 */
function parseActivity(logs, sigInfo, involvesPof, involvesCheckin) {
  const activity = {
    signature: sigInfo.signature,
    timestamp: sigInfo.blockTime,
    date: new Date(sigInfo.blockTime * 1000).toLocaleString(),
    type: "unknown",
    details: {},
  };

  // 解析日志
  for (const log of logs) {
    // 签到活动
    if (log.includes("Check-in successful")) {
      activity.type = "check-in";
      const match = log.match(/Total check-ins: (\d+)/);
      if (match) {
        activity.details.totalCheckins = parseInt(match[1]);
      }
    }
    
    // 积分奖励
    if (log.includes("Awarded") && log.includes("points")) {
      const match = log.match(/Awarded (\d+) points/);
      if (match) {
        activity.details.pointsAwarded = parseInt(match[1]);
      }
    }
    
    // 积分更新
    if (log.includes("points updated")) {
      activity.type = "points-update";
      const match = log.match(/updated by ([-\d]+):/);
      if (match) {
        activity.details.pointsDelta = parseInt(match[1]);
      }
    }
    
    // 账户初始化
    if (log.includes("initialized")) {
      if (log.includes("wallet") && log.includes("points")) {
        activity.type = "init-wallet";
      } else if (log.includes("Check-in record")) {
        activity.type = "init-checkin";
      }
    }
  }

  // 如果找到了活动类型，返回
  if (activity.type !== "unknown") {
    return activity;
  }

  // 否则返回基本信息
  if (involvesPof || involvesCheckin) {
    activity.type = involvesPof ? "pof-interaction" : "checkin-interaction";
    return activity;
  }

  return null;
}

/**
 * 格式化输出活动
 */
function displayActivities(activities) {
  if (activities.length === 0) {
    console.log("📭 No activities found");
    return;
  }

  console.log(`\n📊 Found ${activities.length} activities:\n`);
  console.log("=".repeat(80));

  activities.forEach((activity, index) => {
    console.log(`\n${index + 1}. ${activity.type.toUpperCase()}`);
    console.log(`   Date: ${activity.date}`);
    console.log(`   Signature: ${activity.signature}`);
    
    if (Object.keys(activity.details).length > 0) {
      console.log(`   Details:`, activity.details);
    }
    
    console.log(`   Link: https://explorer.solana.com/tx/${activity.signature}?cluster=devnet`);
  });

  console.log("\n" + "=".repeat(80));
}

// 使用示例
async function example() {
  // 替换为实际的用户钱包地址
  const userWallet = new PublicKey("YOUR_USER_WALLET_ADDRESS");
  
  const activities = await getUserActivity(userWallet, {
    limit: 50,
    network: "devnet",
  });
  
  displayActivities(activities);
}

// 如果直接运行此脚本
if (require.main === module) {
  if (process.argv.length < 3) {
    console.log("Usage: node get-user-activity.js <WALLET_ADDRESS>");
    console.log("Example: node get-user-activity.js 3nPsYeVZ92h8a5idvjmrgEVr5iGgU948VEoKdCj3bbQF");
    process.exit(1);
  }

  const walletAddress = new PublicKey(process.argv[2]);
  
  getUserActivity(walletAddress, { limit: 50, network: "devnet" })
    .then(displayActivities)
    .catch(console.error);
}

module.exports = { getUserActivity, parseActivity, displayActivities };

