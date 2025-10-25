use anchor_lang::prelude::*;

#[account]
pub struct EventAccount {
    /// Event ID (32 bytes)
    pub event_id: [u8; 32],
    
    /// Event name (max 50 chars)
    pub name: String,
    
    /// Event symbol (max 10 chars)
    pub symbol: String,
    
    /// Event organizer
    pub organizer: Pubkey,
    
    /// Metadata URI (IPFS, max 200 chars)
    pub metadata_uri: String,
    
    /// Event start time (Unix timestamp)
    pub start_time: i64,
    
    /// Event end time (Unix timestamp)
    pub end_time: i64,
    
    /// Ticket release time (Unix timestamp)
    pub ticket_release_time: i64,
    
    /// Stop sale before event start (seconds)
    pub stop_sale_before: i64,
    
    /// Resale fee rate in basis points (100 = 1%)
    pub resale_fee_rate: u16,
    
    /// Maximum resale times allowed
    pub max_resale_times: u8,
    
    /// Event status: 0=Draft, 1=Active, 2=Disabled
    pub status: u8,
    
    /// PDA bump
    pub bump: u8,
}

impl EventAccount {
    pub const SEED_PREFIX: &'static [u8] = b"event";
    
    // 32 + 4+50 + 4+10 + 32 + 4+200 + 8 + 8 + 8 + 8 + 2 + 1 + 1 + 1 = 341 bytes
    pub const SIZE: usize = 8 + 341;
    
    pub fn is_active(&self) -> bool {
        self.status == 1
    }
    
    pub fn can_sell_tickets(&self, current_time: i64) -> bool {
        self.is_active() 
            && current_time >= self.ticket_release_time
            && current_time <= (self.start_time - self.stop_sale_before)
    }
}

