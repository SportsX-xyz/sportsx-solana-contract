use anchor_lang::prelude::*;

#[account]
pub struct ListingAccount {
    /// Ticket PDA
    pub ticket_pda: Pubkey,
    
    /// Original seller address (ticket owner when listed)
    pub original_seller: Pubkey,
    
    /// Listing price in USDC (6 decimals)
    pub price: u64,
    
    /// Listed timestamp
    pub listed_at: i64,
    
    /// Active status
    pub is_active: bool,
    
    /// PDA bump
    pub bump: u8,
}

impl ListingAccount {
    pub const SEED_PREFIX: &'static [u8] = b"listing";
    
    // 32 + 32 + 8 + 8 + 1 + 1 = 82 bytes
    pub const SIZE: usize = 8 + 82;
}

