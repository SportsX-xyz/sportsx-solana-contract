use anchor_lang::prelude::*;

/// Global ticket authority PDA for signing PoF CPI calls
#[account]
pub struct TicketAuthority {
    pub bump: u8,
}

impl TicketAuthority {
    pub const SEED_PREFIX: &'static [u8] = b"ticket_authority";
    pub const SIZE: usize = 8 + 1; // discriminator + bump
}

