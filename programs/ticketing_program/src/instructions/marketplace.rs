use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, Transfer},
    associated_token::AssociatedToken,
};
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
    
    /// CHECK: Buyer's USDC ATA (verified at runtime)
    #[account(
        mut,
        constraint = buyer_usdc_account.owner == &anchor_spl::token::ID,
    )]
    pub buyer_usdc_account: AccountInfo<'info>,
    
    /// CHECK: Seller's USDC ATA (verified at runtime)
    #[account(
        mut,
        constraint = seller_usdc_account.owner == &anchor_spl::token::ID,
    )]
    pub seller_usdc_account: AccountInfo<'info>,
    
    /// CHECK: Platform's USDC ATA (verified at runtime)
    #[account(
        mut,
        constraint = platform_usdc_account.owner == &anchor_spl::token::ID,
    )]
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
    
    /// Ticket authority PDA for signing PoF CPI calls
    #[account(
        seeds = [TicketAuthority::SEED_PREFIX],
        bump = ticket_authority.bump
    )]
    pub ticket_authority: Account<'info, TicketAuthority>,
    
    // PoF integration: pass as remaining_accounts in order:
    // [0] seller_pof_wallet (mut), [1] buyer_pof_wallet (mut), 
    // [2] pof_global_state, [3] pof_program
}

pub fn buy_listed_ticket<'info>(
    ctx: Context<'_, '_, '_, 'info, BuyListedTicket<'info>>,
    resale_price: u64,
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // 1. Verify price matches listing
    require!(
        ctx.accounts.listing.price == resale_price,
        ErrorCode::PriceMismatch
    );
    
    // 2. Verify all USDC accounts are correct ATAs
    let expected_buyer_ata = anchor_spl::associated_token::get_associated_token_address(
        &ctx.accounts.buyer.key(),
        &ctx.accounts.usdc_mint.key()
    );
    require!(
        ctx.accounts.buyer_usdc_account.key() == expected_buyer_ata,
        ErrorCode::Unauthorized
    );
    
    let expected_seller_ata = anchor_spl::associated_token::get_associated_token_address(
        &ctx.accounts.original_seller.key(),
        &ctx.accounts.usdc_mint.key()
    );
    require!(
        ctx.accounts.seller_usdc_account.key() == expected_seller_ata,
        ErrorCode::Unauthorized
    );
    
    let expected_platform_ata = anchor_spl::associated_token::get_associated_token_address(
        &ctx.accounts.platform_config.fee_receiver,
        &ctx.accounts.usdc_mint.key()
    );
    require!(
        ctx.accounts.platform_usdc_account.key() == expected_platform_ata,
        ErrorCode::Unauthorized
    );
    
    let expected_organizer_ata = anchor_spl::associated_token::get_associated_token_address(
        &ctx.accounts.event.organizer,
        &ctx.accounts.usdc_mint.key()
    );
    require!(
        ctx.accounts.organizer_usdc_account.key() == expected_organizer_ata,
        ErrorCode::Unauthorized
    );
    
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
    
    // 5. Update ticket ownership (no nonce tracking needed for resale)
    
    // 6. CPI to PoF program for resale points
    // Seller: -original_points, Buyer: +new_points
    // CPI to PoF program for resale points adjustment
    // remaining_accounts: [0] seller_wallet_points, [1] buyer_wallet_points, [2] pof_global_state, [3] pof_program
    if ctx.remaining_accounts.len() >= 4 {
        let original_points = crate::instructions::pof_integration::calculate_purchase_points(original_price);
        let new_points = crate::instructions::pof_integration::calculate_purchase_points(resale_price);
        
        let authority_seeds = &[
            TicketAuthority::SEED_PREFIX,
            &[ctx.accounts.ticket_authority.bump],
        ];
        let signer_seeds = &[&authority_seeds[..]];
        
        // Deduct original purchase points from seller
        match crate::instructions::pof_integration::update_pof_points(
            &ctx.remaining_accounts[0],
            &ctx.remaining_accounts[2],
            &ctx.accounts.ticket_authority.to_account_info(),
            &ctx.remaining_accounts[3],
            -original_points,
            signer_seeds,
        ) {
            Ok(_) => msg!("PoF points deducted from seller: -{} (resale)", original_points),
            Err(e) => msg!("PoF seller deduction failed (non-critical): {:?}", e),
        }
        
        // Add new purchase points to buyer based on resale price
        match crate::instructions::pof_integration::update_pof_points(
            &ctx.remaining_accounts[1],
            &ctx.remaining_accounts[2],
            &ctx.accounts.ticket_authority.to_account_info(),
            &ctx.remaining_accounts[3],
            new_points,
            signer_seeds,
        ) {
            Ok(_) => msg!("PoF points added to buyer: +{} (resale)", new_points),
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

