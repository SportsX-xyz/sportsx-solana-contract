use anchor_lang::prelude::*;

/// NFT 创建辅助函数
pub struct NftCreator;

impl NftCreator {
    /// 创建 Token 2022 NFT 元数据
    /// 这个函数会调用 Token 2022 程序来创建带有元数据的 NFT
    pub fn create_token2022_nft_metadata(
        _mint: &AccountInfo,
        _token_program: &AccountInfo,
        _system_program: &AccountInfo,
        _rent: &AccountInfo,
        name: String,
        symbol: String,
        uri: String,
        seat_row: u16,
        seat_column: u16,
        event_id: String,
        ticket_uuid: String,
    ) -> Result<()> {
        // TODO: 实现 Token 2022 元数据创建逻辑
        // 这需要调用 Token 2022 程序的以下指令：
        // 1. create_initialize_metadata_pointer_instruction - 启用元数据指针扩展
        // 2. create_initialize_instruction - 初始化元数据 (name, symbol, uri)
        // 3. create_update_field_instruction - 添加自定义字段 (座位号, 检票状态等)
        
        msg!(
            "Creating Token 2022 NFT with metadata: name={}, symbol={}, uri={}, seat={}:{}, event={}, uuid={}", 
            name, symbol, uri, seat_row, seat_column, event_id, ticket_uuid
        );
        
        // 目前只是记录日志，实际实现需要调用 Token 2022 CPI
        Ok(())
    }
    
    
    /// 生成票据 NFT 的元数据信息
    pub fn generate_ticket_metadata(
        event_id: &str,
        ticket_type_id: &str,
        row_number: u16,
        column_number: u16,
        event_metadata_uri: &str,
    ) -> (String, String, String) {
        let name = format!("SportsX Ticket - {} #{}", event_id, ticket_type_id);
        let symbol = "SPORTSX".to_string();
        
        // 使用事件的 metadata_uri 作为基础，可以添加座位信息
        let uri = if row_number > 0 && column_number > 0 {
            format!("{}?row={}&col={}", event_metadata_uri, row_number, column_number)
        } else {
            event_metadata_uri.to_string()
        };
        
        (name, symbol, uri)
    }
}
