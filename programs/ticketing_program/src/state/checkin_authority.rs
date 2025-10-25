use anchor_lang::prelude::*;

#[account]
pub struct CheckInAuthority {
    /// Event ID (32 bytes)
    pub event_id: [u8; 32],
    
    /// Operator public key
    pub operator: Pubkey,
    
    /// Active status
    pub is_active: bool,
    
    /// PDA bump
    pub bump: u8,
}

impl CheckInAuthority {
    pub const SEED_PREFIX: &'static [u8] = b"checkin_auth";
    
    // 32 + 32 + 1 + 1 = 66 bytes
    pub const SIZE: usize = 8 + 66;
}

