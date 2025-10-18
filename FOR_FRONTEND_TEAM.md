# 给前端团队的交付文件

## 📦 交付方式

### 方式1: 使用打包文件（推荐）

**文件**: `sportsx-frontend-package.zip` (18KB)

解压后包含所有必需文件和文档。

```bash
# 前端开发者操作
unzip sportsx-frontend-package.zip
cd frontend-package
# 查看 README.md 获取使用说明
```

### 方式2: 直接复制单个文件

如果只需要核心文件：

```bash
# 必需文件（前端项目根目录执行）
mkdir -p src/idl
cp path/to/sportsx-pof-contract/target/idl/sportsx_pof.json src/idl/
cp path/to/sportsx-pof-contract/target/idl/sportsx_checkin.json src/idl/
```

---

## 📋 交付清单

### ✅ 核心文件（必须使用）

| 序号 | 文件 | 大小 | 说明 |
|------|------|------|------|
| 1 | `idl/sportsx_pof.json` | 9.1KB | PoF合约接口定义 |
| 2 | `idl/sportsx_checkin.json` | 7.1KB | 签到合约接口定义 |
| 3 | `DEVNET_ADDRESSES.txt` | <1KB | 部署地址配置 |

### ✅ 类型文件（推荐使用）

| 序号 | 文件 | 大小 | 说明 |
|------|------|------|------|
| 4 | `types/sportsx_pof.ts` | 9.3KB | TypeScript类型定义 |
| 5 | `types/sportsx_checkin.ts` | 7.3KB | TypeScript类型定义 |

### ✅ 文档文件（参考资料）

| 序号 | 文件 | 大小 | 说明 |
|------|------|------|------|
| 6 | `docs/FRONTEND_INTEGRATION.md` | 19KB | 完整集成指南（含代码） |
| 7 | `docs/DEPLOYMENT_SUMMARY.md` | 6KB | 部署信息总结 |
| 8 | `README.md` | 3KB | 快速开始指南 |

---

## 🎯 前端需要做什么

### Step 1: 环境配置（5分钟）

```bash
# 1. 解压文件包
unzip sportsx-frontend-package.zip

# 2. 复制IDL到前端项目
cp frontend-package/idl/*.json your-project/src/idl/

# 3. 创建环境变量文件
cat frontend-package/DEVNET_ADDRESSES.txt
# 复制地址到 .env 文件
```

### Step 2: 安装依赖（2分钟）

```bash
npm install @coral-xyz/anchor @solana/web3.js \
  @solana/wallet-adapter-react \
  @solana/wallet-adapter-react-ui \
  @solana/wallet-adapter-wallets
```

### Step 3: 集成代码（30分钟）

参考 `docs/FRONTEND_INTEGRATION.md`，它提供：

✅ **现成的代码**（复制即用）:
- `usePoF` Hook - 积分管理
- `useCheckin` Hook - 签到管理
- `PointsDisplay` 组件 - 积分显示
- `CheckinButton` 组件 - 签到按钮
- PDA辅助函数
- 完整App示例

✅ **包含**:
- 错误处理
- 加载状态
- 自动刷新
- 样式代码

---

## 📊 合约信息

### 网络环境

- **网络**: Solana Devnet（测试网）
- **RPC**: `https://api.devnet.solana.com`
- **浏览器**: https://explorer.solana.com/?cluster=devnet

### 程序ID

```typescript
// PoF积分合约
const POF_PROGRAM_ID = "E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV";

// 签到合约
const CHECKIN_PROGRAM_ID = "2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX";
```

### 功能特性

**PoF合约**:
- 💰 每个用户独立的积分账户
- 📊 支持查询任何用户的积分
- 🔐 Admin和授权合约可更新积分

**签到合约**:
- ⏰ 24小时签到间隔限制
- 🎁 每次签到奖励10积分
- 📈 记录签到次数和时间
- 🔗 通过CPI自动调用PoF合约加分

---

## 🔍 快速测试

前端开发者可以用以下地址在Devnet测试：

```typescript
// 1. 连接钱包到Devnet
// 2. 在 https://faucet.solana.com 获取测试SOL
// 3. 初始化账户
// 4. 尝试签到
// 5. 查询积分（应该显示10）
```

---

## 🆘 技术支持

### 文档资源

| 文档 | 查看位置 | 内容 |
|------|---------|------|
| 快速开始 | `README.md` | 5分钟上手 |
| 完整集成 | `docs/FRONTEND_INTEGRATION.md` | React集成完整指南 |
| 部署信息 | `docs/DEPLOYMENT_SUMMARY.md` | 合约地址和状态 |

### 常见问题

**Q: IDL文件是什么？**  
A: 合约接口定义，类似于API文档，Anchor自动生成。前端必需。

**Q: 如何派生PDA地址？**  
A: 使用 `PublicKey.findProgramAddressSync()` + 正确的seeds。参考文档中的辅助函数。

**Q: 签到失败怎么办？**  
A: 检查：1) 账户是否初始化 2) 是否满24小时 3) 钱包是否连接 4) 是否有SOL支付gas

**Q: 如何测试？**  
A: 连接到Devnet，使用测试钱包，所有操作免费（需要从faucet获取测试SOL）

---

## ✨ 重要提示

1. **IDL文件版本**: 确保使用这个包中的IDL，它们与部署的合约版本匹配
2. **PDA推导**: 必须使用正确的seeds，参考文档中的函数
3. **网络环境**: 当前部署在Devnet，主网需要重新部署
4. **钱包适配器**: 建议支持Phantom和Solflare钱包
5. **错误处理**: 参考文档中的错误处理示例

---

## 📦 包内容总览

```
总文件数: 8个
总大小: ~65KB（压缩后18KB）

核心文件: 2个 IDL
类型文件: 2个 TS
配置文件: 1个 TXT
文档文件: 3个 MD
```

**一切准备就绪，可以开始前端开发了！** 🚀

