use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Transfer, Token},
    token_2022::Token2022,
    associated_token::AssociatedToken,
};
use spl_token_2022::extension::metadata_pointer::instruction as metadata_pointer_instruction;
use spl_token_2022::extension::ExtensionType;
use spl_token_2022::state::Mint as StateMint;
use spl_token_metadata_interface::instruction as metadata_instruction;
use spl_token_metadata_interface::state::Field;
use anchor_spl::token_interface::spl_pod::optional_keys::OptionalNonZeroPubkey;
use crate::state::*;
use crate::errors::ErrorCode;

const MINT_AUTH_SEED: &[u8] = b"mint_authority";

/// Purchase a ticket
#[derive(Accounts)]
#[instruction(event_id: [u8; 32], ticket_uuid: [u8; 32])]
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
        seeds = [EventAccount::SEED_PREFIX, &event_id],
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
            &event_id,
            &ticket_uuid
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
    
    /// NFT mint address (provided by buyer as signer for account creation)
    #[account(mut)]
    pub ticket_mint: Signer<'info>,
    
    /// CHECK: Mint authority PDA for controlling NFT minting
    #[account(
        seeds = [MINT_AUTH_SEED],
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,
    
    /// CHECK: Buyer's ticket NFT token account (will be created if not exists)
    #[account(mut)]
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
    
    /// USDC Token program (for USDC transfers)
    #[account(address = anchor_spl::token::ID)]
    pub usdc_token_program: Program<'info, Token>,
    
    /// Token 2022 program (for NFT minting)
    #[account(address = anchor_spl::token_2022::ID)]
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// System program
    #[account(address = anchor_lang::system_program::ID)]
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
    event_id: [u8; 32],
    ticket_uuid: [u8; 32],
    ticket_price: u64,
    row_number: u16,
    column_number: u16,
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // Define mint authority PDA signer seeds for reuse
    let bump = ctx.bumps.mint_authority;
    let auth_seeds = &[MINT_AUTH_SEED, &[bump]];
    let signer_seeds = &[&auth_seeds[..]];
    
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
        ctx.accounts.usdc_token_program.to_account_info(),
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
        ctx.accounts.usdc_token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.buyer_usdc_account.to_account_info(),
            to: ctx.accounts.organizer_usdc_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    token::transfer(transfer_organizer_ctx, organizer_amount)?;
    
    // 3. Create ticket (UUID防重复通过PDA init约束自动处理)
    let ticket = &mut ctx.accounts.ticket;
    ticket.event_id = event_id;
    ticket.ticket_uuid = ticket_uuid;
    ticket.owner = ctx.accounts.buyer.key();
    ticket.original_owner = ctx.accounts.buyer.key();
    ticket.resale_count = 0;
    ticket.is_checked_in = false;
    ticket.row_number = row_number;
    ticket.column_number = column_number;
    ticket.original_price = ticket_price;
    ticket.bump = ctx.bumps.ticket;
    
    // Step 1: System program creates ticket_mint account with metadata pointer extension
    let space = match ExtensionType::try_calculate_account_len::<StateMint>(&[ExtensionType::MetadataPointer]) {
        Ok(space) => space,
        Err(_) => {
            return Err(ErrorCode::InvalidMintAccountSpace.into());
        }
    };

    // This is the space required for the metadata account.
    // We put the meta data into the mint account at the end so we
    // don't need to create and additional account.
    let meta_data_space = 250;

    // let rent = Rent::from_account_info(&ctx.accounts.rent)?;
    let lamports_required =  Rent::get()?.minimum_balance(space + meta_data_space);

    msg!(
        "Create Mint and metadata account size and cost: {} lamports: {}",
        space as u64,
        lamports_required
    );
    
    anchor_lang::system_program::create_account(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::CreateAccount {
                from: ctx.accounts.buyer.to_account_info(),
                to: ctx.accounts.ticket_mint.to_account_info(),
            },
        ),
        lamports_required,
        space as u64,
        &anchor_spl::token_2022::ID,
    )?;
    
    // Step 2: Assign the mint to the token program 2022
    anchor_lang::system_program::assign(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Assign {
                account_to_assign: ctx.accounts.ticket_mint.to_account_info(),
            },
        ),
        &anchor_spl::token_2022::ID,
    )?;
    
    // Step 3: Initialize the metadata pointer
    // Use Token 2022 metadata pointer extension to initialize metadata pointer
    let initialize_pointer_ix = metadata_pointer_instruction::initialize(
        &anchor_spl::token_2022::ID,
        &ctx.accounts.ticket_mint.key(),
        Some(ctx.accounts.mint_authority.key()), // authority (mint authority PDA)
        Some(ctx.accounts.ticket_mint.key()), // metadata_address (will be set later)
    )?;

    anchor_lang::solana_program::program::invoke(
        &initialize_pointer_ix,
        &[
            ctx.accounts.ticket_mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
        ],
    )?;
    
    // Step 4: Initialize the mint cpi
    anchor_spl::token_2022::initialize_mint(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token_2022::InitializeMint {
                mint: ctx.accounts.ticket_mint.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
        ),
        0, // decimals (NFTs have 0 decimals)
        &ctx.accounts.mint_authority.key(), // mint authority PDA
        None, // freeze authority
    )?;
    
    // Step 5: init metadata
    // Clone strings once for reuse in multiple operations
    let name = ctx.accounts.event.name.clone();
    let symbol = ctx.accounts.event.symbol.clone();
    let uri = ctx.accounts.event.metadata_uri.clone();
    
    msg!("Step 5: Initializing metadata with name: {}, symbol: {}, uri: {}", name, symbol, uri);
    
    // Initialize metadata using spl_token_metadata_interface
    let initialize_metadata_ix = metadata_instruction::initialize(
        &anchor_spl::token_2022::ID,
        &ctx.accounts.ticket_mint.key(),
        &ctx.accounts.mint_authority.key(),
        &ctx.accounts.ticket_mint.key(),
        &ctx.accounts.mint_authority.key(),
        name.clone(),
        symbol.clone(),
        uri.clone(),
    );

    anchor_lang::solana_program::program::invoke_signed(
        &initialize_metadata_ix,
        &[
            ctx.accounts.ticket_mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        signer_seeds,
    )?;
    
    // Step 6: Update ticket metadata (name, symbol, uri)
    msg!("Step 6: Updating ticket metadata with seat information");
    
    // Update name 
    let update_name_ix = metadata_instruction::update_field(
        &anchor_spl::token_2022::ID,
        &ctx.accounts.ticket_mint.key(),
        &ctx.accounts.mint_authority.key(),
        Field::Name,
        name.clone(),
    );
    anchor_lang::solana_program::program::invoke_signed(
        &update_name_ix,
        &[
            ctx.accounts.ticket_mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        signer_seeds,
    )?;
    
    // Update symbol 
    let update_symbol_ix = metadata_instruction::update_field(
        &anchor_spl::token_2022::ID,
        &ctx.accounts.ticket_mint.key(),
        &ctx.accounts.mint_authority.key(),
        Field::Symbol,
        symbol.clone(),
    );
    anchor_lang::solana_program::program::invoke_signed(
        &update_symbol_ix,
        &[
            ctx.accounts.ticket_mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        signer_seeds,
    )?;
    
    // Update URI 
    let update_uri_ix = metadata_instruction::update_field(
        &anchor_spl::token_2022::ID,
        &ctx.accounts.ticket_mint.key(),
        &ctx.accounts.mint_authority.key(),
        Field::Uri,
        uri.clone(),
    );
    anchor_lang::solana_program::program::invoke_signed(
        &update_uri_ix,
        &[
            ctx.accounts.ticket_mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        signer_seeds,
    )?;
    
 
    // Step 7: Update metadata authority to merchant
    msg!("Step 7: Update metadata authority to merchant");
    
    // Update authority to merchant (using organizer as merchant)
    let new_authority = if ctx.accounts.event.organizer != Pubkey::default() {
        OptionalNonZeroPubkey::try_from(Some(ctx.accounts.event.organizer)).unwrap()
    } else {
        OptionalNonZeroPubkey::try_from(None).unwrap()
    };

    let update_authority_ix = metadata_instruction::update_authority(
        &anchor_spl::token_2022::ID,
        &ctx.accounts.ticket_mint.key(),
        &ctx.accounts.mint_authority.key(),
        new_authority,
    );

    anchor_lang::solana_program::program::invoke_signed(
        &update_authority_ix,
        &[
            ctx.accounts.ticket_mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        signer_seeds,
    )?;
    
    msg!("Step 7 completed: Updated metadata authority to merchant: {}", ctx.accounts.event.organizer);
    
    // Step 8: Create the associated token account
    anchor_spl::associated_token::create(
        CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: ctx.accounts.buyer.to_account_info(),
                associated_token: ctx.accounts.buyer_ticket_account.to_account_info(),
                authority: ctx.accounts.buyer.to_account_info(),
                mint: ctx.accounts.ticket_mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        ),
    )?;

    msg!("Step 8 completed: Created associated token account");
    
    msg!("Step 9: Minting one token to the associated token account of the buyer");
    // Step 9: Mint one token to the associated token account of the buyer
    let mint_to_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        anchor_spl::token_2022::MintTo {
            mint: ctx.accounts.ticket_mint.to_account_info(),
            to: ctx.accounts.buyer_ticket_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        },
        signer_seeds
    );
    anchor_spl::token_2022::mint_to(mint_to_ctx, 1)?; // Mint 1 NFT

    msg!("Step 9 completed: Minted one token to the associated token account of the buyer");
    
    msg!("Step 10: Closing mint authority");
    // Step 10: Close mint authority
    let set_authority_ix = spl_token_2022::instruction::set_authority(
        &anchor_spl::token_2022::ID,
        &ctx.accounts.ticket_mint.key(),
        None,
        spl_token_2022::instruction::AuthorityType::MintTokens,
        &ctx.accounts.mint_authority.key(),
        &[&ctx.accounts.mint_authority.key()],
    )?;

    anchor_lang::solana_program::program::invoke_signed(
        &set_authority_ix,
        &[
            ctx.accounts.ticket_mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        signer_seeds,
    )?;
    
    // Step 11: Mark ticket as minted
    ticket.is_checked_in = false; // Ticket is minted but not checked in yet
    
    // 9. Initialize Token 2022 extension fields for ticket data
    // TODO: Implement Token 2022 extension field creation
    // This would involve calling the Token 2022 program to create extension fields
    // For now, we log the extension data that would be stored
    msg!(
        "NFT minted with extension fields: seat={}:{}, event={:?}, uuid={:?}, price={}", 
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
    
    msg!("Ticket purchased: UUID {:?}", ticket.ticket_uuid);
    
    Ok(())
}

