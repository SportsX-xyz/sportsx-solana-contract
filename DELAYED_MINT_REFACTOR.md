# 延迟Mint重构完成

## 概述

已完成从链上余票管理到延迟mint模式的重构。所有票务信息（价格、余票、UUID）现由后端管理，链上仅在mint时验证后端授权。

## 主要变更

### 1. State结构

**TicketTypeAccount** (简化为注册表)
- ❌ 移除: `price`, `total_supply`, `minted`, `color`
- ✅ 保留: `event_id`, `type_id`, `tier_name`, `bump`

**TicketAccount** (UUID替代序列号)
- ❌ 移除: `sequence_number: u32`
- ✅ 新增: `ticket_uuid: String` (36字符标准UUID)

### 2. Purchase指令

**AuthorizationData**
- ✅ 新增: `ticket_uuid: String` 字段
- 后端通过UUID关联数据库票据

**PurchaseTicket**
- PDA生成: `[b"ticket", event_id, uuid]` (原: `[b"ticket", event_id, sequence]`)
- 参数: 新增 `ticket_uuid: String`
- ❌ 移除: 链上余票检查 `has_available_supply()`
- ❌ 移除: 更新 `ticket_type.minted`
- ✅ 票价从 `authorization_data.max_price` 获取（而非链上存储）

**签名序列化** (verify_backend_signature)
```rust
// 新增UUID到消息序列化
message.extend_from_slice(authorization_data.ticket_uuid.as_bytes());
```

### 3. Ticket Management

**create_ticket_type**
- ❌ 移除参数: `price`, `total_supply`, `color`
- 仅创建票类型注册表

**batch_mint_tickets**
- ❌ 完全删除（不再需要）

### 4. 错误码

- ❌ 删除: `InsufficientSupply` (不再验证链上余票)

### 5. 文档更新

**BACKEND_SIGNATURE.md**
- ✅ 签名函数添加 `ticketUuid` 参数
- ✅ API示例展示UUID生成和验证逻辑
- ✅ 说明防重复mint机制（PDA唯一性）
- ✅ 更新消息序列化格式

### 6. 测试更新

**tests/ticketing.ts**
- ✅ 安装 `uuid` 包
- ✅ 使用 `uuidv4()` 生成UUID
- ✅ 更新所有 `purchaseTicket` 调用
- ✅ PDA推导使用UUID而非序列号
- ❌ 删除批量mint测试

## 关键机制

### 防重复Mint
1. 每个ticket在数据库中有唯一UUID
2. 链上使用UUID生成PDA: `[b"ticket", event_id, uuid]`
3. 相同UUID生成相同PDA地址
4. Anchor `#[account(init)]` 约束检查账户是否已存在
5. UUID重复时交易自动失败

### 二手票购买
- AuthorizationData中 `ticket_uuid` 设为空字符串
- 通过 `ticket_pda` 字段识别二手票交易

## 编译状态

✅ 编译成功
- 合约: `anchor build` 通过
- 测试: 更新完成，等待测试运行
- 堆栈警告: 存在但不影响功能（之前就有）

## 后续步骤

1. 运行测试验证功能
2. 部署到devnet测试
3. 后端实施UUID管理和余票控制
4. 更新前端集成

---
**完成时间**: $(date)
**分支**: pluto

