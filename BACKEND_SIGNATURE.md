# Backend签名实现

## 后端签名（Node.js/TypeScript）

```typescript
import { Keypair, PublicKey } from "@solana/web3.js";
import nacl from "tweetnacl";

// Backend authority密钥（从环境变量或安全存储加载）
const backendAuthority = Keypair.fromSecretKey(
  Buffer.from(process.env.BACKEND_AUTHORITY_SECRET, 'base64')
);

// 签名授权数据
function signPurchaseAuthorization(
  buyer: PublicKey,
  ticketTypeId: string,
  ticketUuid: string,      // Backend-generated UUID (standard UUID format)
  maxPrice: bigint,
  validUntil: bigint,
  nonce: bigint,
  ticketPda?: PublicKey,  // For resale, specify the ticket being purchased
  rowNumber: number = 0,   // Seat row (0 for general admission)
  columnNumber: number = 0 // Seat column (0 for general admission)
): Uint8Array {
  // 序列化消息（必须与合约中的顺序完全一致）
  const parts: Buffer[] = [
    buyer.toBuffer(),                              // 32 bytes
    Buffer.from(ticketTypeId),                     // variable
    Buffer.from(ticketUuid),                       // variable (36 bytes for standard UUID)
    Buffer.from(new BigUint64Array([maxPrice]).buffer),   // 8 bytes LE
    Buffer.from(new BigInt64Array([validUntil]).buffer),  // 8 bytes LE  
    Buffer.from(new BigUint64Array([nonce]).buffer),      // 8 bytes LE
  ];
  
  // Optional ticket_pda (for resale)
  if (ticketPda) {
    parts.push(Buffer.from([1]));           // Option::Some
    parts.push(ticketPda.toBuffer());       // 32 bytes
  } else {
    parts.push(Buffer.from([0]));           // Option::None
  }
  
  // Seat information
  parts.push(Buffer.from(new Uint16Array([rowNumber]).buffer));    // 2 bytes LE
  parts.push(Buffer.from(new Uint16Array([columnNumber]).buffer)); // 2 bytes LE
  
  const message = Buffer.concat(parts);

  // Ed25519签名
  return nacl.sign.detached(message, backendAuthority.secretKey);
}

// API端点示例 - 首次购买
export async function POST_authorize_purchase(req, res) {
  const { buyer, ticketId } = req.body;  // 用户选择具体的票
  
  // 1. 从数据库获取票信息
  const ticket = await db.tickets.findOne({ 
    id: ticketId, 
    status: 'available' 
  });
  require(ticket, "Ticket not available");
  
  // 2. 业务逻辑验证
  //    - KYC检查
  //    - 黑名单
  //    - 限购规则
  //    - 余票验证（数据库）
  
  const nonce = BigInt(Date.now());
  const validUntil = BigInt(Math.floor(Date.now() / 1000) + 60); // 60秒有效
  
  // 重要：使用票的UUID（必须与数据库中存储的UUID一致）
  const ticketUuid = ticket.uuid;  // 数据库中预生成的标准UUID
  
  const signature = signPurchaseAuthorization(
    new PublicKey(buyer),
    ticket.ticketTypeId,
    ticketUuid,       // UUID用于链上PDA生成
    BigInt(ticket.price),
    validUntil,
    nonce,
    undefined,        // First-time purchase: no ticket_pda
    ticket.row,       // 座位行
    ticket.column     // 座位列
  );
  
  // 3. 存储nonce和ticket关联 (10分钟TTL)
  await redis.setex(`nonce:${nonce}`, 600, JSON.stringify({
    ticketId: ticket.id,
    ticketUuid,
    buyer,
  }));
  
  // 4. 标记票为pending
  await db.tickets.update(ticketId, { status: 'pending', buyer });
  
  res.json({
    buyer,
    ticketTypeId: ticket.ticketTypeId,
    ticketUuid,       // 前端需要传给合约
    maxPrice: ticket.price.toString(),
    validUntil: validUntil.toString(),
    nonce: nonce.toString(),
    ticketPda: null,
    rowNumber: ticket.row,
    columnNumber: ticket.column,
    signature: Array.from(signature),
    backendAuthority: backendAuthority.publicKey.toString(),
  });
}

// API端点示例 - 二手票购买
export async function POST_authorize_resale(req, res) {
  const { buyer, ticketPda } = req.body;
  
  // 1. 从链上获取listing和ticket信息
  const listing = await fetchListingFromChain(ticketPda);
  require(listing.isActive, "Listing not active");
  
  const ticket = await fetchTicketFromChain(ticketPda);
  
  // 2. 业务逻辑验证
  const nonce = BigInt(Date.now());
  const validUntil = BigInt(Math.floor(Date.now() / 1000) + 60);
  
  // 二手票：ticketUuid可以传空字符串（不会被使用）
  const signature = signPurchaseAuthorization(
    new PublicKey(buyer),
    ticket.ticketTypeId,
    "",                        // Resale: UUID not used (empty string)
    BigInt(listing.price),
    validUntil,
    nonce,
    new PublicKey(ticketPda),  // Resale: specify ticket PDA
    ticket.rowNumber,          // Keep original seat
    ticket.columnNumber
  );
  
  await redis.setex(`nonce:${nonce}`, 600, '1');
  
  res.json({
    buyer,
    ticketTypeId: ticket.ticketTypeId,
    ticketUuid: "",            // Empty for resale
    maxPrice: listing.price.toString(),
    validUntil: validUntil.toString(),
    nonce: nonce.toString(),
    ticketPda: ticketPda,      // Key difference for resale
    rowNumber: ticket.rowNumber,
    columnNumber: ticket.columnNumber,
    signature: Array.from(signature),
    backendAuthority: backendAuthority.publicKey.toString(),
  });
}
```

## 后端监听购票事件

```typescript
// 监听链上购票成功事件
program.addEventListener('purchaseTicketEvent', async (event) => {
  const { buyer, eventId, ticketUuid, rowNumber, columnNumber, ticketPda } = event;
  
  // 通过UUID直接关联数据库中的票
  const dbTicket = await db.tickets.findOne({
    uuid: ticketUuid,
    status: 'pending'
  });
  
  if (dbTicket) {
    // 更新数据库
    await db.tickets.update(dbTicket.id, {
      status: 'sold',
      ticketPda: ticketPda.toString(),
      soldAt: new Date(),
      owner: buyer.toString(),
    });
    
    console.log(`Ticket ${dbTicket.id} sold: UUID ${ticketUuid}`);
  }
});

// 或者通过nonce查询（如果Redis存了ticketId）
const nonceData = await redis.get(`nonce:${event.nonce}`);
if (nonceData) {
  const { ticketId, ticketUuid } = JSON.parse(nonceData);
  await db.tickets.update(ticketId, { 
    status: 'sold',
    ticketPda: event.ticketPda.toString(),
    soldAt: new Date(),
  });
}

// 防重复mint机制说明：
// 1. 每个ticket在数据库中有唯一UUID
// 2. 链上使用UUID生成PDA: [b"ticket", event_id, uuid]
// 3. 相同UUID会生成相同PDA地址
// 4. Anchor的 #[account(init)] 约束会检查账户是否已存在
// 5. 如果UUID重复，交易自动失败，防止双重mint
```

## 前端调用

```typescript
// 1. 请求授权
const auth = await fetch('/api/authorize-purchase', {
  method: 'POST',
  body: JSON.stringify({
    buyer: wallet.publicKey.toString(),
    ticketTypeId: 'vip',
    maxPrice: 50_000_000, // 50 USDC
  }),
}).then(r => r.json());

// 2. 调用合约
await program.methods
  .purchaseTicket(
    eventId,
    ticketTypeId,
    auth.ticketUuid,  // 新增：UUID参数
    {
      buyer: new PublicKey(auth.buyer),
      ticketTypeId: auth.ticketTypeId,
      ticketUuid: auth.ticketUuid,  // 新增：UUID字段
      maxPrice: new BN(auth.maxPrice),
      validUntil: new BN(auth.validUntil),
      nonce: new BN(auth.nonce),
      ticketPda: null,  // 首次购买为null
      rowNumber: auth.rowNumber,
      columnNumber: auth.columnNumber,
    },
    auth.signature
  )
  .accounts({...})
  .rpc();
```

## 当前合约状态

⚠️ **合约中的签名验证当前为Mock实现**

要启用真实验证，在 `Cargo.toml` 添加：
```toml
ed25519-dalek = "1.0.1"
```

并替换 `purchase.rs` 中的 `verify_backend_signature` 函数为生产版本（参考项目中的注释）。

## 测试时

测试脚本中使用 `new Array(64).fill(0)` 作为mock签名，因为验证暂未实现。

