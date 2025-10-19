# 获取用户活动历史 - 不修改合约代码

## 📖 概述

虽然合约中没有存储完整的历史记录，但可以通过以下方式获取用户的活动：

1. ✅ **查询交易历史** - 获取用户的所有交易签名
2. ✅ **解析程序日志** - 从日志中提取活动信息
3. ✅ **使用RPC方法** - Solana提供丰富的查询API
4. ✅ **第三方索引服务** - 使用Helius、QuickNode等

---

## 方法1: 使用脚本查询（最简单）

### 快速测试

```bash
# 运行脚本获取活动历史
node scripts/get-user-activity.js <钱包地址>

# 示例
node scripts/get-user-activity.js 3nPsYeVZ92h8a5idvjmrgEVr5iGgU948VEoKdCj3bbQF
```

**输出示例**：
```
📊 Found 5 activities:

1. CHECK-IN
   Date: 2025-10-18 14:30:00
   Signature: 4TYdTE...
   Details: { totalCheckins: 1, pointsAwarded: 10 }
   Link: https://explorer.solana.com/tx/...

2. POINTS-UPDATE
   Date: 2025-10-18 12:00:00
   Details: { pointsDelta: 100 }
```

---

## 方法2: 前端集成（推荐）

### React Hook 示例

```typescript
// src/hooks/useUserActivity.ts
import { useEffect, useState } from "react";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";

interface Activity {
  signature: string;
  timestamp: number;
  type: string;
  details: any;
}

export const useUserActivity = (limit = 20) => {
  const { connection } = useConnection();
  const { publicKey } = useWallet();
  const [activities, setActivities] = useState<Activity[]>([]);
  const [loading, setLoading] = useState(false);

  const POF_PROGRAM_ID = new PublicKey(
    process.env.REACT_APP_POF_PROGRAM_ID!
  );
  const CHECKIN_PROGRAM_ID = new PublicKey(
    process.env.REACT_APP_CHECKIN_PROGRAM_ID!
  );

  const fetchActivities = async () => {
    if (!publicKey) return;

    setLoading(true);
    try {
      // 获取交易签名
      const signatures = await connection.getSignaturesForAddress(
        publicKey,
        { limit }
      );

      const parsedActivities: Activity[] = [];

      // 解析每个交易
      for (const sigInfo of signatures) {
        const tx = await connection.getParsedTransaction(
          sigInfo.signature,
          { maxSupportedTransactionVersion: 0 }
        );

        if (!tx || !tx.meta) continue;

        // 检查是否涉及我们的合约
        const accountKeys = tx.transaction.message.accountKeys.map(
          k => k.pubkey.toString()
        );

        const involvesPof = accountKeys.includes(POF_PROGRAM_ID.toString());
        const involvesCheckin = accountKeys.includes(CHECKIN_PROGRAM_ID.toString());

        if (!involvesPof && !involvesCheckin) continue;

        // 解析日志
        const logs = tx.meta.logMessages || [];
        const activity = parseActivityFromLogs(logs, sigInfo);

        if (activity) {
          parsedActivities.push(activity);
        }
      }

      setActivities(parsedActivities);
    } catch (err) {
      console.error("Failed to fetch activities:", err);
    } finally {
      setLoading(false);
    }
  };

  const parseActivityFromLogs = (logs: string[], sigInfo: any): Activity | null => {
    const activity: Activity = {
      signature: sigInfo.signature,
      timestamp: sigInfo.blockTime || 0,
      type: "unknown",
      details: {},
    };

    for (const log of logs) {
      // 签到
      if (log.includes("Check-in successful")) {
        activity.type = "check-in";
        const match = log.match(/Total check-ins: (\d+)/);
        if (match) {
          activity.details.totalCheckins = parseInt(match[1]);
          activity.details.pointsEarned = 10;
        }
      }

      // 积分更新
      if (log.includes("points updated by")) {
        activity.type = "points-update";
        const match = log.match(/updated by ([-\d]+):/);
        if (match) {
          activity.details.change = parseInt(match[1]);
        }
      }

      // 初始化
      if (log.includes("initialized with 0 points")) {
        activity.type = "wallet-initialized";
      }
      if (log.includes("Check-in record initialized")) {
        activity.type = "checkin-initialized";
      }
    }

    return activity.type !== "unknown" ? activity : null;
  };

  useEffect(() => {
    fetchActivities();
  }, [publicKey]);

  return {
    activities,
    loading,
    refresh: fetchActivities,
  };
};
```

### 使用组件

```typescript
// src/components/ActivityHistory.tsx
import { useUserActivity } from "../hooks/useUserActivity";

export const ActivityHistory = () => {
  const { activities, loading, refresh } = useUserActivity(30);

  if (loading) return <div>加载中...</div>;

  return (
    <div className="activity-history">
      <div className="header">
        <h3>最近活动</h3>
        <button onClick={refresh}>刷新</button>
      </div>

      <div className="activity-list">
        {activities.map((activity, i) => (
          <div key={activity.signature} className="activity-item">
            <div className="activity-icon">
              {activity.type === "check-in" && "✅"}
              {activity.type === "points-update" && "💰"}
              {activity.type.includes("initialized") && "🎬"}
            </div>
            
            <div className="activity-info">
              <div className="activity-type">
                {formatActivityType(activity.type)}
              </div>
              <div className="activity-time">
                {new Date(activity.timestamp * 1000).toLocaleString()}
              </div>
              <div className="activity-details">
                {formatDetails(activity)}
              </div>
            </div>

            <a
              href={`https://explorer.solana.com/tx/${activity.signature}?cluster=devnet`}
              target="_blank"
              className="view-tx"
            >
              查看交易
            </a>
          </div>
        ))}
      </div>
    </div>
  );
};

function formatActivityType(type: string) {
  const types = {
    "check-in": "每日签到",
    "points-update": "积分变动",
    "wallet-initialized": "账户初始化",
    "checkin-initialized": "签到初始化",
  };
  return types[type] || type;
}

function formatDetails(activity: any) {
  if (activity.type === "check-in") {
    return `获得 ${activity.details.pointsEarned} 积分（总签到 ${activity.details.totalCheckins} 次）`;
  }
  if (activity.type === "points-update") {
    const change = activity.details.change;
    return change > 0 
      ? `获得 ${change} 积分` 
      : `消耗 ${Math.abs(change)} 积分`;
  }
  return "";
}
```

---

## 方法2: 使用Solana RPC直接查询

### 命令行查询

```bash
# 获取用户最近的交易签名
solana transaction-history <用户钱包地址> --url devnet --limit 20

# 查看具体交易详情
solana confirm -v <交易签名> --url devnet
```

### JavaScript/TypeScript实现

```typescript
import { Connection, PublicKey } from "@solana/web3.js";

async function getRecentActivity(userWallet: PublicKey) {
  const connection = new Connection("https://api.devnet.solana.com");
  
  // 1. 获取交易签名列表
  const signatures = await connection.getSignaturesForAddress(
    userWallet,
    { limit: 50 }
  );

  // 2. 获取交易详情（带日志）
  const activities = [];
  
  for (const sig of signatures) {
    const tx = await connection.getTransaction(sig.signature, {
      maxSupportedTransactionVersion: 0,
    });

    if (!tx || !tx.meta) continue;

    // 3. 检查是否涉及我们的合约
    const isPofTx = tx.transaction.message.staticAccountKeys.some(
      key => key.toString() === "E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV"
    );

    const isCheckinTx = tx.transaction.message.staticAccountKeys.some(
      key => key.toString() === "2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX"
    );

    if (!isPofTx && !isCheckinTx) continue;

    // 4. 解析日志
    const logs = tx.meta.logMessages || [];
    
    activities.push({
      signature: sig.signature,
      blockTime: sig.blockTime,
      logs: logs.filter(log => log.includes("Program log:")),
      type: isCheckinTx ? "checkin" : "pof",
    });
  }

  return activities;
}

// 使用
const userWallet = new PublicKey("用户地址");
const activities = await getRecentActivity(userWallet);

activities.forEach(act => {
  console.log(`${new Date(act.blockTime * 1000).toLocaleString()}`);
  console.log(`Type: ${act.type}`);
  act.logs.forEach(log => console.log(`  ${log}`));
  console.log();
});
```

---

## 方法3: 监听实时活动

### WebSocket订阅

```typescript
// 监听用户钱包的所有交易
const subscriptionId = connection.onLogs(
  userWallet,
  (logs, context) => {
    console.log("New transaction:", logs);
    // 解析日志，更新UI
  },
  "confirmed"
);

// 清理
connection.removeOnLogsListener(subscriptionId);
```

### 监听特定账户变化

```typescript
// 监听用户积分账户
const [pointsPda] = getUserPointsPda(userWallet, pofProgramId);

const subscriptionId = connection.onAccountChange(
  pointsPda,
  (accountInfo) => {
    // 积分账户更新时触发
    const decoded = pofProgram.account.walletPoints.coder.accounts.decode(
      "walletPoints",
      accountInfo.data
    );
    console.log("积分更新:", decoded.points.toNumber());
  },
  "confirmed"
);
```

---

## 方法4: 使用第三方服务（最强大）

### Helius API

```typescript
const HELIUS_API_KEY = "your-api-key";

async function getActivityWithHelius(userWallet: string) {
  const response = await fetch(
    `https://api.helius.xyz/v0/addresses/${userWallet}/transactions?api-key=${HELIUS_API_KEY}&type=TRANSFER`
  );
  
  const data = await response.json();
  return data;
}
```

### The Graph（需要创建Subgraph）

可以创建一个Subgraph来索引合约事件，提供GraphQL查询。

---

## 方法5: 解析链上数据（当前状态）

虽然无法获取完整历史，但可以获取当前状态：

```typescript
// 获取用户当前状态
async function getUserCurrentState(userWallet: PublicKey) {
  const [pointsPda] = getUserPointsPda(userWallet, pofProgramId);
  const [checkinPda] = getCheckinRecordPda(userWallet, checkinProgramId);

  // 并行获取
  const [pointsAccount, checkinAccount] = await Promise.all([
    pofProgram.account.walletPoints.fetch(pointsPda),
    checkinProgram.account.checkinRecord.fetch(checkinPda),
  ]);

  return {
    currentPoints: pointsAccount.points.toNumber(),
    totalCheckins: checkinAccount.totalCheckins.toNumber(),
    lastCheckin: checkinAccount.lastCheckin.toNumber(),
    lastCheckinDate: new Date(checkinAccount.lastCheckin.toNumber() * 1000),
  };
}
```

---

## 🎯 推荐方案

### 对于前端应用

**组合方案**（最佳体验）：

1. **当前状态** - 直接从合约账户读取
   ```typescript
   const points = await pofProgram.account.walletPoints.fetch(pda);
   ```

2. **最近活动** - 查询最近20-50笔交易
   ```typescript
   const activities = await getUserActivity(wallet, { limit: 20 });
   ```

3. **实时更新** - WebSocket订阅账户变化
   ```typescript
   connection.onAccountChange(pointsPda, callback);
   ```

4. **缓存到数据库** - 后端定期同步链上数据
   - 用户签到时，后端也记录到数据库
   - 前端查询时从数据库读取历史
   - 定期从链上同步确保一致性

---

## 📊 数据对比

| 方法 | 优点 | 缺点 | 适用场景 |
|------|------|------|----------|
| **查询交易历史** | 无需修改合约 | 需要解析日志 | 显示最近活动 |
| **账户订阅** | 实时更新 | 仅当前状态 | 实时积分显示 |
| **第三方API** | 功能强大 | 需要付费 | 复杂查询 |
| **后端数据库** | 查询快速 | 需要维护 | 历史数据分析 |

---

## 💡 最佳实践

### 完整方案架构

```
前端 ────┬──→ Solana RPC ──→ 获取最近20笔活动
         │
         ├──→ WebSocket ──→ 实时监听新活动  
         │
         └──→ 后端API ──→ 获取完整历史（数据库）
                         ↓
                      后端定期同步链上数据
```

### 前端实现

```typescript
// 组合使用多种方法
const UserDashboard = () => {
  const { points } = usePoF();                    // 当前积分
  const { activities } = useUserActivity(20);     // 最近20笔活动
  const { checkinInfo } = useCheckin();           // 签到状态

  return (
    <div>
      <PointsDisplay points={points} />
      <CheckinButton info={checkinInfo} />
      <ActivityHistory activities={activities} />
    </div>
  );
};
```

---

## 🔧 示例代码

### 完整示例：获取并显示活动

```typescript
import { Connection, PublicKey } from "@solana/web3.js";

async function displayUserActivity(userWalletAddress: string) {
  const connection = new Connection("https://api.devnet.solana.com");
  const wallet = new PublicKey(userWalletAddress);
  
  const POF_ID = "E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV";
  const CHECKIN_ID = "2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX";

  // 获取签名
  const sigs = await connection.getSignaturesForAddress(wallet, { limit: 30 });
  
  console.log(`找到 ${sigs.length} 笔交易\n`);

  // 遍历交易
  for (const sig of sigs) {
    const tx = await connection.getParsedTransaction(sig.signature, {
      maxSupportedTransactionVersion: 0,
    });

    if (!tx?.meta) continue;

    const logs = tx.meta.logMessages || [];
    const keys = tx.transaction.message.accountKeys.map(k => k.pubkey.toString());

    // 只显示与我们合约相关的
    if (!keys.includes(POF_ID) && !keys.includes(CHECKIN_ID)) continue;

    console.log(`📅 ${new Date(sig.blockTime! * 1000).toLocaleString()}`);
    console.log(`🔗 ${sig.signature.slice(0, 8)}...`);
    
    // 解析活动类型
    const checkinLog = logs.find(l => l.includes("Check-in successful"));
    const pointsLog = logs.find(l => l.includes("points updated"));
    
    if (checkinLog) {
      console.log("   ✅ 每日签到 (+10积分)");
    } else if (pointsLog) {
      const match = pointsLog.match(/updated by ([-\d]+)/);
      if (match) {
        const delta = parseInt(match[1]);
        console.log(`   💰 积分${delta > 0 ? "增加" : "减少"} ${Math.abs(delta)}`);
      }
    }
    console.log();
  }
}

// 运行
displayUserActivity("3nPsYeVZ92h8a5idvjmrgEVr5iGgU948VEoKdCj3bbQF");
```

---

## 📈 性能优化建议

1. **限制查询数量** - `limit: 20` 避免过多RPC调用
2. **缓存结果** - 将活动缓存到localStorage
3. **分页加载** - 支持"加载更多"
4. **后台同步** - 定期后台刷新，而非每次访问
5. **WebSocket** - 实时监听新活动，减少轮询

---

## 总结

**✅ 不修改合约也能获取活动历史**

推荐方案：
1. 使用 `getSignaturesForAddress` 获取交易列表
2. 解析交易日志提取活动信息
3. 在前端展示最近N条活动
4. 使用WebSocket实时更新

前端只需要调用Solana RPC API，无需修改合约代码！

