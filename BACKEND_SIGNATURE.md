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
  nonce: bigint
): Uint8Array {
  // 序列化消息（必须与合约中的顺序完全一致）
  const message = Buffer.concat([
    buyer.toBuffer(),                              // 32 bytes
    Buffer.from(ticketTypeId),                     // variable
    Buffer.from(new BigUint64Array([maxPrice]).buffer),   // 8 bytes LE
    Buffer.from(new BigInt64Array([validUntil]).buffer),  // 8 bytes LE  
    Buffer.from(new BigUint64Array([nonce]).buffer),      // 8 bytes LE
  ]);

  // Ed25519签名
  return nacl.sign.detached(message, backendAuthority.secretKey);
}

// API端点示例
export async function POST_authorize_purchase(req, res) {
  const { buyer, ticketTypeId, maxPrice } = req.body;
  
  // 1. 业务逻辑验证
  //    - KYC检查
  //    - 黑名单
  //    - 限购规则
  //    - 票务可用性
  
  const nonce = BigInt(Date.now());
  const validUntil = BigInt(Math.floor(Date.now() / 1000) + 300); // 5分钟有效
  
  const signature = signPurchaseAuthorization(
    new PublicKey(buyer),
    ticketTypeId,
    BigInt(maxPrice),
    validUntil,
    nonce
  );
  
  // 2. 存储nonce防重放
  await redis.setex(`nonce:${nonce}`, 600, '1');
  
  res.json({
    buyer,
    ticketTypeId,
    maxPrice: maxPrice.toString(),
    validUntil: validUntil.toString(),
    nonce: nonce.toString(),
    signature: Array.from(signature),
    backendAuthority: backendAuthority.publicKey.toString(),
  });
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

