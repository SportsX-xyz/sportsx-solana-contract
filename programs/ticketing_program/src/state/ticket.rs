use anchor_lang::prelude::*;

#[account]
pub struct TicketAccount {
    /// Event ID (max 32 chars)
    pub event_id: String,
    
    /// Ticket type ID (max 32 chars)
    pub ticket_type_id: String,
    
    /// Sequence number (unique within event)
    pub sequence_number: u32,
    
    /// Current owner
    pub owner: Pubkey,
    
    /// Original owner
    pub original_owner: Pubkey,
    
    /// Resale count
    pub resale_count: u8,
    
    /// Check-in status
    pub is_checked_in: bool,
    
    /// Seat row number
    pub row_number: u16,
    
    /// Seat column number
    pub column_number: u16,
    
    /// Original purchase price (for PoF points calculation on resale)
    pub original_price: u64,
    
    /// PDA bump
    pub bump: u8,
}

impl TicketAccount {
    pub const SEED_PREFIX: &'static [u8] = b"ticket";
    
    // 4+32 + 4+32 + 4 + 32 + 32 + 1 + 1 + 2 + 2 + 8 + 1 = 155 bytes
    pub const SIZE: usize = 8 + 155;
    
    pub fn can_resell(&self, max_resale_times: u8) -> bool {
        !self.is_checked_in && self.resale_count < max_resale_times
    }
}

