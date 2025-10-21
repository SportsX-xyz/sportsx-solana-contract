use anchor_lang::prelude::*;

#[account]
pub struct CheckInAuthority {
    /// Event ID (max 32 chars)
    pub event_id: String,
    
    /// Operator public key
    pub operator: Pubkey,
    
    /// Active status
    pub is_active: bool,
    
    /// PDA bump
    pub bump: u8,
}

impl CheckInAuthority {
    pub const SEED_PREFIX: &'static [u8] = b"checkin_auth";
    
    // 4+32 + 32 + 1 + 1 = 70 bytes
    pub const SIZE: usize = 8 + 70;
}

