# 改动总结

## 核心改动

### 1. Event创建权限控制
- PlatformConfig新增`event_admin`字段
- 只有event_admin可以创建event
- 测试中deployer作为event_admin

### 2. 二级市场"回票池"机制
- **挂单**：票所有权转给`program_authority` PDA
- **购买**：需要后端签名授权（与首次购买统一）
- **取消**：票所有权归还给original_seller

### 3. 统一购买流程
- 首次购买：`ticketPda = null`
- 二次购买：`ticketPda = Some(ticket_address)`
- 后端签名包含`ticket_pda`字段

### 4. NonceTracker优化
- 改用循环Buffer：500组 (nonce + buyer + timestamp)
- 自动过期：10分钟后自动淘汰
- 授权有效期：60秒
- 防重放：nonce + buyer绑定，防timestamp冲突

## 数据同步保证

### Event
- ✅ 只能通过event_admin创建
- ✅ 后端控制创建权限

### Ticket
- ✅ 首次购买需后端签名
- ✅ 二次购买需后端签名  
- ✅ 后端完全控制购票流程

### Listing
- ⚠️ 挂单无需后端签名（用户直接调用）
- ✅ 购买listing需后端签名
- ✅ 取消listing无需签名（用户自己取消）

### Check-in
- ⚠️ 无后端签名（直接链上调用）
- ✅ 需要checkin_operator权限
- 建议：后端监听check-in事件

## 安全改进

### 防重放攻击
- Nonce + Buyer绑定（防timestamp重复）
- 10分钟过期窗口（覆盖60秒授权）
- 循环Buffer（无容量限制）

### 防挂单+核销
- 挂单时票转给program_authority
- 核销要求票owner为用户
- 挂单的票无法核销 ✅

### 防授权混用
- 首次购买：ticket_pda必须为None
- 二次购买：ticket_pda必须为Some(specific_ticket)
- 防止授权跨类型使用

## 成本

### NonceTracker
```
大小: 24KB
- nonces[500]:     4KB
- buyers[500]:    16KB
- timestamps[500]: 4KB
- next_index:       2B

Rent: ~0.17 SOL ($17 @ $100/SOL)
一次性，永久有效
```

### PlatformConfig
```
增加: +32 bytes (event_admin)
总大小: 138 bytes
Rent变化: 可忽略
```

## API变化

### initialize_platform
```diff
- initialize_platform(fee_receiver, fee_usdc, backend_auth)
+ initialize_platform(fee_receiver, fee_usdc, backend_auth, event_admin)
```

### update_platform_config
```diff
- update_platform_config(fee_receiver?, fee_usdc?, backend_auth?)
+ update_platform_config(fee_receiver?, fee_usdc?, backend_auth?, event_admin?)
```

### buy_listed_ticket
```diff
- buy_listed_ticket()
+ buy_listed_ticket(authorization_data, signature)
```

### cancel_listing
```diff
- cancel_listing()  // 只需listing账户
+ cancel_listing()  // 需要listing + ticket账户
```

## 测试更新

- ✅ 所有authData添加`ticketPda`字段
- ✅ deployer作为event_admin创建event
- ✅ buy_listed_ticket添加授权参数
- ✅ cancel_listing添加ticket账户
- ✅ 新增测试：挂单票无法核销
- ✅ 验证票所有权转移（挂单→program，取消→seller）

---

## 现在讨论Check-in

### 当前设计
```rust
check_in_ticket(event_id) {
  验证: operator有权限
  执行: ticket.is_checked_in = true
  无需: 后端签名
}
```

### 你的担忧
> "不会有人单独调用合约或绕过平台做核销吧？"

### 潜在风险

**场景1：内部作弊**
```
恶意operator（项目方内部人员）:
- 私下收钱
- 直接调合约给朋友核销
- 平台不知道，无法统计
```

**场景2：数据不同步**
```
Operator钱包直接调合约 → 核销成功
后端不知道 → 数据库显示未核销
前端显示错误
```

**场景3：应急核销**
```
优点：前端/后端故障时仍可核销
缺点：绕过监控和统计
```

### 三种方案对比

**方案A：保持现状（推荐）**
- ✅ 核销快（无网络延迟）
- ✅ 离线可用（后端故障不影响）
- ✅ 去中心化
- ⚠️ 后端监听事件同步数据
- 风险：数据可能短暂不同步

**方案B：添加后端签名**
```rust
check_in_ticket(event_id, auth_data, signature) {
  验证后端签名
  验证operator权限
  执行核销
}
```
- ✅ 后端完全掌控
- ❌ 依赖后端可用性
- ❌ 核销变慢
- ❌ 违背去中心化

**方案C：双模式**
```rust
check_in_ticket_normal(需签名)
check_in_ticket_emergency(仅operator，记录emergency标记)
```
- ✅ 平衡安全和可用性
- ✅ 应急场景可用
- ⚠️ 复杂度增加

### 我的建议

**保持现状 + 后端监听**

理由：
1. **核销场景特殊**：现场操作，需要快速
2. **风险可控**：operator由organizer指定，信任模型成立
3. **数据最终一致**：后端监听事件，延迟1-2秒同步
4. **应急可用**：网络故障时仍能核销

**额外保护措施**：
```typescript
// 后端监听check-in事件
program.addEventListener('checkInTicketEvent', (event) => {
  database.updateTicketStatus(event.ticket, 'checked_in');
  analytics.recordCheckIn(event);
});

// 定期对账
cron.daily(() => {
  syncCheckInStatusFromChain();
});
```

你觉得呢？需要添加后端签名还是保持现状？