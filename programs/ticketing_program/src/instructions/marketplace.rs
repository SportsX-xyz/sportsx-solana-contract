use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};
use crate::state::*;
use crate::errors::ErrorCode;

/// List a ticket for resale
#[derive(Accounts)]
pub struct ListTicket<'info> {
    #[account(
        seeds = [EventAccount::SEED_PREFIX, ticket.event_id.as_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, EventAccount>,
    
    #[account(
        mut,
        constraint = ticket.owner == seller.key() @ ErrorCode::NotTicketOwner,
        constraint = ticket.can_resell(event.max_resale_times) @ ErrorCode::CannotResellTicket
    )]
    pub ticket: Account<'info, TicketAccount>,
    
    #[account(
        init,
        payer = seller,
        space = ListingAccount::SIZE,
        seeds = [ListingAccount::SEED_PREFIX, ticket.key().as_ref()],
        bump
    )]
    pub listing: Account<'info, ListingAccount>,
    
    #[account(mut)]
    pub seller: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn list_ticket(
    ctx: Context<ListTicket>,
    resale_price: u64,
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // Check that listing is before event start
    require!(
        current_time < ctx.accounts.event.start_time,
        ErrorCode::SalesEnded
    );
    
    // Get program authority PDA
    let (program_authority, _) = PlatformConfig::derive_program_authority(&crate::ID);
    
    // Transfer ticket ownership to program authority
    let ticket = &mut ctx.accounts.ticket;
    ticket.owner = program_authority;
    
    // Create listing record
    let listing = &mut ctx.accounts.listing;
    listing.ticket_pda = ctx.accounts.ticket.key();
    listing.original_seller = ctx.accounts.seller.key();
    listing.price = resale_price;
    listing.listed_at = current_time;
    listing.is_active = true;
    listing.bump = ctx.bumps.listing;
    
    msg!("Ticket listed, ownership transferred to program authority");
    
    Ok(())
}

/// Buy a listed ticket
#[derive(Accounts)]
pub struct BuyListedTicket<'info> {
    #[account(
        seeds = [PlatformConfig::SEED_PREFIX],
        bump = platform_config.bump,
        constraint = !platform_config.is_paused @ ErrorCode::PlatformPaused
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    
    #[account(
        seeds = [EventAccount::SEED_PREFIX, ticket.event_id.as_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, EventAccount>,
    
    #[account(
        mut,
        close = original_seller,
        constraint = listing.is_active @ ErrorCode::ListingNotActive
    )]
    pub listing: Account<'info, ListingAccount>,
    
    #[account(
        mut,
        constraint = listing.ticket_pda == ticket.key()
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
    
    /// CHECK: Original seller receives USDC and rent refund
    #[account(
        mut,
        constraint = listing.original_seller == original_seller.key()
    )]
    pub original_seller: AccountInfo<'info>,
    
    /// CHECK: Verified by token program
    #[account(mut)]
    pub buyer_usdc_account: AccountInfo<'info>,
    
    /// CHECK: Verified by token program
    #[account(mut)]
    pub seller_usdc_account: AccountInfo<'info>,
    
    /// CHECK: Verified by token program
    #[account(mut)]
    pub platform_usdc_account: AccountInfo<'info>,
    
    /// CHECK: Verified by token program
    #[account(mut)]
    pub organizer_usdc_account: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
    
    // PoF integration: pass as remaining_accounts in order:
    // [0] seller_pof_wallet (mut), [1] buyer_pof_wallet (mut), 
    // [2] pof_global_state, [3] pof_program
}

pub fn buy_listed_ticket<'info>(
    ctx: Context<'_, '_, '_, 'info, BuyListedTicket<'info>>,
    authorization_data: super::purchase::AuthorizationData,
    backend_signature: [u8; 64],
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // 1. Verify this is a resale transaction
    require!(
        authorization_data.ticket_pda == Some(ctx.accounts.ticket.key()),
        ErrorCode::InvalidTicketPda
    );
    
    // 2. Verify backend signature
    super::purchase::verify_backend_signature(
        &ctx.accounts.platform_config.backend_authority,
        &authorization_data,
        &backend_signature,
    )?;
    
    // 3. Check authorization not expired
    require!(
        current_time <= authorization_data.valid_until,
        ErrorCode::AuthorizationExpired
    );
    
    // 4. Check nonce not used
    require!(
        !ctx.accounts.nonce_tracker.is_nonce_used(
            authorization_data.nonce,
            &ctx.accounts.buyer.key(),
            current_time
        ),
        ErrorCode::NonceAlreadyUsed
    );
    
    // 5. Verify buyer matches
    require!(
        authorization_data.buyer == ctx.accounts.buyer.key(),
        ErrorCode::Unauthorized
    );
    
    // 6. Verify price
    require!(
        ctx.accounts.listing.price <= authorization_data.max_price,
        ErrorCode::PriceMismatch
    );
    
    let resale_price = ctx.accounts.listing.price;
    let platform_fee = ctx.accounts.platform_config.fee_amount_usdc;
    
    // Calculate organizer resale fee
    let organizer_fee = (resale_price as u128)
        .checked_mul(ctx.accounts.event.resale_fee_rate as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(10000)
        .ok_or(ErrorCode::ArithmeticOverflow)? as u64;
    
    // Calculate seller amount
    let seller_amount = resale_price
        .checked_sub(platform_fee)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_sub(organizer_fee)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    // 1. Transfer platform fee
    let transfer_platform_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.buyer_usdc_account.to_account_info(),
            to: ctx.accounts.platform_usdc_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    token::transfer(transfer_platform_ctx, platform_fee)?;
    
    // 2. Transfer organizer resale fee
    let transfer_organizer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.buyer_usdc_account.to_account_info(),
            to: ctx.accounts.organizer_usdc_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    token::transfer(transfer_organizer_ctx, organizer_fee)?;
    
    // 3. Transfer remaining to seller
    let transfer_seller_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.buyer_usdc_account.to_account_info(),
            to: ctx.accounts.seller_usdc_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    token::transfer(transfer_seller_ctx, seller_amount)?;
    
    // 4. Update ticket ownership
    let ticket = &mut ctx.accounts.ticket;
    let original_price = ticket.original_price;
    ticket.owner = ctx.accounts.buyer.key();
    ticket.resale_count = ticket.resale_count
        .checked_add(1)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    // 5. Mark nonce as used
    ctx.accounts.nonce_tracker.mark_nonce_used(
        authorization_data.nonce,
        ctx.accounts.buyer.key(),
        current_time
    );
    
    // 6. CPI to PoF program for resale points
    // Seller: -original_points, Buyer: +new_points
    if ctx.remaining_accounts.len() >= 4 {
        let original_points = crate::instructions::calculate_purchase_points(original_price);
        let new_points = crate::instructions::calculate_purchase_points(resale_price);
        
        // Deduct original purchase points from seller
        match crate::instructions::update_pof_points(
            &ctx.remaining_accounts[0],
            &ctx.remaining_accounts[2],
            &ctx.accounts.buyer.to_account_info(),
            &ctx.remaining_accounts[3],
            -original_points,
            None,
        ) {
            Ok(_) => msg!("PoF points deducted from seller: -{}", original_points),
            Err(e) => msg!("PoF seller deduction failed (non-critical): {:?}", e),
        }
        
        // Add new purchase points to buyer based on resale price
        match crate::instructions::update_pof_points(
            &ctx.remaining_accounts[1],
            &ctx.remaining_accounts[2],
            &ctx.accounts.buyer.to_account_info(),
            &ctx.remaining_accounts[3],
            new_points,
            None,
        ) {
            Ok(_) => msg!("PoF points added to buyer: +{}", new_points),
            Err(e) => msg!("PoF buyer addition failed (non-critical): {:?}", e),
        }
    }
    
    msg!("Ticket resold successfully");
    
    // Listing account is closed automatically via close constraint
    
    Ok(())
}

/// Cancel a ticket listing
#[derive(Accounts)]
pub struct CancelListing<'info> {
    #[account(
        mut,
        close = seller,
        constraint = listing.original_seller == seller.key() @ ErrorCode::Unauthorized
    )]
    pub listing: Account<'info, ListingAccount>,
    
    #[account(
        mut,
        constraint = listing.ticket_pda == ticket.key()
    )]
    pub ticket: Account<'info, TicketAccount>,
    
    #[account(mut)]
    pub seller: Signer<'info>,
}

pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
    // Return ticket ownership to original seller
    let ticket = &mut ctx.accounts.ticket;
    ticket.owner = ctx.accounts.listing.original_seller;
    
    msg!("Listing cancelled, ownership returned to seller");
    
    // Listing account is closed automatically via close constraint
    
    Ok(())
}

