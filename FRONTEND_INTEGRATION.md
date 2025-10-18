# 前端集成指南

本文档说明如何在前端应用中集成 SportsX PoF 和 Check-in 合约。

## 📦 安装依赖

```bash
npm install @coral-xyz/anchor @solana/web3.js @solana/wallet-adapter-react @solana/wallet-adapter-react-ui @solana/wallet-adapter-wallets
```

## 🔧 基础配置

### 1. 初始化程序连接

```typescript
// src/lib/anchor.ts
import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Connection } from "@solana/web3.js";
import { Program, AnchorProvider } from "@coral-xyz/anchor";

// 导入IDL（从 target/idl/ 复制到前端项目）
import pofIdl from "./idl/sportsx_pof.json";
import checkinIdl from "./idl/sportsx_checkin.json";

// 程序ID（根据实际部署更新）
const POF_PROGRAM_ID = new PublicKey("E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV");
const CHECKIN_PROGRAM_ID = new PublicKey("2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX");

// 网络配置
const NETWORK = "devnet"; // 或 "mainnet-beta"
const connection = new Connection(
  NETWORK === "devnet" 
    ? "https://api.devnet.solana.com"
    : "https://api.mainnet-beta.solana.com",
  "confirmed"
);

// 创建Provider
export const getProvider = (wallet: any) => {
  return new AnchorProvider(connection, wallet, {
    preflightCommitment: "confirmed",
  });
};

// 获取程序实例
export const getPofProgram = (wallet: any) => {
  const provider = getProvider(wallet);
  return new Program(pofIdl as any, provider);
};

export const getCheckinProgram = (wallet: any) => {
  const provider = getProvider(wallet);
  return new Program(checkinIdl as any, provider);
};
```

### 2. PDA辅助函数

```typescript
// src/lib/pda.ts
import { PublicKey } from "@solana/web3.js";

export class PdaHelper {
  // PoF相关PDA
  static getGlobalStatePda(programId: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("global_state")],
      programId
    );
  }

  static getWalletPointsPda(
    wallet: PublicKey,
    programId: PublicKey
  ): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("wallet_points"), wallet.toBuffer()],
      programId
    );
  }

  // Check-in相关PDA
  static getCheckinRecordPda(
    wallet: PublicKey,
    programId: PublicKey
  ): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("checkin_record"), wallet.toBuffer()],
      programId
    );
  }

  static getCheckinAuthorityPda(
    wallet: PublicKey,
    programId: PublicKey
  ): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("checkin_authority"), wallet.toBuffer()],
      programId
    );
  }
}
```

## 🪝 React Hooks

### usePoF Hook

```typescript
// src/hooks/usePoF.ts
import { useState, useEffect, useCallback } from "react";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { SystemProgram } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { getPofProgram } from "../lib/anchor";
import { PdaHelper } from "../lib/pda";

export const usePoF = () => {
  const { connection } = useConnection();
  const wallet = useWallet();
  const [points, setPoints] = useState<number>(0);
  const [loading, setLoading] = useState(false);
  const [initialized, setInitialized] = useState(false);

  // 查询积分
  const fetchPoints = useCallback(async () => {
    if (!wallet.publicKey) {
      setPoints(0);
      setInitialized(false);
      return;
    }

    try {
      const program = getPofProgram(wallet);
      const [pointsPda] = PdaHelper.getWalletPointsPda(
        wallet.publicKey,
        program.programId
      );

      const account = await program.account.walletPoints.fetch(pointsPda);
      setPoints(account.points.toNumber());
      setInitialized(true);
    } catch (err) {
      console.log("积分账户未初始化");
      setPoints(0);
      setInitialized(false);
    }
  }, [wallet.publicKey]);

  // 初始化积分账户
  const initializeWallet = useCallback(async () => {
    if (!wallet.publicKey) {
      throw new Error("请先连接钱包");
    }

    setLoading(true);
    try {
      const program = getPofProgram(wallet);
      const [pointsPda] = PdaHelper.getWalletPointsPda(
        wallet.publicKey,
        program.programId
      );

      const tx = await program.methods
        .initializeWallet()
        .accounts({
          walletPoints: pointsPda,
          wallet: wallet.publicKey,
          payer: wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("初始化交易:", tx);
      await fetchPoints();
      return tx;
    } catch (err: any) {
      console.error("初始化失败:", err);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [wallet.publicKey, fetchPoints]);

  // 自动查询
  useEffect(() => {
    fetchPoints();
  }, [fetchPoints]);

  return {
    points,
    loading,
    initialized,
    initializeWallet,
    refreshPoints: fetchPoints,
  };
};
```

### useCheckin Hook

```typescript
// src/hooks/useCheckin.ts
import { useState, useEffect, useCallback } from "react";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { SystemProgram } from "@solana/web3.js";
import { getPofProgram, getCheckinProgram } from "../lib/anchor";
import { PdaHelper } from "../lib/pda";

export interface CheckinInfo {
  lastCheckin: number; // 时间戳（毫秒）
  totalCheckins: number;
  canCheckin: boolean;
  timeUntilNext: number; // 秒
}

export const useCheckin = () => {
  const { connection } = useConnection();
  const wallet = useWallet();
  const [checkinInfo, setCheckinInfo] = useState<CheckinInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [initialized, setInitialized] = useState(false);

  // 查询签到信息
  const fetchCheckinInfo = useCallback(async () => {
    if (!wallet.publicKey) {
      setCheckinInfo(null);
      setInitialized(false);
      return;
    }

    try {
      const program = getCheckinProgram(wallet);
      const [recordPda] = PdaHelper.getCheckinRecordPda(
        wallet.publicKey,
        program.programId
      );

      const info = await program.methods
        .getCheckinInfo()
        .accounts({
          checkinRecord: recordPda,
        })
        .view();

      setCheckinInfo({
        lastCheckin: info.lastCheckin.toNumber() * 1000,
        totalCheckins: info.totalCheckins.toNumber(),
        canCheckin: info.canCheckin,
        timeUntilNext: info.timeUntilNextCheckin.toNumber(),
      });
      setInitialized(true);
    } catch (err) {
      console.log("签到记录未初始化");
      setCheckinInfo(null);
      setInitialized(false);
    }
  }, [wallet.publicKey]);

  // 初始化签到记录
  const initializeCheckin = useCallback(async () => {
    if (!wallet.publicKey) {
      throw new Error("请先连接钱包");
    }

    setLoading(true);
    try {
      const program = getCheckinProgram(wallet);
      const [recordPda] = PdaHelper.getCheckinRecordPda(
        wallet.publicKey,
        program.programId
      );

      const tx = await program.methods
        .initializeCheckin()
        .accounts({
          checkinRecord: recordPda,
          wallet: wallet.publicKey,
          payer: wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("初始化签到记录:", tx);
      await fetchCheckinInfo();
      return tx;
    } catch (err: any) {
      console.error("初始化签到失败:", err);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [wallet.publicKey, fetchCheckinInfo]);

  // 执行签到
  const performCheckin = useCallback(async () => {
    if (!wallet.publicKey) {
      throw new Error("请先连接钱包");
    }

    setLoading(true);
    try {
      const checkinProgram = getCheckinProgram(wallet);
      const pofProgram = getPofProgram(wallet);

      const [recordPda] = PdaHelper.getCheckinRecordPda(
        wallet.publicKey,
        checkinProgram.programId
      );

      const [authorityPda] = PdaHelper.getCheckinAuthorityPda(
        wallet.publicKey,
        checkinProgram.programId
      );

      const [globalStatePda] = PdaHelper.getGlobalStatePda(
        pofProgram.programId
      );

      const [pointsPda] = PdaHelper.getWalletPointsPda(
        wallet.publicKey,
        pofProgram.programId
      );

      const tx = await checkinProgram.methods
        .dailyCheckin()
        .accounts({
          checkinRecord: recordPda,
          wallet: wallet.publicKey,
          checkinAuthority: authorityPda,
          walletPoints: pointsPda,
          globalState: globalStatePda,
          pofProgram: pofProgram.programId,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("签到成功:", tx);
      await fetchCheckinInfo();
      return tx;
    } catch (err: any) {
      console.error("签到失败:", err);
      
      // 解析错误
      if (err.toString().includes("CheckinTooSoon")) {
        throw new Error("24小时内只能签到一次");
      }
      throw err;
    } finally {
      setLoading(false);
    }
  }, [wallet.publicKey, fetchCheckinInfo]);

  // 自动查询
  useEffect(() => {
    fetchCheckinInfo();
  }, [fetchCheckinInfo]);

  return {
    checkinInfo,
    loading,
    initialized,
    initializeCheckin,
    performCheckin,
    refreshCheckinInfo: fetchCheckinInfo,
  };
};
```

## 🎨 React组件示例

### 积分显示组件

```tsx
// src/components/PointsDisplay.tsx
import React from "react";
import { usePoF } from "../hooks/usePoF";

export const PointsDisplay: React.FC = () => {
  const { points, loading, initialized, initializeWallet } = usePoF();

  if (loading) {
    return <div>加载中...</div>;
  }

  if (!initialized) {
    return (
      <div className="points-uninitialized">
        <p>积分账户未初始化</p>
        <button onClick={initializeWallet}>
          初始化账户
        </button>
      </div>
    );
  }

  return (
    <div className="points-display">
      <div className="points-badge">
        <span className="label">我的积分</span>
        <span className="value">{points}</span>
      </div>
    </div>
  );
};
```

### 签到按钮组件

```tsx
// src/components/CheckinButton.tsx
import React, { useState } from "react";
import { useCheckin } from "../hooks/useCheckin";
import { usePoF } from "../hooks/usePoF";

export const CheckinButton: React.FC = () => {
  const { refreshPoints } = usePoF();
  const {
    checkinInfo,
    loading,
    initialized,
    initializeCheckin,
    performCheckin,
  } = useCheckin();
  const [message, setMessage] = useState("");

  const handleCheckin = async () => {
    try {
      setMessage("");
      await performCheckin();
      await refreshPoints();
      setMessage("签到成功！获得10积分 🎉");
    } catch (err: any) {
      setMessage(err.message || "签到失败");
    }
  };

  const handleInit = async () => {
    try {
      setMessage("");
      await initializeCheckin();
      setMessage("初始化成功！");
    } catch (err: any) {
      setMessage("初始化失败: " + err.message);
    }
  };

  // 格式化剩余时间
  const formatTimeRemaining = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}小时${minutes}分钟`;
  };

  if (!initialized) {
    return (
      <div className="checkin-widget">
        <p>签到记录未初始化</p>
        <button onClick={handleInit} disabled={loading}>
          {loading ? "初始化中..." : "初始化签到"}
        </button>
        {message && <p className="message">{message}</p>}
      </div>
    );
  }

  return (
    <div className="checkin-widget">
      <div className="checkin-stats">
        <p>总签到次数: <strong>{checkinInfo?.totalCheckins || 0}</strong></p>
        {checkinInfo?.lastCheckin && checkinInfo.lastCheckin > 0 && (
          <p>
            上次签到: {new Date(checkinInfo.lastCheckin).toLocaleString("zh-CN")}
          </p>
        )}
      </div>

      <button
        onClick={handleCheckin}
        disabled={!checkinInfo?.canCheckin || loading}
        className={`checkin-btn ${checkinInfo?.canCheckin ? "active" : "disabled"}`}
      >
        {loading
          ? "处理中..."
          : checkinInfo?.canCheckin
          ? "立即签到 (+10分)"
          : `${formatTimeRemaining(checkinInfo?.timeUntilNext || 0)}后可签到`}
      </button>

      {message && (
        <p className={`message ${message.includes("成功") ? "success" : "error"}`}>
          {message}
        </p>
      )}
    </div>
  );
};
```

### 完整应用示例

```tsx
// src/App.tsx
import React, { useMemo } from "react";
import {
  ConnectionProvider,
  WalletProvider,
} from "@solana/wallet-adapter-react";
import { WalletAdapterNetwork } from "@solana/wallet-adapter-base";
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
} from "@solana/wallet-adapter-wallets";
import { WalletModalProvider, WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { clusterApiUrl } from "@solana/web3.js";

import { PointsDisplay } from "./components/PointsDisplay";
import { CheckinButton } from "./components/CheckinButton";

// 导入钱包样式
import "@solana/wallet-adapter-react-ui/styles.css";
import "./App.css";

function App() {
  // 配置网络
  const network = WalletAdapterNetwork.Devnet;
  const endpoint = useMemo(() => clusterApiUrl(network), [network]);

  // 配置钱包
  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter(),
    ],
    []
  );

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          <div className="App">
            <header className="App-header">
              <h1>SportsX 每日签到</h1>
              <WalletMultiButton />
            </header>

            <main className="App-main">
              <PointsDisplay />
              <CheckinButton />
            </main>

            <footer>
              <p>每日签到可获得10积分</p>
            </footer>
          </div>
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}

export default App;
```

### 样式示例

```css
/* src/App.css */
.App {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.App-header {
  padding: 2rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
  color: white;
}

.App-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2rem;
  padding: 2rem;
}

.points-display {
  background: white;
  border-radius: 20px;
  padding: 2rem 4rem;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
}

.points-badge {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
}

.points-badge .label {
  font-size: 1rem;
  color: #666;
}

.points-badge .value {
  font-size: 3rem;
  font-weight: bold;
  color: #667eea;
}

.checkin-widget {
  background: white;
  border-radius: 20px;
  padding: 2rem;
  min-width: 400px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
}

.checkin-stats {
  margin-bottom: 1.5rem;
  text-align: center;
}

.checkin-stats p {
  margin: 0.5rem 0;
  color: #666;
}

.checkin-btn {
  width: 100%;
  padding: 1rem 2rem;
  font-size: 1.2rem;
  font-weight: bold;
  border: none;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.3s ease;
}

.checkin-btn.active {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.checkin-btn.active:hover {
  transform: translateY(-2px);
  box-shadow: 0 5px 15px rgba(102, 126, 234, 0.4);
}

.checkin-btn.disabled {
  background: #e0e0e0;
  color: #999;
  cursor: not-allowed;
}

.message {
  margin-top: 1rem;
  padding: 0.75rem;
  border-radius: 8px;
  text-align: center;
}

.message.success {
  background: #d4edda;
  color: #155724;
}

.message.error {
  background: #f8d7da;
  color: #721c24;
}
```

## 🔍 高级功能

### 监听账户变化

```typescript
// src/hooks/useAccountSubscription.ts
import { useEffect } from "react";
import { useConnection } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";

export const useAccountSubscription = (
  accountPda: PublicKey | null,
  onAccountChange: () => void
) => {
  const { connection } = useConnection();

  useEffect(() => {
    if (!accountPda) return;

    const subscriptionId = connection.onAccountChange(
      accountPda,
      (accountInfo) => {
        console.log("账户更新:", accountInfo);
        onAccountChange();
      },
      "confirmed"
    );

    return () => {
      connection.removeAccountChangeListener(subscriptionId);
    };
  }, [accountPda, connection, onAccountChange]);
};

// 使用示例
const { refreshPoints } = usePoF();
const { wallet } = useWallet();
const [pointsPda] = PdaHelper.getWalletPointsPda(wallet.publicKey, programId);

useAccountSubscription(pointsPda, refreshPoints);
```

### 批量初始化

```typescript
// src/lib/batchInit.ts
export const batchInitialize = async (wallet: any) => {
  const pofProgram = getPofProgram(wallet);
  const checkinProgram = getCheckinProgram(wallet);
  
  const [pointsPda] = PdaHelper.getWalletPointsPda(
    wallet.publicKey,
    pofProgram.programId
  );
  
  const [checkinPda] = PdaHelper.getCheckinRecordPda(
    wallet.publicKey,
    checkinProgram.programId
  );

  try {
    // 检查是否已初始化
    const [pointsExists, checkinExists] = await Promise.all([
      pofProgram.account.walletPoints.fetchNullable(pointsPda),
      checkinProgram.account.checkinRecord.fetchNullable(checkinPda),
    ]);

    const txs = [];

    // 初始化积分账户
    if (!pointsExists) {
      const tx = await pofProgram.methods
        .initializeWallet()
        .accounts({
          walletPoints: pointsPda,
          wallet: wallet.publicKey,
          payer: wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      txs.push(tx);
    }

    // 初始化签到记录
    if (!checkinExists) {
      const tx = await checkinProgram.methods
        .initializeCheckin()
        .accounts({
          checkinRecord: checkinPda,
          wallet: wallet.publicKey,
          payer: wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      txs.push(tx);
    }

    return txs;
  } catch (err) {
    console.error("批量初始化失败:", err);
    throw err;
  }
};
```

## 📋 常见问题

### Q: 如何获取IDL文件?
```bash
# 构建后IDL自动生成在
target/idl/sportsx_pof.json
target/idl/sportsx_checkin.json

# 复制到前端项目
cp target/idl/*.json ../frontend/src/idl/
```

### Q: 如何处理交易失败?
```typescript
try {
  const tx = await program.methods.dailyCheckin().rpc();
} catch (err: any) {
  if (err.logs) {
    console.log("交易日志:", err.logs);
  }
  
  // 解析错误码
  if (err.toString().includes("0x1770")) {
    // CheckinTooSoon error code
    alert("24小时内只能签到一次");
  }
}
```

### Q: 如何测试?
使用Devnet进行测试，确保：
1. 钱包连接到Devnet
2. 有足够的Devnet SOL（通过faucet获取）
3. 程序已部署到Devnet

---

更多详细信息，请查看 `README.md` 和测试文件。

