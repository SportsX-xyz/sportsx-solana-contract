use anchor_lang::prelude::*;

#[account]
pub struct TicketAccount {
    /// Event ID (32 bytes)
    pub event_id: [u8; 32],
    
    /// Ticket type ID (max 32 chars)
    pub ticket_type_id: String,
    
    /// Ticket UUID (32 bytes, UUID without hyphens)
    pub ticket_uuid: [u8; 32],
    
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
    
    // 32 + 4+32 + 32 + 32 + 32 + 1 + 1 + 2 + 2 + 8 + 1 = 147 bytes
    pub const SIZE: usize = 8 + 147;
    
    pub fn can_resell(&self, max_resale_times: u8) -> bool {
        !self.is_checked_in && self.resale_count < max_resale_times
    }
}

