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
    
    /// Event admin (only this address can create events)
    pub event_admin: Pubkey,
    
    /// Platform pause status
    pub is_paused: bool,
    
    /// PDA bump
    pub bump: u8,
}

impl PlatformConfig {
    pub const SEED_PREFIX: &'static [u8] = b"platform_config";
    pub const PROGRAM_AUTHORITY_SEED: &'static [u8] = b"program_authority";
    
    // 32 + 8 + 32 + 32 + 32 + 1 + 1 = 138 bytes
    pub const SIZE: usize = 8 + 138;
    
    /// Derive program authority PDA for holding listed tickets
    pub fn derive_program_authority(program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[Self::PROGRAM_AUTHORITY_SEED],
            program_id
        )
    }
}

