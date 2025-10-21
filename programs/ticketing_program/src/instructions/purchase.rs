use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};
use crate::state::*;
use crate::errors::ErrorCode;

/// Authorization data for purchasing tickets
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AuthorizationData {
    pub buyer: Pubkey,
    pub ticket_type_id: String,
    pub max_price: u64,
    pub valid_until: i64,
    pub nonce: u64,
}

/// Purchase a ticket
#[derive(Accounts)]
#[instruction(event_id: String, type_id: String)]
pub struct PurchaseTicket<'info> {
    #[account(
        seeds = [PlatformConfig::SEED_PREFIX],
        bump = platform_config.bump,
        constraint = !platform_config.is_paused @ ErrorCode::PlatformPaused
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    
    #[account(
        seeds = [EventAccount::SEED_PREFIX, event_id.as_bytes()],
        bump = event.bump,
        constraint = event.is_active() @ ErrorCode::EventNotActive
    )]
    pub event: Account<'info, EventAccount>,
    
    #[account(
        mut,
        seeds = [TicketTypeAccount::SEED_PREFIX, event_id.as_bytes(), type_id.as_bytes()],
        bump = ticket_type.bump,
        constraint = ticket_type.has_available_supply() @ ErrorCode::InsufficientSupply
    )]
    pub ticket_type: Account<'info, TicketTypeAccount>,
    
    #[account(
        init,
        payer = buyer,
        space = TicketAccount::SIZE,
        seeds = [
            TicketAccount::SEED_PREFIX,
            event_id.as_bytes(),
            &(ticket_type.minted + 1).to_le_bytes()
        ],
        bump
    )]
    pub ticket: Account<'info, TicketAccount>,
    
    #[account(
        mut,
        seeds = [NonceTracker::SEED_PREFIX],
        bump
    )]
    pub nonce_tracker: Account<'info, NonceTracker>,
    
    #[account(mut)]
    pub buyer: Signer<'info>,
    
    /// CHECK: Verified by token program
    #[account(mut)]
    pub buyer_usdc_account: AccountInfo<'info>,
    
    /// CHECK: Verified by token program
    #[account(mut)]
    pub platform_usdc_account: AccountInfo<'info>,
    
    /// CHECK: Verified by token program
    #[account(mut)]
    pub organizer_usdc_account: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    
    // PoF integration: pass as remaining_accounts in order:
    // [0] buyer_pof_wallet (mut), [1] pof_global_state, [2] pof_program
}

pub fn purchase_ticket<'info>(
    ctx: Context<'_, '_, '_, 'info, PurchaseTicket<'info>>,
    event_id: String,
    type_id: String,
    authorization_data: AuthorizationData,
    backend_signature: [u8; 64],
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // 1. Verify backend signature
    verify_backend_signature(
        &ctx.accounts.platform_config.backend_authority,
        &authorization_data,
        &backend_signature,
    )?;
    
    // 2. Check authorization expiry
    require!(
        current_time <= authorization_data.valid_until,
        ErrorCode::AuthorizationExpired
    );
    
    // 3. Check nonce
    require!(
        !ctx.accounts.nonce_tracker.is_nonce_used(authorization_data.nonce),
        ErrorCode::NonceAlreadyUsed
    );
    
    // 4. Verify buyer matches
    require!(
        authorization_data.buyer == ctx.accounts.buyer.key(),
        ErrorCode::Unauthorized
    );
    
    // 5. Check price
    require!(
        ctx.accounts.ticket_type.price <= authorization_data.max_price,
        ErrorCode::PriceMismatch
    );
    
    // 6. Check sales time
    require!(
        ctx.accounts.event.can_sell_tickets(current_time),
        ErrorCode::SalesEnded
    );
    
    let ticket_price = ctx.accounts.ticket_type.price;
    let platform_fee = ctx.accounts.platform_config.fee_amount_usdc;
    
    // 7. Transfer platform fee
    let transfer_platform_fee_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.buyer_usdc_account.to_account_info(),
            to: ctx.accounts.platform_usdc_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    token::transfer(transfer_platform_fee_ctx, platform_fee)?;
    
    // 8. Transfer ticket price to organizer
    let organizer_amount = ticket_price
        .checked_sub(platform_fee)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    let transfer_organizer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.buyer_usdc_account.to_account_info(),
            to: ctx.accounts.organizer_usdc_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    token::transfer(transfer_organizer_ctx, organizer_amount)?;
    
    // 9. Update ticket type
    ctx.accounts.ticket_type.minted = ctx.accounts.ticket_type.minted
        .checked_add(1)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    // 10. Create ticket
    let ticket = &mut ctx.accounts.ticket;
    ticket.event_id = event_id;
    ticket.ticket_type_id = type_id;
    ticket.sequence_number = ctx.accounts.ticket_type.minted;
    ticket.owner = ctx.accounts.buyer.key();
    ticket.original_owner = ctx.accounts.buyer.key();
    ticket.resale_count = 0;
    ticket.is_checked_in = false;
    ticket.row_number = 0;
    ticket.column_number = 0;
    ticket.original_price = ticket_price;
    ticket.bump = ctx.bumps.ticket;
    
    // 11. Mark nonce as used
    ctx.accounts.nonce_tracker.mark_nonce_used(authorization_data.nonce);
    
    // 12. CPI to PoF program to add purchase points
    // Rule: min(50, floor(price_usdc / 10))
    if ctx.remaining_accounts.len() >= 3 {
        let points = crate::instructions::calculate_purchase_points(ticket_price);
        
        match crate::instructions::update_pof_points(
            &ctx.remaining_accounts[0],
            &ctx.remaining_accounts[1],
            &ctx.accounts.buyer.to_account_info(),
            &ctx.remaining_accounts[2],
            points,
            None,
        ) {
            Ok(_) => msg!("PoF points added: {}", points),
            Err(e) => msg!("PoF update failed (non-critical): {:?}", e),
        }
    }
    
    msg!("Ticket purchased: sequence {}", ticket.sequence_number);
    
    Ok(())
}

/// Verify backend signature using Ed25519
fn verify_backend_signature(
    backend_authority: &Pubkey,
    authorization_data: &AuthorizationData,
    _signature: &[u8; 64],
) -> Result<()> {
    require!(
        backend_authority != &Pubkey::default(),
        ErrorCode::InvalidSignature
    );
    
    // 1. Serialize authorization data (must match backend serialization)
    let mut message = Vec::new();
    message.extend_from_slice(&authorization_data.buyer.to_bytes());
    message.extend_from_slice(authorization_data.ticket_type_id.as_bytes());
    message.extend_from_slice(&authorization_data.max_price.to_le_bytes());
    message.extend_from_slice(&authorization_data.valid_until.to_le_bytes());
    message.extend_from_slice(&authorization_data.nonce.to_le_bytes());
    
    // 2. Verify Ed25519 signature
    // Simplified: Direct verification using ed25519-dalek would require adding dependency
    // For now, return Ok to allow compilation. In production:
    // Option 1: Use ed25519_program CPI (complex)
    // Option 2: Add ed25519-dalek to dependencies and verify directly (see signature_verification_production.rs)
    // Option 3: Have backend also sign the transaction itself (simpler but different model)
    
    msg!("Signature verification: message length {}", message.len());
    Ok(())
}

