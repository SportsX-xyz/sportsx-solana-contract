use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, transfer};
use spl_token_2022::{
    extension::{ExtensionType, metadata_pointer::instruction::{initialize as initialize_metadata_pointer}},
    ID as TOKEN_2022_PROGRAM_ID
};
use spl_token_metadata_interface::{
    instruction::{initialize as initialize_metadata, update_field},
    state::Field,
};
use solana_program::{program::invoke_signed, pubkey::Pubkey, hash::hash};

declare_id!("tDdGYG37gZufntQqs7ZPuiSRyrceNP5ZdygqVQLjUGw");

// Constants
const MINT_AUTH_SEED: &[u8] = b"MINT_AUTH";
const TICKET_SEED: &[u8] = b"TICKET";
const EVENT_SEED: &[u8] = b"EVENT";
const PLATFORM_CONFIG_SEED: &[u8] = b"PLATFORM_CONFIG";
const PLATFORM_MINT_FEE: u64 = 100000; // $0.10 USDT (6 decimals)
const MAX_TICKET_ID_LENGTH: usize = 32;
const MAX_EVENT_ID_LENGTH: usize = 32;
const MAX_URI_LENGTH: usize = 200;
const MAX_NAME_LENGTH: usize = 32;
const MAX_SYMBOL_LENGTH: usize = 10;
const MAX_SEAT_NUMBER_LENGTH: usize = 10;

#[program]
pub mod ticketing_program {
    use super::*;

    /// Initialize platform configuration
    pub fn initialize_platform_config(
        ctx: Context<InitializePlatformConfig>,
        platform_authority: Pubkey,
        usdt_mint: Pubkey,
    ) -> Result<()> {
        let config = &mut ctx.accounts.platform_config;
        if ctx.accounts.payer.key() != ctx.accounts.admin.key() {
            return Err(ErrorCode::UnauthorizedAdmin.into());
        }
        config.platform_authority = platform_authority;
        config.usdt_mint = usdt_mint;
        config.bump = ctx.bumps.platform_config;

        emit!(PlatformConfigInitialized {
            platform_authority,
            usdt_mint,
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }

    /// Create an event and initialize NFT mint
    pub fn create_event(
        ctx: Context<CreateEvent>,
        event_id: String,
        uri: String,
        merchant_key: Pubkey,
        name: String,
        symbol: String,
        expiry_timestamp: i64,
    ) -> Result<()> {
        let event = &mut ctx.accounts.event;
        if ctx.accounts.platform_authority.key() != ctx.accounts.platform_config.platform_authority {
            return Err(ErrorCode::InvalidPlatformAuthority.into());
        }
        if event_id.len() > MAX_EVENT_ID_LENGTH {
            return Err(ErrorCode::EventIdTooLong.into());
        }
        if uri.len() > MAX_URI_LENGTH || !uri.starts_with("https://") {
            return Err(ErrorCode::InvalidUri.into());
        }
        if name.len() > MAX_NAME_LENGTH {
            return Err(ErrorCode::NameTooLong.into());
        }
        if symbol.len() > MAX_SYMBOL_LENGTH {
            return Err(ErrorCode::SymbolTooLong.into());
        }
        if expiry_timestamp <= Clock::get()?.unix_timestamp {
            return Err(ErrorCode::InvalidExpiryTimestamp.into());
        }

        // Store event data
        event.event_id = event_id.clone();
        event.uri = uri.clone();
        event.merchant_key = merchant_key;
        event.name = name.clone();
        event.symbol = symbol.clone();
        event.expiry_timestamp = expiry_timestamp;
        event.bump = ctx.bumps.event;

        emit!(EventCreated {
            event_id,
            uri,
            merchant_key,
            name,
            symbol,
            expiry_timestamp,
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }

    /// Purchase and mint a ticket NFT
    pub fn purchase_and_mint(
        ctx: Context<PurchaseAndMint>,
        ticket_price_usdt: u64,
        ticket_id: String,
        event_id: String,
        seat_number: String,
    ) -> Result<()> {
        let accounts = ctx.accounts;

        // Validate inputs
        if ticket_id.len() > MAX_TICKET_ID_LENGTH {
            return Err(ErrorCode::TicketIdTooLong.into());
        }
        if event_id.len() > MAX_EVENT_ID_LENGTH {
            return Err(ErrorCode::EventIdTooLong.into());
        }
        if seat_number.len() > MAX_SEAT_NUMBER_LENGTH {
            return Err(ErrorCode::SeatNumberTooLong.into());
        }
        if accounts.usdt_mint.key() != accounts.platform_config.usdt_mint {
            return Err(ErrorCode::InvalidUsdtMint.into());
        }
        if accounts.ticket_mint.decimals != 0 || accounts.ticket_mint.supply != 0 {
            return Err(ErrorCode::MintNotInitialized.into());
        }
        if accounts.seat_account.is_minted {
            return Err(ErrorCode::TicketAlreadyMinted.into());
        }
        if accounts.event.event_id != event_id {
            return Err(ErrorCode::InvalidEventId.into());
        }
        if accounts.platform_config.platform_authority != accounts.platform_authority.key() {
            return Err(ErrorCode::InvalidPlatformAuthority.into());
        }
        if accounts.merchant_usdt_vault.owner != accounts.event.merchant_key {
            return Err(ErrorCode::InvalidMerchantAuthority.into());
        }

        // Calculate merchant revenue
        let merchant_revenue = ticket_price_usdt
            .checked_sub(PLATFORM_MINT_FEE)
            .ok_or(ErrorCode::InsufficientFunds)?;

        // Get PDA bump for mint authority
        let bump = ctx.bumps.mint_authority;
        let auth_seeds = &[MINT_AUTH_SEED, &[bump]];
        let signer_seeds = &[&auth_seeds[..]];

        // PHASE I: USDT PAYMENT
        let cpi_accounts_platform = Transfer {
            from: accounts.user_usdt_ata.to_account_info(),
            to: accounts.platform_usdt_vault.to_account_info(),
            authority: accounts.user.to_account_info(),
        };
        let cpi_program = accounts.token_program.to_account_info();
        transfer(
            CpiContext::new(cpi_program.clone(), cpi_accounts_platform),
            PLATFORM_MINT_FEE,
        )?;

        let cpi_accounts_merchant = Transfer {
            from: accounts.user_usdt_ata.to_account_info(),
            to: accounts.merchant_usdt_vault.to_account_info(),
            authority: accounts.user.to_account_info(),
        };
        transfer(
            CpiContext::new(cpi_program, cpi_accounts_merchant),
            merchant_revenue,
        )?;

        // PHASE II: NFT MINTING
        token::mint_to(
            CpiContext::new_with_signer(
                accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: accounts.ticket_mint.to_account_info(),
                    to: accounts.user_nft_ata.to_account_info(),
                    authority: accounts.mint_authority.to_account_info(),
                },
                signer_seeds,
            ),
            1,
        )?;

        // Update ticket metadata (name, symbol, uri, seat_number)
        let update_name_ix = update_field(
            &TOKEN_2022_PROGRAM_ID,
            &accounts.ticket_mint.key(),
            &accounts.mint_authority.key(),
            Field::Name,
            accounts.event.name.clone(),
        );
        invoke_signed(
            &update_name_ix,
            &[
                accounts.ticket_mint.to_account_info(),
                accounts.mint_authority.to_account_info(),
            ],
            signer_seeds,
        )?;

        let update_symbol_ix = update_field(
            &TOKEN_2022_PROGRAM_ID,
            &accounts.ticket_mint.key(),
            &accounts.mint_authority.key(),
            Field::Symbol,
            accounts.event.symbol.clone(),
        );
        invoke_signed(
            &update_symbol_ix,
            &[
                accounts.ticket_mint.to_account_info(),
                accounts.mint_authority.to_account_info(),
            ],
            signer_seeds,
        )?;

        let update_uri_ix = update_field(
            &TOKEN_2022_PROGRAM_ID,
            &accounts.ticket_mint.key(),
            &accounts.mint_authority.key(),
            Field::Uri,
            accounts.event.uri.clone(),
        );
        invoke_signed(
            &update_uri_ix,
            &[
                accounts.ticket_mint.to_account_info(),
                accounts.mint_authority.to_account_info(),
            ],
            signer_seeds,
        )?;

        let update_seat_number_ix = update_field(
            &TOKEN_2022_PROGRAM_ID,
            &accounts.ticket_mint.key(),
            &accounts.mint_authority.key(),
            Field::Key("seat_number".to_string()),
            seat_number.clone(),
        );
        invoke_signed(
            &update_seat_number_ix,
            &[
                accounts.ticket_mint.to_account_info(),
                accounts.mint_authority.to_account_info(),
            ],
            signer_seeds,
        )?;

        // Close mint authority
        let set_authority_ix = spl_token_2022::instruction::set_authority(
            &TOKEN_2022_PROGRAM_ID,
            &accounts.ticket_mint.key(),
            None,
            spl_token_2022::instruction::AuthorityType::MintTokens,
            &accounts.mint_authority.key(),
            &[&accounts.mint_authority.key()],
        )?;
        invoke_signed(
            &set_authority_ix,
            &[
                accounts.ticket_mint.to_account_info(),
                accounts.mint_authority.to_account_info(),
                accounts.token_program.to_account_info(),
            ],
            signer_seeds,
        )?;

        // Mark ticket as minted
        accounts.seat_account.is_minted = true;
        accounts.seat_account.bump = ctx.bumps.seat_account;

        // Emit event
        emit!(TicketMinted {
            user: accounts.user.key(),
            mint: accounts.ticket_mint.key(),
            ticket_id: ticket_id.clone(),
            event_id: event_id.clone(),
            ticket_price: ticket_price_usdt,
            merchant: accounts.event.merchant_key,
            seat_number: seat_number.clone(),
            name: accounts.event.name.clone(),
            symbol: accounts.event.symbol.clone(),
            uri: accounts.event.uri.clone(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!("NFT Minted successfully for {}", accounts.user.key());
        msg!("Ticket ID: {}, Event ID: {}, Seat: {}, Name: {}, Symbol: {}, URI: {}, Price: {}, Merchant: {}, Revenue: {}", 
             ticket_id, event_id, seat_number, accounts.event.name, accounts.event.symbol, accounts.event.uri, ticket_price_usdt, accounts.event.merchant_key, merchant_revenue);

        Ok(())
    }

    /// Scan a ticket
    pub fn scan_ticket(
        ctx: Context<ScanTicket>,
        ticket_id: String,
        event_id: String,
    ) -> Result<()> {
        let accounts = ctx.accounts;

        // Validate inputs
        if ticket_id.len() > MAX_TICKET_ID_LENGTH {
            return Err(ErrorCode::TicketIdTooLong.into());
        }
        if event_id.len() > MAX_EVENT_ID_LENGTH {
            return Err(ErrorCode::EventIdTooLong.into());
        }
        if accounts.event.event_id != event_id {
            return Err(ErrorCode::InvalidEventId.into());
        }
        if accounts.merchant.key() != accounts.event.merchant_key {
            return Err(ErrorCode::InvalidMerchantAuthority.into());
        }
        if Clock::get()?.unix_timestamp >= accounts.event.expiry_timestamp {
            return Err(ErrorCode::EventExpired.into());
        }
        if !accounts.seat_account.is_minted {
            return Err(ErrorCode::TicketNotMinted.into());
        }
        if accounts.seat_account.is_scanned {
            return Err(ErrorCode::TicketAlreadyScanned.into());
        }

        // Mark ticket as scanned
        accounts.seat_account.is_scanned = true;

        emit!(TicketScanned {
            ticket_id: ticket_id.clone(),
            event_id: event_id.clone(),
            merchant: accounts.merchant.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!("Ticket ID: {}, Event ID: {}, Scanned by: {}", 
             ticket_id, event_id, accounts.merchant.key());

        Ok(())
    }

    /// Update seat number
    pub fn update_seat_number(
        ctx: Context<UpdateSeatNumber>,
        ticket_id: String,
        event_id: String,
        new_seat_number: String,
    ) -> Result<()> {
        let accounts = ctx.accounts;

        // Validate inputs
        if ticket_id.len() > MAX_TICKET_ID_LENGTH {
            return Err(ErrorCode::TicketIdTooLong.into());
        }
        if event_id.len() > MAX_EVENT_ID_LENGTH {
            return Err(ErrorCode::EventIdTooLong.into());
        }
        if new_seat_number.len() > MAX_SEAT_NUMBER_LENGTH {
            return Err(ErrorCode::SeatNumberTooLong.into());
        }
        if accounts.event.event_id != event_id {
            return Err(ErrorCode::InvalidEventId.into());
        }
        if accounts.merchant.key() != accounts.event.merchant_key {
            return Err(ErrorCode::InvalidMerchantAuthority.into());
        }
        // if accounts.mint.key() != accounts.event.mint {
        //     return Err(ErrorCode::InvalidMint.into());
        // }
        if !accounts.seat_account.is_minted {
            return Err(ErrorCode::TicketNotMinted.into());
        }

        // Update seat number in additionalMetadata
        let bump = ctx.bumps.mint_authority;
        let auth_seeds = &[MINT_AUTH_SEED, &[bump]];
        let signer_seeds = &[&auth_seeds[..]];

        let update_metadata_ix = update_field(
            &TOKEN_2022_PROGRAM_ID,
            &accounts.mint.key(),
            &accounts.mint_authority.key(),
            Field::Key("seat_number".to_string()),
            new_seat_number.clone(),
        );
        invoke_signed(
            &update_metadata_ix,
            &[
                accounts.mint.to_account_info(),
                accounts.mint_authority.to_account_info(),
            ],
            signer_seeds,
        )?;

        emit!(SeatNumberUpdated {
            ticket_id: ticket_id.clone(),
            event_id: event_id.clone(),
            new_seat_number: new_seat_number.clone(),
            merchant: accounts.merchant.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!("Ticket ID: {}, Event ID: {}, New Seat: {}, Updated by: {}", 
             ticket_id, event_id, new_seat_number, accounts.merchant.key());

        Ok(())
    }

    /// Query ticket status
    pub fn query_ticket_status(
        ctx: Context<QueryTicketStatus>,
        ticket_id: String,
        event_id: String,
    ) -> Result<()> {
        let accounts = ctx.accounts;

        // Validate inputs
        if ticket_id.len() > MAX_TICKET_ID_LENGTH {
            return Err(ErrorCode::TicketIdTooLong.into());
        }
        if event_id.len() > MAX_EVENT_ID_LENGTH {
            return Err(ErrorCode::EventIdTooLong.into());
        }
        if accounts.event.event_id != event_id {
            return Err(ErrorCode::InvalidEventId.into());
        }

        // Emit event with ticket status
        emit!(TicketStatusQueried {
            ticket_id: ticket_id.clone(),
            event_id: event_id.clone(),
            uri: accounts.event.uri.clone(),
            is_minted: accounts.seat_account.is_minted,
            is_scanned: accounts.seat_account.is_scanned,
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!("Ticket ID: {}, Event ID: {}, URI: {}, Is Minted: {}, Is Scanned: {}", 
             ticket_id, event_id, accounts.event.uri, accounts.seat_account.is_minted, accounts.seat_account.is_scanned);

        Ok(())
    }
}

// ACCOUNT STRUCTURES
#[derive(Accounts)]
pub struct InitializePlatformConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + PlatformConfig::LEN,
        seeds = [PLATFORM_CONFIG_SEED],
        bump
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(event_id: String, uri: String, merchant_key: Pubkey, name: String, symbol: String)]
pub struct CreateEvent<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + Events::LEN,
        seeds = [EVENT_SEED, event_id.as_bytes()],
        bump
    )]
    pub event: Account<'info, Events>,
    #[account(
        seeds = [PLATFORM_CONFIG_SEED],
        bump = platform_config.bump
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub platform_authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(ticket_price_usdt: u64, ticket_id: String, event_id: String, seat_number: String)]
pub struct PurchaseAndMint<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub platform_authority: Signer<'info>,
    pub usdt_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        token::mint = usdt_mint,
        token::authority = user
    )]
    pub user_usdt_ata: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint = usdt_mint,
        token::authority = platform_authority
    )]
    pub platform_usdt_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint = usdt_mint,
        token::authority = event.merchant_key
    )]
    pub merchant_usdt_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = user,
        mint::decimals = 0,
        mint::authority = mint_authority,
        owner = token_program.key()
    )]
    pub ticket_mint: Box<Account<'info, Mint>>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = ticket_mint,
        associated_token::authority = user
    )]
    pub user_nft_ata: Box<Account<'info, TokenAccount>>,
    /// CHECK: Validate address by deriving pda
    #[account(seeds = [MINT_AUTH_SEED], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = user,
        space = 8 + SeatStatus::LEN,
        seeds = [TICKET_SEED, ticket_id.as_bytes(), event.key().as_ref()],
        bump
    )]
    pub seat_account: Account<'info, SeatStatus>,
    #[account(
        seeds = [EVENT_SEED, event_id.as_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Events>,
    #[account(
        seeds = [PLATFORM_CONFIG_SEED],
        bump = platform_config.bump,
        constraint = platform_config.platform_authority == platform_authority.key() @ ErrorCode::InvalidPlatformAuthority
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    pub system_program: Program<'info, System>,
    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(ticket_id: String, event_id: String)]
pub struct ScanTicket<'info> {
    #[account(mut)]
    pub merchant: Signer<'info>,
    #[account(
        seeds = [TICKET_SEED, ticket_id.as_bytes(), event.key().as_ref()],
        bump = seat_account.bump
    )]
    pub seat_account: Account<'info, SeatStatus>,
    #[account(
        seeds = [EVENT_SEED, event_id.as_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Events>,
}

#[derive(Accounts)]
#[instruction(ticket_id: String, event_id: String, new_seat_number: String)]
pub struct UpdateSeatNumber<'info> {
    #[account(mut)]
    pub merchant: Signer<'info>,
    #[account(
        mut,
        token::token_program = token_program
    )]
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: Validate address by deriving pda
    #[account(seeds = [MINT_AUTH_SEED], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [TICKET_SEED, ticket_id.as_bytes(), event.key().as_ref()],
        bump = seat_account.bump
    )]
    pub seat_account: Account<'info, SeatStatus>,
    #[account(
        seeds = [EVENT_SEED, event_id.as_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Events>,
    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(ticket_id: String, event_id: String)]
pub struct QueryTicketStatus<'info> {
    #[account(
        seeds = [TICKET_SEED, ticket_id.as_bytes(), event.key().as_ref()],
        bump
    )]
    pub seat_account: Account<'info, SeatStatus>,
    #[account(
        seeds = [EVENT_SEED, event_id.as_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, Events>,
}

// DATA STRUCTURES
#[account]
pub struct PlatformConfig {
    pub platform_authority: Pubkey,
    pub usdt_mint: Pubkey,
    pub bump: u8,
}

impl PlatformConfig {
    pub const LEN: usize = 32 + 32 + 1; // Pubkey (32) + Pubkey (32) + u8 (1)
}

#[account]
pub struct Events {
    pub event_id: String,
    pub uri: String,
    pub merchant_key: Pubkey,
    pub name: String,
    pub symbol: String,
    pub expiry_timestamp: i64,
    pub bump: u8,
}

impl Events {
    pub const LEN: usize = 32 + 200 + 32 + 32 + 10 + 8 + 1; // event_id (32) + uri (200) + merchant_key (32) + name (32) + symbol (10)  + expiry_timestamp (8) + bump (1)
}

#[account]
pub struct SeatStatus {
    pub is_minted: bool,
    pub is_scanned: bool,
    pub bump: u8,
}

impl SeatStatus {
    pub const LEN: usize = 1 + 1 + 1; // bool + bool + u8
}

// EVENTS
#[event]
pub struct PlatformConfigInitialized {
    pub platform_authority: Pubkey,
    pub usdt_mint: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct EventCreated {
    pub event_id: String,
    pub uri: String,
    pub merchant_key: Pubkey,
    pub name: String,
    pub symbol: String,
    pub expiry_timestamp: i64,
    pub timestamp: i64,
}

#[event]
pub struct TicketMinted {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub ticket_id: String,
    pub event_id: String,
    pub ticket_price: u64,
    pub merchant: Pubkey,
    pub seat_number: String,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub timestamp: i64,
}

#[event]
pub struct TicketScanned {
    pub ticket_id: String,
    pub event_id: String,
    pub merchant: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct SeatNumberUpdated {
    pub ticket_id: String,
    pub event_id: String,
    pub new_seat_number: String,
    pub merchant: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct TicketStatusQueried {
    pub ticket_id: String,
    pub event_id: String,
    pub uri: String,
    pub is_minted: bool,
    pub is_scanned: bool,
    pub timestamp: i64,
}

// ERROR CODES
#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient funds for ticket purchase")]
    InsufficientFunds,
    #[msg("Ticket ID exceeds maximum length")]
    TicketIdTooLong,
    #[msg("Event ID exceeds maximum length")]
    EventIdTooLong,
    #[msg("Seat number exceeds maximum length")]
    SeatNumberTooLong,
    #[msg("Name exceeds maximum length")]
    NameTooLong,
    #[msg("Symbol exceeds maximum length")]
    SymbolTooLong,
    #[msg("Mint account not properly initialized")]
    MintNotInitialized,
    #[msg("Invalid USDT mint provided")]
    InvalidUsdtMint,
    #[msg("Invalid URI format or length")]
    InvalidUri,
    #[msg("Token-2022 extension initialization failed")]
    ExtensionInitializationFailed,
    #[msg("Ticket already minted")]
    TicketAlreadyMinted,
    #[msg("Invalid event ID")]
    InvalidEventId,
    #[msg("Invalid platform authority")]
    InvalidPlatformAuthority,
    #[msg("Invalid merchant authority")]
    InvalidMerchantAuthority,
    #[msg("Unauthorized admin")]
    UnauthorizedAdmin,
    #[msg("Event has expired")]
    EventExpired,
    #[msg("Ticket not minted")]
    TicketNotMinted,
    #[msg("Ticket already scanned")]
    TicketAlreadyScanned,
    #[msg("Invalid expiry timestamp")]
    InvalidExpiryTimestamp,
    #[msg("Invalid mint account")]
    InvalidMint,
}