use anchor_lang::prelude::*;

#[account]
pub struct TicketTypeAccount {
    /// Event ID (max 32 chars)
    pub event_id: String,
    
    /// Ticket type ID (max 32 chars)
    pub type_id: String,
    
    /// Tier name (max 64 chars)
    pub tier_name: String,
    
    /// Price in USDC (6 decimals)
    pub price: u64,
    
    /// Total supply
    pub total_supply: u32,
    
    /// Number of tickets minted
    pub minted: u32,
    
    /// Color code for UI display
    pub color: u32,
    
    /// PDA bump
    pub bump: u8,
}

impl TicketTypeAccount {
    pub const SEED_PREFIX: &'static [u8] = b"ticket_type";
    
    // 4+32 + 4+32 + 4+64 + 8 + 4 + 4 + 4 + 1 = 161 bytes
    pub const SIZE: usize = 8 + 161;
    
    pub fn has_available_supply(&self) -> bool {
        self.minted < self.total_supply
    }
}

