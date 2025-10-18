use anchor_lang::{ prelude::*, system_program };
use anchor_spl::{
    associated_token::{self, AssociatedToken},
    // --- Token-2022 Token Interface (NFT) ---
    token_interface::{self, Mint as MintInterface, TokenAccount as TokenAccountInterface, spl_pod::optional_keys::OptionalNonZeroPubkey},
    token_interface::Token2022, // Token-2022 Program
    token_2022,
};

use spl_token_2022::{
    extension::{
        ExtensionType, metadata_pointer::instruction::{initialize as initialize_metadata_pointer},
    },
    ID as TOKEN_2022_PROGRAM_ID,
    state::Mint as StateMint,
};
use spl_token_metadata_interface::{
    instruction::{initialize as initialize_metadata, update_field},
    state::Field,
};
use solana_program::{program::{invoke, invoke_signed}, pubkey::Pubkey};

declare_id!("tDdGYG37gZufntQqs7ZPuiSRyrceNP5ZdygqVQLjUGw");

// Constants
const MINT_AUTH_SEED: &[u8] = b"MINT_AUTH";
const TICKET_SEED: &[u8] = b"TICKET";
const EVENT_SEED: &[u8] = b"EVENT";
const PLATFORM_CONFIG_SEED: &[u8] = b"PLATFORM_CONFIG";
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
    ) -> Result<()> {
        let config = &mut ctx.accounts.platform_config;
        if ctx.accounts.payer.key() != ctx.accounts.admin.key() {
            return Err(ErrorCode::UnauthorizedAdmin.into());
        }
        config.platform_authority = platform_authority;
        config.bump = ctx.bumps.platform_config;

        emit!(PlatformConfigInitialized {
            platform_authority,
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

    /// mint a ticket NFT
    pub fn mint_ticket(
        ctx: Context<MintTicket>,
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
        // if accounts.ticket_mint.decimals != 0 || accounts.ticket_mint.supply != 0 {
        //     return Err(ErrorCode::MintNotInitialized.into());
        // }
        if accounts.seat_account.is_minted {
            return Err(ErrorCode::TicketAlreadyMinted.into());
        }
        if accounts.event.event_id != event_id {
            return Err(ErrorCode::InvalidEventId.into());
        }
        if accounts.platform_config.platform_authority != accounts.platform_authority.key() {
            return Err(ErrorCode::InvalidPlatformAuthority.into());
        }

        // Get PDA bump for mint authority
        let bump = ctx.bumps.mint_authority;
        let auth_seeds = &[MINT_AUTH_SEED, &[bump]];
        let signer_seeds = &[&auth_seeds[..]];

        let space = match
        ExtensionType::try_calculate_account_len::<StateMint>(&[ExtensionType::MetadataPointer])
        {
            Ok(space) => space,
            Err(_) => {
                return Err(ErrorCode::InvalidMintAccountSpace.into());
            }
        };

        // This is the space required for the metadata account.
        // We put the meta data into the mint account at the end so we
        // don't need to create and additional account.
        let meta_data_space = 250;

        let lamports_required = Rent::get()?.minimum_balance(space + meta_data_space);

        msg!(
            "Create Mint and metadata account size and cost: {} lamports: {}",
            space as u64,
            lamports_required
        );

        //
        system_program::create_account(
            CpiContext::new(
                accounts.token_program.to_account_info(),
                system_program::CreateAccount {
                    from: accounts.user.to_account_info(),
                    to: accounts.ticket_mint.to_account_info(),
                }
            ),
            lamports_required,
            space as u64,
            &accounts.token_program.key()
        )?;


        // Assign the mint to the token program
        system_program::assign(
            CpiContext::new(accounts.token_program.to_account_info(), system_program::Assign {
                account_to_assign: accounts.ticket_mint.to_account_info(),
            }),
            &TOKEN_2022_PROGRAM_ID
        )?;

        // Initialize the metadata pointer (Need to do this before initializing the mint)
        let initialize_pointer_ix = spl_token_2022::extension::metadata_pointer::instruction::initialize(
            &TOKEN_2022_PROGRAM_ID,
            &accounts.ticket_mint.key(),
            Some(accounts.mint_authority.key()), // authority
            Some(accounts.ticket_mint.key()), // metadata_address
        )?;

        invoke(
            &initialize_pointer_ix,
            &[
                accounts.ticket_mint.to_account_info(),
                accounts.mint_authority.to_account_info()
            ]
        )?;

        // Initialize the mint cpi
        token_2022::initialize_mint2(
            CpiContext::new(
                accounts.token_program.to_account_info(),
                token_2022::InitializeMint2 {
                    mint: accounts.ticket_mint.to_account_info(),
                }
            ),
            0,                                      // decimals
            &accounts.mint_authority.key(),         // mint_authority
            None                                    // freeze_authority
        ).unwrap();

        // init metadata
        let initialize_metadata_ix = initialize_metadata(
            &TOKEN_2022_PROGRAM_ID,
            &accounts.ticket_mint.key(),
            &accounts.mint_authority.key(),
            &accounts.ticket_mint.key(),
            &accounts.mint_authority.key(),
            accounts.event.name.clone(),
            accounts.event.symbol.clone(),
            accounts.event.uri.clone(),
        );

        invoke_signed(
            &initialize_metadata_ix,
            &[
                accounts.ticket_mint.to_account_info(),
                accounts.mint_authority.to_account_info(),
                accounts.token_program.to_account_info(),
            ],
            signer_seeds,
        )?;


        // NFT MINTING
        token_2022::mint_to(
            CpiContext::new_with_signer(
                accounts.token_program.to_account_info(),
                token_2022::MintTo {
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
                accounts.token_program.to_account_info(),
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
                accounts.token_program.to_account_info(),
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
                accounts.token_program.to_account_info(),
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
                accounts.token_program.to_account_info(),
            ],
            signer_seeds,
        )?;

        // Update authority to merchant
        let new_authority = if accounts.event.merchant_key != Pubkey::default() {
            OptionalNonZeroPubkey::try_from(Some(accounts.event.merchant_key)).unwrap()
        } else {
            OptionalNonZeroPubkey::try_from(None).unwrap()
        };

        let update_authority_ix = spl_token_metadata_interface::instruction::update_authority(
            &TOKEN_2022_PROGRAM_ID,
            &accounts.ticket_mint.key(),
            &accounts.mint_authority.key(),
            new_authority,
        );

        invoke_signed(
            &update_authority_ix,
            &[
                accounts.ticket_mint.to_account_info(),
                accounts.mint_authority.to_account_info(),
                accounts.token_program.to_account_info(),
            ],
            signer_seeds,
        )?;

        // Create the associated token account
        associated_token::create(
            CpiContext::new(
                accounts.associated_token_program.to_account_info(),
                associated_token::Create {
                    payer: accounts.user.to_account_info(),
                    associated_token: accounts.user_nft_ata.to_account_info(),
                    authority: accounts.user.to_account_info(),
                    mint: accounts.ticket_mint.to_account_info(),
                    system_program: accounts.system_program.to_account_info(),
                    token_program: accounts.token_program.to_account_info(),
                }
            )
        )?;

        // Mint one token to the associated token account of the player
        token_2022::mint_to(
            CpiContext::new_with_signer(
                accounts.token_program.to_account_info(),
                token_2022::MintTo {
                    mint: accounts.ticket_mint.to_account_info(),
                    to: accounts.user_nft_ata.to_account_info(),
                    authority: accounts.mint_authority.to_account_info(),
                },
                signer_seeds
            ),
            1
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
            merchant: accounts.event.merchant_key,
            seat_number: seat_number.clone(),
            name: accounts.event.name.clone(),
            symbol: accounts.event.symbol.clone(),
            uri: accounts.event.uri.clone(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!("NFT Minted successfully for {}", accounts.user.key());
        msg!("Ticket ID: {}, Event ID: {}, Seat: {}, Name: {}, Symbol: {}, URI: {}, Merchant: {}",
             ticket_id, event_id, seat_number, accounts.event.name, accounts.event.symbol, accounts.event.uri, accounts.event.merchant_key);

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
#[instruction(ticket_id: String, event_id: String, seat_number: String)]
pub struct MintTicket<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub platform_authority: Signer<'info>,

    /// CHECK
    #[account(mut)]
    pub ticket_mint: Signer<'info>,
    // #[account(
    //     init_if_needed,
    //     payer = user,
    //     associated_token::mint = ticket_mint,
    //     associated_token::authority = user
    // )]
    /// CHECK
    #[account(mut)]
    pub user_nft_ata: AccountInfo<'info>,
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

    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(ticket_id: String, event_id: String)]
pub struct ScanTicket<'info> {
    #[account(mut)]
    pub merchant: Signer<'info>,
    #[account(
        mut,
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
    pub mint: Box<InterfaceAccount<'info, MintInterface>>,
    /// CHECK: Validate address by deriving pda
    #[account(seeds = [MINT_AUTH_SEED], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        mut,
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
    pub token_program: Program<'info, Token2022>,
}

// DATA STRUCTURES
#[account]
pub struct PlatformConfig {
    pub platform_authority: Pubkey,
    pub bump: u8,
}

impl PlatformConfig {
    pub const LEN: usize = 32 + 1; // Pubkey (32) + u8 (1)
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


// ERROR CODES
#[error_code]
pub enum ErrorCode {
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
    #[msg("Invalid Mint account space")]
    InvalidMintAccountSpace,
}