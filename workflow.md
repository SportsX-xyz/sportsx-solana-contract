# Web3票务系统 - 整体设计文档

## 一、合约架构

### 单一Program架构
```
TicketingProgram
├── PlatformConfig (全局配置)
├── EventAccount (活动)
├── TicketTypeAccount (票种)
├── TicketAccount (票务)
├── ListingAccount (转售挂单)
├── CheckInAuthority (核销权限)
└── NonceTracker (授权nonce追踪)
```

---

## 二、账户结构

### 1. PlatformConfig
```rust
PDA: ["platform_config"]

字段:
- fee_receiver: Pubkey                 // 平台费接收地址
- fee_amount_usdc: u64                 // 平台费(USDC, 6 decimals) 0.1 USDC = 100000
- update_authority: Pubkey             // 多签地址
- backend_authority: Pubkey            // 后端签名公钥
- is_paused: bool
- bump: u8
```

### 2. EventAccount
```rust
PDA: ["event", event_id]

字段:
- event_id: String
- organizer: Pubkey
- metadata_uri: String                 // IPFS
- start_time: i64
- end_time: i64
- ticket_release_time: i64
- stop_sale_before: i64
- resale_fee_rate: u16                 // basis points (100 = 1%)
- max_resale_times: u8
- status: u8                           // 0=Draft, 1=Active, 2=Disabled
- bump: u8
```

### 3. TicketTypeAccount
```rust
PDA: ["ticket_type", event_id, type_id]

字段:
- event_id: String
- type_id: String
- tier_name: String
- price: u64                           // USDC (6 decimals)
- total_supply: u32
- minted: u32
- color: u32
- bump: u8
```

### 4. TicketAccount
```rust
PDA: ["ticket", event_id, sequence_number]

字段:
- event_id: String
- ticket_type_id: String
- sequence_number: u32
- owner: Pubkey
- original_owner: Pubkey
- resale_count: u8
- is_checked_in: bool
- row_number: u16
- column_number: u16
- bump: u8
```

### 5. ListingAccount
```rust
PDA: ["listing", ticket_pda]

字段:
- ticket_pda: Pubkey
- seller: Pubkey
- price: u64                           // USDC
- listed_at: i64
- is_active: bool
- bump: u8
```

### 6. CheckInAuthority
```rust
PDA: ["checkin_auth", event_id, operator]

字段:
- event_id: String
- operator: Pubkey
- is_active: bool
- bump: u8
```

### 7. NonceTracker
```rust
PDA: ["nonce_tracker"]

字段:
- used_nonces: Vec<u64>                // bitmap优化
```

---

## 三、合约方法

### 平台管理 (调用者: 多签/部署者)

#### 1. initialize_platform
```rust
参数:
- initial_fee_receiver: Pubkey
- initial_fee_usdc: u64
- backend_authority: Pubkey

权限: 部署者
执行: 仅一次
```

#### 2. update_platform_config
```rust
参数:
- new_fee_receiver: Option<Pubkey>
- new_fee_usdc: Option<u64>
- new_backend_authority: Option<Pubkey>

权限: update_authority (多签)
```

#### 3. toggle_pause
```rust
权限: update_authority
```

---

### 活动管理 (调用者: 项目方)

#### 4. create_event
```rust
参数:
- event_id: String
- metadata_uri: String
- start_time: i64
- end_time: i64
- ticket_release_time: i64
- stop_sale_before: i64
- resale_fee_rate: u16
- max_resale_times: u8

权限: 项目方签名
创建: EventAccount
```

#### 5. update_event_status
```rust
参数:
- event_id: String
- new_status: u8

权限: event.organizer
```

#### 6. add_checkin_operator
```rust
参数:
- event_id: String
- operator: Pubkey

权限: event.organizer
创建: CheckInAuthority
```

#### 7. remove_checkin_operator
```rust
参数:
- event_id: String
- operator: Pubkey

权限: event.organizer
标记: CheckInAuthority.is_active = false
```

---

### 票务管理 (调用者: 项目方)

#### 8. create_ticket_type
```rust
参数:
- event_id: String
- type_id: String
- tier_name: String
- price: u64
- total_supply: u32
- color: u32

权限: event.organizer
创建: TicketTypeAccount
```

#### 9. batch_mint_tickets
```rust
参数:
- event_id: String
- type_id: String
- quantity: u32

权限: event.organizer
操作: ticket_type.total_supply += quantity
注意: Lazy Minting - 不创建实际TicketAccount
```

---

### 购票流程 (调用者: 用户)

#### 10. purchase_ticket
```rust
参数:
- event_id: String
- type_id: String
- authorization_data: {
    buyer: Pubkey,
    ticket_type_id: String,
    max_price: u64,
    valid_until: i64,
    nonce: u64
  }
- backend_signature: [u8; 64]

权限: 任何人 + 后端签名验证

账户:
- platform_config (读)
- event (读)
- ticket_type (读写)
- ticket (创建)
- buyer (签名, USDC扣款)
- platform_fee_receiver (USDC收款)
- organizer (USDC收款)
- buyer_usdc_account (mut)
- platform_usdc_account (mut)
- organizer_usdc_account (mut)
- token_program
- pof_program

逻辑:
1. 验证backend_signature
2. 检查valid_until > now
3. 检查nonce未使用
4. 检查库存: ticket_type.minted < total_supply
5. USDC转账:
   - platform: 0.1 USDC
   - organizer: ticket_type.price - 0.1 USDC
6. 更新ticket_type.minted += 1
7. 创建TicketAccount:
   - sequence_number = ticket_type.minted
   - owner = buyer
8. 标记nonce已使用
9. CPI调用PoF: update_score(+积分)
   - 积分 = min(50, floor(price_usdc / 10))

Gas: ~0.005 SOL
```

---

### 二级市场 (调用者: 用户)

#### 11. list_ticket
```rust
参数:
- ticket_pda: Pubkey
- resale_price: u64

权限: ticket.owner

检查:
- ticket.resale_count < event.max_resale_times
- !ticket.is_checked_in
- now < event.start_time

创建: ListingAccount
```

#### 12. buy_listed_ticket
```rust
参数:
- listing_pda: Pubkey

权限: 任何人

账户:
- listing (读)
- ticket (更新owner)
- event (读)
- buyer (签名, USDC扣款)
- seller (USDC收款)
- platform_fee_receiver (USDC收款)
- organizer (USDC收款)
- [各方USDC账户]

逻辑:
1. 验证listing.is_active
2. USDC转账:
   - platform: 0.1 USDC
   - organizer: resale_price * resale_fee_rate
   - seller: 剩余
3. 更新ticket:
   - owner = buyer
   - resale_count += 1
4. 关闭ListingAccount (退还rent给seller)
5. CPI调用PoF:
   - 卖家: update_score(-原积分)
   - 买家: update_score(+新积分)
```

#### 13. cancel_listing
```rust
参数:
- listing_pda: Pubkey

权限: listing.seller
操作: 关闭ListingAccount
```

---

### 核销流程 (调用者: 核销员)

#### 14. check_in_ticket
```rust
参数:
- event_id: String
- ticket_pda: Pubkey

权限: CheckInAuthority中的operator

账户:
- checkin_authority (验证)
- event (读)
- ticket (更新)
- operator (签名)
- pof_program

逻辑:
1. 验证operator在CheckInAuthority中
2. 检查!ticket.is_checked_in
3. 检查时间: event.start_time - check_in_before < now < event.end_time
4. 更新ticket.is_checked_in = true
5. CPI调用PoF: update_score(+100)
```

---
关于多签：
初始化用部署者，之后转移（灵活）
rust// 1. 初始化时用部署者
pub fn initialize_platform(ctx: Context<Initialize>) -> Result<()> {
    let config = &mut ctx.accounts.platform_config;
    config.update_authority = ctx.accounts.deployer.key();  // 暂时用部署者
    // ...
}

// 2. 后续转移权限给多签
pub fn transfer_authority(
    ctx: Context<TransferAuthority>,
    new_authority: Pubkey,
) -> Result<()> {
    let config = &mut ctx.accounts.platform_config;
    
    require!(
        ctx.accounts.current_authority.key() == config.update_authority,
        ErrorCode::Unauthorized
    );
    
    config.update_authority = new_authority;
    msg!("权限已转移至: {}", new_authority);
    Ok(())
}

#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    #[account(mut)]
    pub platform_config: Account<'info, PlatformConfig>,
    
    pub current_authority: Signer<'info>,  // 当前权限持有者签名
}
```
**流程**:
1. 部署合约 → update_authority = deployer
2. 创建多签钱包 (Squads/Realms)
3. 调用transfer_authority → update_authority = multisig
4. 部署者失去控制权
```


## 四、后端API职责

### 1. 活动管理
```
POST /api/events
- 存储EventData到数据库
- 生成metadata上传IPFS
- 调用合约create_event

POST /api/events/{id}/ticket-types
- 存储TicketType到数据库
- 调用合约create_ticket_type
- 调用合约batch_mint_tickets
```

### 2. 授权签名生成
```
POST /api/request-purchase-auth
请求: {
  event_id,
  ticket_type_id,
  buyer_pubkey,
  wallet_signature
}

逻辑:
1. 验证wallet_signature
2. 检查数据库库存
3. 检查用户购买历史
4. 锁定库存(临时)
5. 生成授权签名:
   message = {
     buyer,
     ticket_type_id,
     max_price: current_price,
     valid_until: now + 300,
     nonce: random()
   }
   signature = sign(message, backend_private_key)

响应: {
  authorization_data: message,
  signature
}
```

### 3. 链上事件监听
```
监听purchase_ticket事件:
- 释放库存锁
- 更新数据库sold_tickets
- 发送确认邮件

监听check_in事件:
- 更新票状态
- 记录核销时间
```

---

## 五、前端调用流程

### 购票流程
```typescript
// 1. 用户选择票种
const ticketType = await fetch(`/api/events/${eventId}/ticket-types`);

// 2. 请求购买授权
const authRequest = await fetch('/api/request-purchase-auth', {
  method: 'POST',
  body: JSON.stringify({
    event_id: eventId,
    ticket_type_id: typeId,
    buyer_pubkey: wallet.publicKey.toString(),
    wallet_signature: await wallet.signMessage(...)
  })
});

const { authorization_data, signature } = await authRequest.json();

// 3. 调用合约
const tx = await program.methods
  .purchaseTicket(
    eventId,
    typeId,
    authorization_data,
    signature
  )
  .accounts({
    platformConfig: configPDA,
    event: eventPDA,
    ticketType: ticketTypePDA,
    ticket: ticketPDA,
    buyer: wallet.publicKey,
    platformFeeReceiver: configData.fee_receiver,
    organizer: eventData.organizer,
    buyerUsdcAccount: buyerUSDC,
    platformUsdcAccount: platformUSDC,
    organizerUsdcAccount: organizerUSDC,
    tokenProgram: TOKEN_PROGRAM_ID,
    pofProgram: POF_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### 转售流程
```typescript
// 1. 挂单
await program.methods
  .listTicket(resalePrice)
  .accounts({
    listing: listingPDA,
    ticket: ticketPDA,
    event: eventPDA,
    seller: wallet.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();

// 2. 购买
await program.methods
  .buyListedTicket()
  .accounts({
    listing: listingPDA,
    ticket: ticketPDA,
    event: eventPDA,
    buyer: wallet.publicKey,
    seller: sellerPubkey,
    platformFeeReceiver: ...,
    organizer: ...,
    // [各方USDC账户]
    tokenProgram: TOKEN_PROGRAM_ID,
    pofProgram: POF_PROGRAM_ID,
  })
  .rpc();
```

### 核销流程
```typescript
// 扫描二维码获取ticket_pda
const ticketPDA = parseQRCode(qrData);

await program.methods
  .checkInTicket(eventId, ticketPDA)
  .accounts({
    checkinAuthority: authorityPDA,
    event: eventPDA,
    ticket: ticketPDA,
    operator: wallet.publicKey,
    pofProgram: POF_PROGRAM_ID,
  })
  .rpc();
```

---

## 六、USDC支付集成

### Token账户准备
```
每个参与方需要关联的USDC账户(ATA):
- Buyer: getAssociatedTokenAddress(USDC_MINT, buyer)
- Platform: getAssociatedTokenAddress(USDC_MINT, platform_fee_receiver)
- Organizer: getAssociatedTokenAddress(USDC_MINT, organizer)
- Seller (转售时): getAssociatedTokenAddress(USDC_MINT, seller)
```

### SPL Token转账
```rust
// 合约内使用anchor_spl::token::transfer

use anchor_spl::token::{self, Token, TokenAccount, Transfer};

// 平台费
token::transfer(
    CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.buyer_usdc_account.to_account_info(),
            to: ctx.accounts.platform_usdc_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        }
    ),
    platform_fee_amount
)?;

// 项目方
token::transfer(
    CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.buyer_usdc_account.to_account_info(),
            to: ctx.accounts.organizer_usdc_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        }
    ),
    organizer_amount
)?;
```

---

## 七、成本估算

### 项目方成本 (1000张票活动)
```
- create_event: 0.002 SOL
- create_ticket_type (3个): 0.003 SOL
- batch_mint_tickets (3次): 0.003 SOL
- add_checkin_operator (5个): 0.005 SOL
总计: ~0.013 SOL (~$2.6)
```

### 用户成本
```
- purchase_ticket: 0.005 SOL + ticket_price_USDC
- list_ticket: 0.002 SOL
- buy_listed_ticket: 0.003 SOL + resale_price_USDC
```

---

## 八、安全机制

### 1. 授权机制
- 后端签名验证(Ed25519)
- Nonce防重放
- 5分钟时效性

### 2. 权限控制
- 多签控制平台配置
- 项目方控制Event
- 核销员白名单

### 3. 并发安全
- TicketType.minted原子更新
- Nonce唯一性检查

### 4. 金额验证
- max_price上限保护
- 检查USDC账户余额
- checked_arithmetic防溢出

---

## 九、PDA推导汇总

```
platform_config:    ["platform_config"]
event:              ["event", event_id]
ticket_type:        ["ticket_type", event_id, type_id]
ticket:             ["ticket", event_id, sequence_number]
listing:            ["listing", ticket_pda]
checkin_authority:  ["checkin_auth", event_id, operator]
nonce_tracker:      ["nonce_tracker"]
```

---

## 十、PoF积分规则

```
购票: min(50, floor(price_usdc / 10))
核销: +100
转售 (卖家): -原积分
转售 (买家): +新积分
```