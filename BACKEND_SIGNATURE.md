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
  
  const nonce = BigInt(Date.now());
  const validUntil = BigInt(Math.floor(Date.now() / 1000) + 60); // 60秒有效
  
  const signature = signPurchaseAuthorization(
    new PublicKey(buyer),
    ticket.ticketTypeId,
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
    buyer,
  }));
  
  // 4. 标记票为pending
  await db.tickets.update(ticketId, { status: 'pending', buyer });
  
  res.json({
    buyer,
    ticketTypeId: ticket.ticketTypeId,
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
  
  const signature = signPurchaseAuthorization(
    new PublicKey(buyer),
    ticket.ticketTypeId,
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
    maxPrice: listing.price.toString(),
    validUntil: validUntil.toString(),
    nonce: nonce.toString(),
    ticketPda: ticketPda,  // Key difference for resale
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
  const { buyer, eventId, sequenceNumber, rowNumber, columnNumber, ticketPda } = event;
  
  // 通过row+column关联数据库中的票
  const dbTicket = await db.tickets.findOne({
    eventId,
    row: rowNumber,
    column: columnNumber,
    status: 'pending'
  });
  
  if (dbTicket) {
    // 更新数据库
    await db.tickets.update(dbTicket.id, {
      status: 'sold',
      ticketPda: ticketPda.toString(),
      sequenceNumber,
      soldAt: new Date(),
      owner: buyer.toString(),
    });
    
    console.log(`Ticket ${dbTicket.id} sold: ${rowNumber}排${columnNumber}座`);
  }
});

// 或者通过nonce查询（如果Redis存了ticketId）
const nonceData = await redis.get(`nonce:${event.nonce}`);
if (nonceData) {
  const { ticketId } = JSON.parse(nonceData);
  await db.tickets.update(ticketId, { ... });
}
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
    {
      buyer: new PublicKey(auth.buyer),
      ticketTypeId: auth.ticketTypeId,
      maxPrice: new BN(auth.maxPrice),
      validUntil: new BN(auth.validUntil),
      nonce: new BN(auth.nonce),
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

