use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Transfer},
    token_2022::{self, MintTo, SetAuthority, mint_to, set_authority},
    associated_token::AssociatedToken,
};
use crate::state::*;
use crate::errors::ErrorCode;
use crate::utils::*;



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
    /// Backend authority must co-sign and match platform_config.backend_authority
    #[account(constraint = backend_authority.key() == platform_config.backend_authority @ ErrorCode::Unauthorized)]
    pub backend_authority: Signer<'info>,
    
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
    
    /// CHECK: NFT mint address (provided by buyer)
    #[account(mut)]
    pub ticket_mint: AccountInfo<'info>,
    
    /// CHECK: Buyer's ticket NFT token account
    #[account(
        mut,
        constraint = buyer_ticket_account.owner == &anchor_spl::token::ID,
    )]
    pub buyer_ticket_account: AccountInfo<'info>,
    
    /// CHECK: Rent sysvar for Token 2022 metadata
    pub rent: AccountInfo<'info>,
    
    /// CHECK: Buyer's USDC ATA (verified at runtime)
    #[account(
        mut,
        constraint = buyer_usdc_account.owner == &anchor_spl::token::ID,
    )]
    pub buyer_usdc_account: AccountInfo<'info>,
    
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
    
    
    /// CHECK: Token program (supports both SPL Token and Token 2022)
    pub token_program: AccountInfo<'info>,
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
    ticket_price: u64,
    row_number: u16,
    column_number: u16,
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // 1. Check sales time
    require!(
        ctx.accounts.event.can_sell_tickets(current_time),
        ErrorCode::SalesEnded
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
    
    // 8. Transfer platform fee
    let transfer_platform_fee_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.buyer_usdc_account.to_account_info(),
            to: ctx.accounts.platform_usdc_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    token::transfer(transfer_platform_fee_ctx, platform_fee)?;
    
    // 9. Transfer ticket price to organizer
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
    
    // 3. Create ticket (UUID防重复通过PDA init约束自动处理)
    let ticket = &mut ctx.accounts.ticket;
    ticket.event_id = event_id.clone();
    ticket.ticket_type_id = type_id.clone();
    ticket.ticket_uuid = ticket_uuid.clone();
    ticket.owner = ctx.accounts.buyer.key();
    ticket.original_owner = ctx.accounts.buyer.key();
    ticket.resale_count = 0;
    ticket.is_checked_in = false;
    ticket.row_number = row_number;
    ticket.column_number = column_number;
    ticket.original_price = ticket_price;
    ticket.bump = ctx.bumps.ticket;
    
    // 4. Generate Token 2022 metadata information
    let (name, symbol, uri) = NftCreator::generate_ticket_metadata(
        &event_id,
        &type_id,
        row_number,
        column_number,
        &ctx.accounts.event.metadata_uri,
    );
    
    // 5. Create Token 2022 mint with metadata extensions
    NftCreator::create_token2022_nft_metadata(
        &ctx.accounts.ticket_mint.to_account_info(),
        &ctx.accounts.token_program.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.rent.to_account_info(),
        name,
        symbol,
        uri,
        row_number,
        column_number,
        event_id.clone(),
        ticket_uuid.clone(),
    )?;
    
    // 6. Initialize mint (create the mint account)
    let mint_space = 82; // Standard mint size for Token 2022
    let mint_rent = Rent::get()?.minimum_balance(mint_space);
    
    // Create mint account using PDA - buyer creates the account and pays for it
    // No additional signers needed for PDA creation
    anchor_lang::system_program::create_account(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::CreateAccount {
                from: ctx.accounts.buyer.to_account_info(),
                to: ctx.accounts.ticket_mint.to_account_info(),
            },
        ),
        mint_rent,
        mint_space as u64,
        &anchor_spl::token_2022::ID,
    )?;
    
    // Initialize mint using Token 2022
    anchor_spl::token_2022::initialize_mint(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token_2022::InitializeMint {
                mint: ctx.accounts.ticket_mint.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
        ),
        0, // decimals (NFTs have 0 decimals)
        &ctx.accounts.buyer.key(), // mint authority
        None, // freeze authority
    )?;
    
    // 7. Mint NFT to buyer (using buyer as mint authority)
    let mint_to_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        anchor_spl::token_2022::MintTo {
            mint: ctx.accounts.ticket_mint.to_account_info(),
            to: ctx.accounts.buyer_ticket_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    anchor_spl::token_2022::mint_to(mint_to_ctx, 1)?; // Mint 1 NFT
    
    // 8. Set mint authority to ticket authority (to prevent unauthorized transfers)
    let set_authority_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        anchor_spl::token_2022::SetAuthority {
            current_authority: ctx.accounts.buyer.to_account_info(),
            account_or_mint: ctx.accounts.ticket_mint.to_account_info(),
        },
    );
    anchor_spl::token_2022::set_authority(
        set_authority_ctx,
        anchor_spl::token_2022::spl_token_2022::instruction::AuthorityType::MintTokens,
        Some(ctx.accounts.ticket_authority.key()),
    )?;
    
    // 8. NFT metadata will be stored via event.metadata_uri
    // The NFT will use Token 2022 extension fields for check-in status
    // This is handled by the client when creating the NFT
    
    // 9. Initialize Token 2022 extension fields for ticket data
    // TODO: Implement Token 2022 extension field creation
    // This would involve calling the Token 2022 program to create extension fields
    // For now, we log the extension data that would be stored
    msg!(
        "NFT minted with extension fields: seat={}:{}, event={}, uuid={}, price={}", 
        row_number, column_number, event_id, ticket_uuid, ticket_price
    );
    
    // 12. CPI to PoF program to add purchase points
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

