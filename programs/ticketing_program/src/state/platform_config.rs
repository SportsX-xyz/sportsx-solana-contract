use anchor_lang::prelude::*;

#[account]
pub struct PlatformConfig {
    /// Platform fee receiver address
    pub fee_receiver: Pubkey,
    
    /// Platform fee in USDC (6 decimals), e.g., 0.1 USDC = 100000
    pub fee_amount_usdc: u64,
    
    /// Update authority (multisig address)
    pub update_authority: Pubkey,
    
    /// Backend signing authority
    pub backend_authority: Pubkey,
    
    /// Platform pause status
    pub is_paused: bool,
    
    /// PDA bump
    pub bump: u8,
}

impl PlatformConfig {
    pub const SEED_PREFIX: &'static [u8] = b"platform_config";
    
    // 32 + 8 + 32 + 32 + 1 + 1 = 106 bytes
    pub const SIZE: usize = 8 + 106;
}

