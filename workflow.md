好，基于最新代码重新说一遍完整流程：

## 1. 平台初始化（一次性）

**合约**：`initialize_platform`
- 部署者设置：平台费用、backend_authority、event_admin
- event_admin是唯一能创建event的账户

## 2. Admin创建活动

**前端**：平台管理员（event_admin）填表单
**合约**：调用`create_event`
- 传入：event_id、活动元数据、**organizer地址**（组织者钱包）
- 只有platform_config.event_admin能调用
- 创建EventAccount，里面存着organizer地址

## 3. 后端生成票据

**后端**：
- 数据库创建tickets表：`{uuid(36位), event_id, ticket_type_id, price, row, column, status: 'available'}`
- 票类型完全在数据库（VIP、普通座等）
- 余票、价格全在数据库，**链上不存**

## 4. 用户购票

**前端**：用户选座位，点购买
**后端**：
- KYC、黑名单、限购验证
- 检查数据库余票
- 锁定票（status: 'pending'）
- **生成授权**：
```typescript
signAuthorization(
  buyer_pubkey,
  ticket_type_id,  // 只是个标识符，合约不验证
  ticket_uuid,     // 关键！决定链上PDA
  price,           // 从数据库取，可随时变
  valid_until: now + 60秒,
  nonce,
  row, column
)
```
- 返回给前端

**前端**：调用合约
```typescript
purchaseTicket(event_id, ticket_type_id, ticket_uuid, authData, signature)
```

**合约**：
- 验证backend签名
- 验证nonce、授权时间
- **验证organizer_usdc_account是event.organizer的ATA**（新增！）
- 用UUID生成PDA：`[b"ticket", event_id, uuid]`
- 如果UUID重复，PDA已存在→失败
- 转账：
  - 平台费→platform_usdc_account
  - 票款→**event.organizer的ATA**（强制！）
- 创建TicketAccount NFT
- 标记nonce已用

**后端监听**：链上事件→更新数据库`{status: 'sold', ticket_pda, owner}`

## 5. 二手市场

**卖家挂单**：
- 前端调用`list_ticket(price)`
- 合约把票ownership转给program_authority，创建Listing

**买家购买**：
- 后端授权（`ticket_pda`有值，`ticket_uuid`传空字符串）
- 前端调用`buy_listed_ticket`
- 合约：
  - 验证签名
  - **验证organizer_usdc_account是event.organizer的ATA**
  - 分账：
    - 平台费→平台
    - 组织者转售费→**event.organizer的ATA**
    - 剩余→卖家
  - 更新ownership
  - PoF积分调整

## 6. 核销

**现场**：用户出示ticket_pda二维码

**前端/核销设备**：
```typescript
checkInTicket(event_id)
  .accounts({ ticket: ticket_pda, operator: 核销员 })
```

**合约**：
- 验证operator是这个event的CheckInAuthority
- 验证时间窗口（活动前1小时到结束）
- 检查未核销过
- 标记`is_checked_in = true`
- PoF +100积分

---

## 关键变化点

1. **分账安全**：组织者USDC账户必须是`event.organizer`推导的ATA，无法被篡改
2. **无TicketType**：链上不再有票类型注册表，完全后端管理
3. **完全后端控制**：
   - 余票数量：数据库
   - 票价：数据库（随时改）
   - 票类型：数据库（不验证）
   - UUID：数据库生成
4. **链上只管**：验证签名、mint NFT、强制分账给正确地址
5. **防双花**：UUID→PDA唯一性自动防止

整个系统的核心就是：**后端授权谁能mint哪张票（UUID）、多少钱，链上负责验证签名、mint NFT、把钱强制给event.organizer**。

更改核销人员：
修改内容：
AddCheckInOperator账户结构：
新增：platform_config账户
约束改为：admin.key() == platform_config.event_admin
参数名：organizer → admin（更清晰）
RemoveCheckInOperator账户结构：
新增：platform_config账户
约束改为：admin.key() == platform_config.event_admin
参数名：organizer → admin
现在的流程：
后端接收前端请求 → 后端调用合约：
关键点：
前端调用后端API
后端验证权限后，用admin钱包调用合约
只有拥有event_admin私钥的后端能成功调用
组织者无权添加/删除核销员