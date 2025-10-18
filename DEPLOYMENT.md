# 部署记录

## Devnet 部署信息

**部署时间**: 2025-10-18  
**网络**: Devnet  
**管理员钱包**: `3nPsYeVZ92h8a5idvjmrgEVr5iGgU948VEoKdCj3bbQF`

### 程序地址

| 合约 | Program ID | 浏览器链接 |
|------|-----------|----------|
| **PoF** | `E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV` | [查看](https://explorer.solana.com/address/E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV?cluster=devnet) |
| **Check-in** | `2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX` | [查看](https://explorer.solana.com/address/2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX?cluster=devnet) |

### 重要PDAs

| 账户 | 地址 | 说明 |
|------|------|------|
| **Global State** | `2hcdXc13orDhHYkTCxLPkKwbcNwcXWMjkdYXK1BPw2ym` | PoF全局状态 |
| **Checkin Authority** | `6VhHaApS3DFizoVfaFgskskV2XN2hFkmfZEh1VqF4XMA` | 签到合约统一权限 |

### 初始化交易

| 操作 | 交易签名 | 链接 |
|------|---------|------|
| PoF全局状态初始化 | `3hnrtBXQrpp468BXs49ASGCQU8RgUBzUcNrgw4mXdevUQ6xeFJihNAQ8teLyEUPdn1kN4zDfoRRPHzvnKTF66zjo` | [查看](https://explorer.solana.com/tx/3hnrtBXQrpp468BXs49ASGCQU8RgUBzUcNrgw4mXdevUQ6xeFJihNAQ8teLyEUPdn1kN4zDfoRRPHzvnKTF66zjo?cluster=devnet) |
| 授权签到合约 | `4TYdTEBd1spsH7UBhdqMA4DGV38KC4Mt84GLS6cYA6enPb8WuhCstv451QU7g7CcCy5HxsQmvHRzDvruq6WrHfWg` | [查看](https://explorer.solana.com/tx/4TYdTEBd1spsH7UBhdqMA4DGV38KC4Mt84GLS6cYA6enPb8WuhCstv451QU7g7CcCy5HxsQmvHRzDvruq6WrHfWg?cluster=devnet) |

---

## 合约状态

✅ **PoF合约**
- 已部署到 Devnet
- 全局状态已初始化
- 管理员: `3nPsYeVZ92h8a5idvjmrgEVr5iGgU948VEoKdCj3bbQF`
- 已授权合约数: 1

✅ **签到合约**
- 已部署到 Devnet
- 已被PoF合约授权
- 可以通过CPI调用PoF增加积分

---

## 前端使用

### 环境变量配置（.env）

```env
REACT_APP_SOLANA_NETWORK=devnet
REACT_APP_SOLANA_RPC_URL=https://api.devnet.solana.com

REACT_APP_POF_PROGRAM_ID=E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV
REACT_APP_CHECKIN_PROGRAM_ID=2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX
```

### 使用示例

```typescript
// 连接到已部署的合约
const pofProgramId = new PublicKey(process.env.REACT_APP_POF_PROGRAM_ID);
const checkinProgramId = new PublicKey(process.env.REACT_APP_CHECKIN_PROGRAM_ID);

// 派生用户积分PDA
const [pointsPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("wallet_points"), userWallet.toBuffer()],
  pofProgramId
);

// 派生签到记录PDA
const [checkinPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("checkin_record"), userWallet.toBuffer()],
  checkinProgramId
);

// 派生统一签到权限PDA（已授权）
const [authorityPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("checkin_authority")],
  checkinProgramId
);
```

---

## 测试合约

### 在Devnet测试

```bash
# 连接到Devnet
export ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
export ANCHOR_WALLET=~/.config/solana/id.json

# 运行初始化脚本（已执行）
node scripts/deploy-init.js

# 运行完整测试（可选）
anchor test --skip-build --skip-deploy --provider.cluster devnet
```

### 手动测试用户签到流程

1. **初始化用户积分账户**
2. **初始化用户签到记录**
3. **执行签到** → 自动获得10积分
4. **查询积分** → 验证积分增加
5. **24小时后再次签到** → 再获得10积分

---

## 管理操作

### 授权新合约

```bash
# 使用脚本授权其他合约
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
ANCHOR_WALLET=~/.config/solana/id.json \
node -e "
const anchor = require('@coral-xyz/anchor');
const { PublicKey } = require('@solana/web3.js');

(async () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.SportsxPof;
  
  const [globalState] = PublicKey.findProgramAddressSync(
    [Buffer.from('global_state')],
    program.programId
  );
  
  const contractToAuthorize = new PublicKey('新合约地址');
  
  await program.methods
    .authorizeContract(contractToAuthorize)
    .accounts({
      globalState,
      admin: provider.wallet.publicKey,
    })
    .rpc();
  
  console.log('✅ Contract authorized');
})();
"
```

### 查看当前授权列表

```bash
solana account 2hcdXc13orDhHYkTCxLPkKwbcNwcXWMjkdYXK1BPw2ym --url devnet
```

---

## 升级合约

如需更新合约代码：

```bash
# 1. 修改代码
# 2. 重新构建
anchor build

# 3. 升级部署（保持相同程序ID）
anchor upgrade target/deploy/sportsx_pof.so \
  --program-id E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV \
  --provider.cluster devnet

anchor upgrade target/deploy/sportsx_checkin.so \
  --program-id 2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX \
  --provider.cluster devnet
```

---

## 监控和调试

### 查看程序日志

```bash
# 实时查看交易日志
solana logs --url devnet
```

### 查看账户信息

```bash
# 查看全局状态
solana account 2hcdXc13orDhHYkTCxLPkKwbcNwcXWMjkdYXK1BPw2ym --url devnet

# 查看用户积分（需替换PDA）
solana account <用户积分PDA> --url devnet
```

### Solana Explorer

- **PoF合约**: https://explorer.solana.com/address/E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV?cluster=devnet
- **签到合约**: https://explorer.solana.com/address/2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX?cluster=devnet
- **全局状态**: https://explorer.solana.com/address/2hcdXc13orDhHYkTCxLPkKwbcNwcXWMjkdYXK1BPw2ym?cluster=devnet

---

## 安全注意事项

⚠️ **重要提醒**:

1. **保护私钥**: `~/.config/solana/id.json` 是你的管理员钱包，请妥善保管
2. **审计代码**: 部署到主网前，务必进行安全审计
3. **权限管理**: 定期检查授权合约列表
4. **升级权限**: 当前钱包拥有合约升级权限
5. **备份**: 保存好程序keypair文件

---

## 后续步骤

1. ✅ 合约已部署
2. ✅ 全局状态已初始化
3. ✅ 签到合约已授权
4. 📝 复制 `.env.devnet` 到前端项目
5. 📝 参考 `FRONTEND_INTEGRATION.md` 集成前端
6. 🧪 在Devnet测试完整流程
7. 🚀 准备好后部署到主网

