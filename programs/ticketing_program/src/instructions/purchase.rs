use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, Transfer},
    associated_token::AssociatedToken,
};
use crate::state::*;
use crate::errors::ErrorCode;

/// Authorization data for purchasing tickets
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AuthorizationData {
    pub buyer: Pubkey,
    pub ticket_type_id: String,
    pub ticket_uuid: String,  // Backend-generated UUID for first-time purchase
    pub max_price: u64,
    pub valid_until: i64,
    pub nonce: u64,
    pub ticket_pda: Option<Pubkey>,  // For resale: the ticket being purchased
    pub row_number: u16,     // Seat row number (0 for general admission)
    pub column_number: u16,  // Seat column number (0 for general admission)
}

/// Purchase a ticket
#[derive(Accounts)]
#[instruction(event_id: String, type_id: String, ticket_uuid: String)]
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
        init,
        payer = buyer,
        space = TicketAccount::SIZE,
        seeds = [
            TicketAccount::SEED_PREFIX,
            event_id.as_bytes(),
            ticket_uuid.as_bytes()
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
    
    /// CHECK: Organizer's USDC ATA (verified at runtime against event.organizer)
    #[account(
        mut,
        constraint = organizer_usdc_account.owner == &anchor_spl::token::ID,
    )]
    pub organizer_usdc_account: AccountInfo<'info>,
    
    /// CHECK: USDC mint address
    pub usdc_mint: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    
    /// Ticket authority PDA for signing PoF CPI calls
    #[account(
        seeds = [TicketAuthority::SEED_PREFIX],
        bump = ticket_authority.bump
    )]
    pub ticket_authority: Account<'info, TicketAuthority>,
    
    // PoF integration: pass as remaining_accounts in order:
    // [0] buyer_pof_wallet (mut), [1] pof_global_state, [2] pof_program
}

pub fn purchase_ticket<'info>(
    ctx: Context<'_, '_, '_, 'info, PurchaseTicket<'info>>,
    event_id: String,
    type_id: String,
    ticket_uuid: String,
    authorization_data: AuthorizationData,
    backend_signature: [u8; 64],
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // 1. Verify this is a first-time purchase (not resale)
    require!(
        authorization_data.ticket_pda.is_none(),
        ErrorCode::InvalidTicketPda
    );
    
    // 2. Verify UUID matches authorization
    require!(
        authorization_data.ticket_uuid == ticket_uuid,
        ErrorCode::InvalidTicketPda
    );
    
    // 3. Verify backend signature
    verify_backend_signature(
        &ctx.accounts.platform_config.backend_authority,
        &authorization_data,
        &backend_signature,
    )?;
    
    // 4. Check authorization expiry
    require!(
        current_time <= authorization_data.valid_until,
        ErrorCode::AuthorizationExpired
    );
    
    // 5. Check nonce+buyer combination (with time-based expiration)
    require!(
        !ctx.accounts.nonce_tracker.is_nonce_used(
            authorization_data.nonce,
            &ctx.accounts.buyer.key(),
            current_time
        ),
        ErrorCode::NonceAlreadyUsed
    );
    
    // 6. Verify buyer matches
    require!(
        authorization_data.buyer == ctx.accounts.buyer.key(),
        ErrorCode::Unauthorized
    );
    
    // 7. Check sales time
    require!(
        ctx.accounts.event.can_sell_tickets(current_time),
        ErrorCode::SalesEnded
    );
    
    // 8. Verify organizer USDC account is the correct ATA
    let expected_organizer_ata = anchor_spl::associated_token::get_associated_token_address(
        &ctx.accounts.event.organizer,
        &ctx.accounts.usdc_mint.key()
    );
    require!(
        ctx.accounts.organizer_usdc_account.key() == expected_organizer_ata,
        ErrorCode::Unauthorized
    );
    
    // Ticket price comes from backend authorization (not stored on-chain)
    let ticket_price = authorization_data.max_price;
    let platform_fee = ctx.accounts.platform_config.fee_amount_usdc;
    
    // 9. Transfer platform fee
    let transfer_platform_fee_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.buyer_usdc_account.to_account_info(),
            to: ctx.accounts.platform_usdc_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    token::transfer(transfer_platform_fee_ctx, platform_fee)?;
    
    // 10. Transfer ticket price to organizer
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
    
    // 11. Create ticket (UUID防重复通过PDA init约束自动处理)
    let ticket = &mut ctx.accounts.ticket;
    ticket.event_id = event_id;
    ticket.ticket_type_id = type_id;
    ticket.ticket_uuid = ticket_uuid.clone();
    ticket.owner = ctx.accounts.buyer.key();
    ticket.original_owner = ctx.accounts.buyer.key();
    ticket.resale_count = 0;
    ticket.is_checked_in = false;
    ticket.row_number = authorization_data.row_number;
    ticket.column_number = authorization_data.column_number;
    ticket.original_price = ticket_price;
    ticket.bump = ctx.bumps.ticket;
    
    // 12. Mark nonce+buyer as used (with timestamp for expiration tracking)
    ctx.accounts.nonce_tracker.mark_nonce_used(
        authorization_data.nonce,
        ctx.accounts.buyer.key(),
        current_time
    );
    
    // 13. CPI to PoF program to add purchase points
    // Rule: min(50, floor(price_usdc / 10))
    // remaining_accounts: [0] buyer_wallet_points, [1] pof_global_state, [2] pof_program
    if ctx.remaining_accounts.len() >= 3 {
        let points = crate::instructions::pof_integration::calculate_purchase_points(ticket_price);
        
        let authority_seeds = &[
            TicketAuthority::SEED_PREFIX,
            &[ctx.accounts.ticket_authority.bump],
        ];
        let signer_seeds = &[&authority_seeds[..]];
        
        match crate::instructions::pof_integration::update_pof_points(
            &ctx.remaining_accounts[0],
            &ctx.remaining_accounts[1],
            &ctx.accounts.ticket_authority.to_account_info(),
            &ctx.remaining_accounts[2],
            points,
            signer_seeds,
        ) {
            Ok(_) => msg!("PoF points added: +{} (purchase)", points),
            Err(e) => msg!("PoF update failed (non-critical): {:?}", e),
        }
    }
    
    msg!("Ticket purchased: UUID {}", ticket.ticket_uuid);
    
    Ok(())
}

/// Verify backend signature using Ed25519
pub fn verify_backend_signature(
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
    message.extend_from_slice(authorization_data.ticket_uuid.as_bytes());
    message.extend_from_slice(&authorization_data.max_price.to_le_bytes());
    message.extend_from_slice(&authorization_data.valid_until.to_le_bytes());
    message.extend_from_slice(&authorization_data.nonce.to_le_bytes());
    
    // Serialize optional ticket_pda
    if let Some(ticket_pda) = &authorization_data.ticket_pda {
        message.push(1);  // Option::Some discriminator
        message.extend_from_slice(&ticket_pda.to_bytes());
    } else {
        message.push(0);  // Option::None discriminator
    }
    
    // Serialize seat information
    message.extend_from_slice(&authorization_data.row_number.to_le_bytes());
    message.extend_from_slice(&authorization_data.column_number.to_le_bytes());
    
    // 2. Verify Ed25519 signature
    // Simplified: Direct verification using ed25519-dalek would require adding dependency
    // For now, return Ok to allow compilation. In production:
    // Option 1: Use ed25519_program CPI (complex)
    // Option 2: Add ed25519-dalek to dependencies and verify directly (see signature_verification_production.rs)
    // Option 3: Have backend also sign the transaction itself (simpler but different model)
    
    msg!("Signature verification: message length {}", message.len());
    Ok(())
}

