# SportsX Ticketing Contract API

## 合约概览

**程序ID**: `EFuMNTn1zfn6Zhvdq1Vjaxs83sz2gTWvDgjuJcKDYjhw`

一个 Solana 票务系统合约，支持票务购买、二级市场交易、检票等功能。

---

## 合约方法分类

### 1️⃣ 平台管理 (Platform Management)

#### `initialize_platform`
- **作用**: 初始化平台配置（仅部署时调用一次）
- **调用方**: 后端（部署脚本）
- **参数**:
  - `initial_fee_receiver`: 平台手续费接收地址
  - `initial_fee_usdc`: 平台手续费金额（USDC）
  - `backend_authority`: 后端签名授权地址
  - `event_admin`: 活动管理员地址

#### `update_platform_config`
- **作用**: 更新平台配置
- **调用方**: 后端（管理员操作）
- **参数**: 可选更新任何平台配置项

#### `toggle_pause`
- **作用**: 暂停/恢复平台运营
- **调用方**: 后端（管理员操作）

#### `transfer_authority`
- **作用**: 转移平台管理权限
- **调用方**: 后端（管理员操作）

---

### 2️⃣ 活动管理 (Event Management)

#### `create_event`
- **作用**: 创建新活动
- **调用方**: 后端
- **参数**:
  - `event_id`: 活动ID（最大32字符）
  - `metadata_uri`: 元数据URI（IPFS）
  - `start_time`: 活动开始时间
  - `end_time`: 活动结束时间
  - `ticket_release_time`: 门票开售时间
  - `stop_sale_before`: 停售时间（活动开始前多久）
  - `resale_fee_rate`: 转售手续费率（基点，100=1%）
  - `max_resale_times`: 最大转售次数

#### `update_event_status`
- **作用**: 更新活动状态（0=草稿，1=激活，2=禁用）
- **调用方**: 后端（活动组织者）

#### `add_checkin_operator`
- **作用**: 添加检票员
- **调用方**: 后端（活动管理员）

#### `remove_checkin_operator`
- **作用**: 移除检票员
- **调用方**: 后端（活动管理员）

---

### 3️⃣ 购票流程 (Purchase Flow)

#### `purchase_ticket`
- **作用**: 用户购买门票（需要后端签名授权）
- **调用方**: **前端** → 用户操作
- **参数**:
  - `event_id`: 活动ID
  - `type_id`: 票种ID
  - `ticket_uuid`: 门票UUID
  - `authorization_data`: 授权数据（包含价格、有效期、nonce等）
  - `backend_signature`: 后端签名（64字节）

**流程**:
1. 前端向后端请求购票授权
2. 后端生成 `authorization_data` 和签名
3. 前端调用此合约方法完成链上购票

---

### 4️⃣ 二级市场 (Marketplace)

#### `list_ticket`
- **作用**: 挂单转售门票
- **调用方**: **前端** → 用户操作
- **参数**: `resale_price` - 转售价格（USDC）

#### `buy_listed_ticket`
- **作用**: 购买挂单门票（需要后端签名授权）
- **调用方**: **前端** → 用户操作
- **参数**: 同 `purchase_ticket`（需要后端签名）

#### `cancel_listing`
- **作用**: 取消挂单
- **调用方**: **前端** → 用户操作

---

### 5️⃣ 检票流程 (Check-in)

#### `check_in_ticket`
- **作用**: 检票入场
- **调用方**: **前端/后端** → 检票员操作
- **参数**: `event_id` - 活动ID

---

## 前后端调用分工

### 前端调用（用户操作）
| 方法 | 场景 | 需要后端签名 |
|------|------|------------|
| `purchase_ticket` | 购买门票 | ✅ 是 |
| `list_ticket` | 挂单转售 | ❌ 否 |
| `buy_listed_ticket` | 购买二手票 | ✅ 是 |
| `cancel_listing` | 取消挂单 | ❌ 否 |
| `check_in_ticket` | 检票入场 | ❌ 否 |

### 后端调用（管理操作）
| 方法 | 场景 |
|------|------|
| `initialize_platform` | 部署初始化 |
| `update_platform_config` | 更新平台配置 |
| `toggle_pause` | 暂停/恢复平台 |
| `transfer_authority` | 转移权限 |
| `create_event` | 创建活动 |
| `update_event_status` | 更新活动状态 |
| `add_checkin_operator` | 添加检票员 |
| `remove_checkin_operator` | 移除检票员 |

---

## TypeScript 调用示例

### 安装依赖
```bash
npm install @coral-xyz/anchor @solana/web3.js
```

### 前端示例：购买门票

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TicketingProgram } from "../target/types/ticketing_program";

// 1. 初始化
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
const program = anchor.workspace.TicketingProgram as Program<TicketingProgram>;

// 2. 向后端请求购票授权
const authData = await fetch('/api/request-ticket-authorization', {
  method: 'POST',
  body: JSON.stringify({
    eventId: "event123",
    typeId: "vip",
    ticketUuid: "abc123...",
    buyer: provider.wallet.publicKey.toString()
  })
}).then(r => r.json());

// 3. 调用合约购票
const eventPda = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("event"), Buffer.from(eventId)],
  program.programId
)[0];

const ticketPda = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("ticket"), Buffer.from(eventId), Buffer.from(ticketUuid)],
  program.programId
)[0];

const tx = await program.methods
  .purchaseTicket(
    eventId,
    typeId,
    ticketUuid,
    authData.authorization_data,  // 后端提供
    Array.from(authData.backend_signature)  // 后端签名
  )
  .accounts({
    platformConfig: platformConfigPda,
    event: eventPda,
    ticket: ticketPda,
    nonceTracker: nonceTrackerPda,
    buyer: provider.wallet.publicKey,
    buyerUsdcAccount: buyerUsdcAta,
    platformUsdcAccount: platformUsdcAta,
    organizerUsdcAccount: organizerUsdcAta,
    usdcMint: USDC_MINT,
    // ...其他账户
  })
  .rpc();
```

### 前端示例：挂单转售

```typescript
const listingPda = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("listing"), ticketPda.toBuffer()],
  program.programId
)[0];

const tx = await program.methods
  .listTicket(new anchor.BN(50_000000))  // 50 USDC (6 decimals)
  .accounts({
    event: eventPda,
    ticket: ticketPda,
    listing: listingPda,
    seller: provider.wallet.publicKey,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .rpc();
```

### 前端示例：取消挂单

```typescript
const tx = await program.methods
  .cancelListing()
  .accounts({
    listing: listingPda,
    ticket: ticketPda,
    seller: provider.wallet.publicKey,
  })
  .rpc();
```

### 后端示例：创建活动

```typescript
const tx = await program.methods
  .createEvent(
    "event123",
    "ipfs://QmXxx...",
    new anchor.BN(startTime),
    new anchor.BN(endTime),
    new anchor.BN(ticketReleaseTime),
    new anchor.BN(stopSaleBefore),
    500,  // 5% 转售费率
    3     // 最多转售3次
  )
  .accounts({
    platformConfig: platformConfigPda,
    event: eventPda,
    organizer: organizerKeypair.publicKey,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .signers([organizerKeypair])
  .rpc();
```

---

## 重要数据结构

### AuthorizationData（授权数据）
```typescript
{
  buyer: PublicKey,           // 购买者地址
  ticketTypeId: string,       // 票种ID
  ticketUuid: string,         // 门票UUID
  maxPrice: u64,              // 最高价格
  validUntil: i64,            // 授权有效期（Unix时间戳）
  nonce: u64,                 // 防重放nonce
  ticketPda: PublicKey | null, // 二手票的PDA（购买二手票时需要）
  rowNumber: u16,             // 座位行号
  columnNumber: u16           // 座位列号
}
```

---

## PDA（Program Derived Address）推导

| 账户类型 | Seeds |
|----------|-------|
| `platform_config` | `["platform_config"]` |
| `nonce_tracker` | `["nonce_tracker"]` |
| `event` | `["event", event_id]` |
| `ticket` | `["ticket", event_id, ticket_uuid]` |
| `listing` | `["listing", ticket_pda]` |
| `checkin_authority` | `["checkin_auth", event_id, operator]` |

---

## 后端签名说明

详见 [`BACKEND_SIGNATURE.md`](./BACKEND_SIGNATURE.md)

**核心流程**:
1. 后端使用 Ed25519 私钥签名授权数据
2. 合约验证签名有效性
3. 防止重放攻击（nonce机制）

---

## 错误码

| Code | Error | 说明 |
|------|-------|------|
| 6000 | PlatformPaused | 平台已暂停 |
| 6001 | Unauthorized | 未授权访问 |
| 6003 | EventNotActive | 活动未激活 |
| 6004 | SalesNotStarted | 销售未开始 |
| 6005 | SalesEnded | 销售已结束 |
| 6006 | InvalidSignature | 签名无效 |
| 6007 | AuthorizationExpired | 授权已过期 |
| 6008 | NonceAlreadyUsed | Nonce已使用 |
| 6009 | PriceMismatch | 价格不匹配 |
| 6010 | AlreadyCheckedIn | 已检票 |
| 6011 | NotTicketOwner | 非票主 |
| 6012 | ResaleLimitReached | 达到转售上限 |
| 6014 | ListingNotActive | 挂单未激活 |

---

## 总结

**前端职责**: 用户交互操作（购票、挂单、检票）  
**后端职责**: 管理操作 + 提供购票授权签名  
**核心安全机制**: 后端签名授权 + Nonce防重放 + 链上状态验证

