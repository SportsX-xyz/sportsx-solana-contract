use anchor_lang::prelude::*;

/// NFT 创建辅助函数
pub struct NftCreator;

impl NftCreator {
    /// 创建 Token 2022 NFT 元数据
    /// 这个函数会调用 Token 2022 程序来创建带有元数据的 NFT
    pub fn create_token2022_nft_metadata(
        mint: &AccountInfo,
        token_program: &AccountInfo,
        system_program: &AccountInfo,
        rent: &AccountInfo,
        name: String,
        symbol: String,
        uri: String,
        seat_row: u16,
        seat_column: u16,
        event_id: String,
        ticket_uuid: String,
    ) -> Result<()> {
        msg!(
            "Creating Token 2022 NFT with metadata: name={}, symbol={}, uri={}, seat={}:{}, event={}, uuid={}", 
            name, symbol, uri, seat_row, seat_column, event_id, ticket_uuid
        );
        
        // For now, we'll create a basic mint without metadata extensions
        // The mint will be created by the calling instruction
        // TODO: Implement Token 2022 metadata extensions when needed
        
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
