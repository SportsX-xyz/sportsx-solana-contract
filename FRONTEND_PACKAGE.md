# 前端集成包 - 文件清单

## 📦 必需文件

### 1. IDL文件（必须）

这些文件包含合约的接口定义，前端必需。

```
📁 需要提供的文件：
├── target/idl/sportsx_pof.json         # PoF合约IDL
└── target/idl/sportsx_checkin.json     # 签到合约IDL
```

**位置**：`target/idl/`  
**用途**：
- 生成TypeScript类型定义
- 自动创建合约调用方法
- 验证交易参数

**前端使用**：
```typescript
import pofIdl from './idl/sportsx_pof.json';
import checkinIdl from './idl/sportsx_checkin.json';

const pofProgram = new Program(pofIdl, provider);
```

---

### 2. 类型定义文件（可选但推荐）

TypeScript类型定义，提供更好的开发体验。

```
📁 推荐提供的文件：
├── target/types/sportsx_pof.ts         # PoF合约类型
└── target/types/sportsx_checkin.ts     # 签到合约类型
```

**位置**：`target/types/`  
**用途**：
- TypeScript类型提示
- IDE自动补全
- 编译时类型检查

**前端使用**：
```typescript
import { SportsxPof } from './types/sportsx_pof';
import { SportsxCheckin } from './types/sportsx_checkin';

const program = workspace.SportsxPof as Program<SportsxPof>;
```

---

### 3. 配置信息文件（必须）

部署地址和配置信息。

```
📁 提供配置文件：
├── DEVNET_ADDRESSES.txt                # 所有Devnet地址
└── DEPLOYMENT_SUMMARY.md               # 部署总结
```

**内容**：
```
POF_PROGRAM_ID=E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV
CHECKIN_PROGRAM_ID=2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX
GLOBAL_STATE_PDA=2hcdXc13orDhHYkTCxLPkKwbcNwcXWMjkdYXK1BPw2ym
CHECKIN_AUTHORITY_PDA=6VhHaApS3DFizoVfaFgskskV2XN2hFkmfZEh1VqF4XMA
```

---

### 4. 集成文档（强烈推荐）

帮助前端开发者快速集成。

```
📁 文档文件：
└── FRONTEND_INTEGRATION.md             # 完整前端集成指南
```

**包含内容**：
- ✅ 依赖安装说明
- ✅ 完整的React Hooks代码
- ✅ 现成的组件示例
- ✅ PDA派生辅助函数
- ✅ 错误处理示例
- ✅ 样式代码

---

## 📂 打包方式

### 方式1: 创建压缩包

```bash
# 自动打包所有必需文件
cd /Users/renhaozhang/Documents/sportsx/sportsx-pof-contract

# 创建前端包目录
mkdir -p frontend-package/idl
mkdir -p frontend-package/types
mkdir -p frontend-package/docs

# 复制文件
cp target/idl/*.json frontend-package/idl/
cp target/types/*.ts frontend-package/types/
cp DEVNET_ADDRESSES.txt frontend-package/
cp FRONTEND_INTEGRATION.md frontend-package/docs/
cp DEPLOYMENT_SUMMARY.md frontend-package/docs/

# 打包
tar -czf sportsx-frontend-package.tar.gz frontend-package/
# 或
zip -r sportsx-frontend-package.zip frontend-package/
```

### 方式2: Git仓库共享

前端可以直接从这个仓库获取文件：

```bash
# 前端开发者执行
git clone <this-repo>
cd sportsx-pof-contract

# 复制需要的文件到前端项目
cp target/idl/*.json ../frontend/src/idl/
cp target/types/*.ts ../frontend/src/types/
cp DEVNET_ADDRESSES.txt ../frontend/
```

### 方式3: NPM包（高级）

如果经常更新，可以发布为NPM包：

```bash
# 创建package.json
cat > frontend-package/package.json << EOF
{
  "name": "@sportsx/contracts",
  "version": "1.0.0",
  "description": "SportsX Smart Contracts IDL and Types",
  "main": "index.js",
  "files": ["idl/", "types/"],
  "keywords": ["solana", "sportsx", "anchor"]
}
EOF

# 发布到私有NPM或GitHub Packages
npm publish
```

---

## 📋 文件清单和说明

### 必需文件（3个）

| 文件 | 大小 | 用途 | 优先级 |
|------|------|------|--------|
| `target/idl/sportsx_pof.json` | ~10KB | PoF合约接口定义 | 🔴 必须 |
| `target/idl/sportsx_checkin.json` | ~8KB | 签到合约接口定义 | 🔴 必须 |
| `DEVNET_ADDRESSES.txt` | <1KB | 部署地址配置 | 🔴 必须 |

### 推荐文件（2个）

| 文件 | 大小 | 用途 | 优先级 |
|------|------|------|--------|
| `target/types/sportsx_pof.ts` | ~10KB | TypeScript类型定义 | 🟡 推荐 |
| `target/types/sportsx_checkin.ts` | ~8KB | TypeScript类型定义 | 🟡 推荐 |

### 文档文件（2个）

| 文件 | 大小 | 用途 | 优先级 |
|------|------|------|--------|
| `FRONTEND_INTEGRATION.md` | ~25KB | 集成指南和代码示例 | 🟢 有帮助 |
| `DEPLOYMENT_SUMMARY.md` | ~8KB | 部署信息和使用说明 | 🟢 有帮助 |

---

## 🎯 前端开发者需要做什么

### 第1步：获取文件

```bash
# 从你这里获取以下文件：
1. sportsx_pof.json          # 放到 src/idl/
2. sportsx_checkin.json      # 放到 src/idl/
3. DEVNET_ADDRESSES.txt      # 放到项目根目录
4. FRONTEND_INTEGRATION.md   # 参考文档
```

### 第2步：安装依赖

```bash
npm install @coral-xyz/anchor @solana/web3.js @solana/wallet-adapter-react @solana/wallet-adapter-react-ui
```

### 第3步：配置环境变量

创建 `.env` 文件：
```env
REACT_APP_POF_PROGRAM_ID=E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV
REACT_APP_CHECKIN_PROGRAM_ID=2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX
REACT_APP_SOLANA_RPC_URL=https://api.devnet.solana.com
```

### 第4步：使用示例代码

参考 `FRONTEND_INTEGRATION.md` 中的：
- PDA辅助函数 → 复制到 `src/lib/pda.ts`
- React Hooks → 复制到 `src/hooks/`
- 组件示例 → 参考实现UI

---

## 💾 快速打包命令

运行以下命令创建前端包：

```bash
cd /Users/renhaozhang/Documents/sportsx/sportsx-pof-contract

# 创建前端包
mkdir -p frontend-package/{idl,types,docs}

# 复制文件
cp target/idl/*.json frontend-package/idl/
cp target/types/*.ts frontend-package/types/
cp DEVNET_ADDRESSES.txt frontend-package/
cp FRONTEND_INTEGRATION.md frontend-package/docs/
cp DEPLOYMENT_SUMMARY.md frontend-package/docs/

# 创建README
cat > frontend-package/README.md << 'EOF'
# SportsX 合约前端集成包

## 包含文件

- `idl/` - 合约IDL文件（必需）
- `types/` - TypeScript类型定义（推荐）
- `docs/` - 集成文档
- `DEVNET_ADDRESSES.txt` - 部署地址

## 快速开始

1. 将 `idl/*.json` 复制到你的项目 `src/idl/`
2. 将 `DEVNET_ADDRESSES.txt` 中的地址添加到 `.env`
3. 参考 `docs/FRONTEND_INTEGRATION.md` 集成

## 环境变量

```env
REACT_APP_POF_PROGRAM_ID=E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV
REACT_APP_CHECKIN_PROGRAM_ID=2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX
REACT_APP_SOLANA_RPC_URL=https://api.devnet.solana.com
```

## 文档

详细集成指南请查看 `docs/FRONTEND_INTEGRATION.md`
EOF

# 打包
zip -r sportsx-frontend-package.zip frontend-package/

echo "✅ 前端包已创建: sportsx-frontend-package.zip"
```

---

## 📤 交付清单

给前端开发者提供：

### 最小交付（必须）
- ✅ `sportsx_pof.json`
- ✅ `sportsx_checkin.json`  
- ✅ 程序ID配置（环境变量）

### 标准交付（推荐）
- ✅ 上述必须文件
- ✅ TypeScript类型文件
- ✅ `FRONTEND_INTEGRATION.md`

### 完整交付（最佳）
- ✅ 上述所有文件
- ✅ `DEPLOYMENT_SUMMARY.md`
- ✅ 打包成zip文件

---

## 🔍 验证清单

前端收到文件后，应该能够：

- [ ] 导入IDL创建Program实例
- [ ] 派生正确的PDA地址
- [ ] 调用合约方法
- [ ] 处理交易和错误
- [ ] 查询链上数据

---

需要我现在创建这个前端包吗？

