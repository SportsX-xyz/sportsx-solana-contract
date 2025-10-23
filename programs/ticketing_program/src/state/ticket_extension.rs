use anchor_lang::prelude::*;

/// Token 2022 扩展字段数据结构
/// 用于在 NFT 中存储票据的扩展信息
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct TicketExtensionData {
    /// 检票状态
    pub check_in_status: bool,
    
    /// 检票时间 (Unix timestamp)
    pub check_in_time: i64,
    
    /// 检票操作员
    pub check_in_operator: Pubkey,
    
    /// 座位行号
    pub seat_row: u16,
    
    /// 座位列号
    pub seat_column: u16,
    
    /// 事件 ID (最大 32 字节)
    pub event_id: String,
    
    /// 票据 UUID (最大 32 字节)
    pub ticket_uuid: String,
    
    /// 原始价格 (用于 PoF 计算)
    pub original_price: u64,
}

impl TicketExtensionData {
    /// 创建新的扩展字段数据
    pub fn new(
        seat_row: u16,
        seat_column: u16,
        event_id: String,
        ticket_uuid: String,
        original_price: u64,
    ) -> Self {
        Self {
            check_in_status: false,
            check_in_time: 0,
            check_in_operator: Pubkey::default(),
            seat_row,
            seat_column,
            event_id,
            ticket_uuid,
            original_price,
        }
    }
    
    /// 更新检票状态
    pub fn check_in(&mut self, operator: Pubkey, timestamp: i64) {
        self.check_in_status = true;
        self.check_in_time = timestamp;
        self.check_in_operator = operator;
    }
    
    /// 序列化扩展字段数据
    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(self.try_to_vec()?)
    }
    
    /// 反序列化扩展字段数据
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        Ok(Self::try_from_slice(data)?)
    }
    
    /// 验证扩展字段数据的有效性
    pub fn validate(&self) -> Result<()> {
        require!(
            self.event_id.len() <= 32,
            crate::errors::ErrorCode::InvalidEventId
        );
        require!(
            self.ticket_uuid.len() <= 32,
            crate::errors::ErrorCode::InvalidTicketPda
        );
        Ok(())
    }
}

/// Token 2022 扩展字段类型
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum TicketExtensionType {
    /// 票据数据扩展字段
    TicketData,
}

impl TicketExtensionType {
    /// 获取扩展字段类型的标识符
    pub fn identifier(&self) -> u8 {
        match self {
            TicketExtensionType::TicketData => 0x01,
        }
    }
}
