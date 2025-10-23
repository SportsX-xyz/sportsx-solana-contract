use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::ErrorCode;

/// Create a new event
#[derive(Accounts)]
#[instruction(event_id: String)]
pub struct CreateEvent<'info> {
    #[account(
        seeds = [PlatformConfig::SEED_PREFIX],
        bump = platform_config.bump
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    
    #[account(
        init,
        payer = organizer,
        space = EventAccount::SIZE,
        seeds = [EventAccount::SEED_PREFIX, event_id.as_bytes()],
        bump
    )]
    pub event: Account<'info, EventAccount>,
    
    #[account(
        mut,
        constraint = organizer.key() == platform_config.event_admin @ ErrorCode::Unauthorized
    )]
    pub organizer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn create_event(
    ctx: Context<CreateEvent>,
    event_id: String,
    name: String,
    symbol: String,
    metadata_uri: String,
    start_time: i64,
    end_time: i64,
    ticket_release_time: i64,
    stop_sale_before: i64,
    resale_fee_rate: u16,
    max_resale_times: u8,
) -> Result<()> {
    let event = &mut ctx.accounts.event;
    
    event.event_id = event_id;
    event.name = name;
    event.symbol = symbol;
    event.organizer = ctx.accounts.organizer.key();
    event.metadata_uri = metadata_uri;
    event.start_time = start_time;
    event.end_time = end_time;
    event.ticket_release_time = ticket_release_time;
    event.stop_sale_before = stop_sale_before;
    event.resale_fee_rate = resale_fee_rate;
    event.max_resale_times = max_resale_times;
    event.status = 1; // Active by default
    event.bump = ctx.bumps.event;
    
    msg!("Event created in Active status: {}", event.event_id);
    
    Ok(())
}

/// Update event status
#[derive(Accounts)]
#[instruction(event_id: String)]
pub struct UpdateEventStatus<'info> {
    #[account(
        mut,
        seeds = [EventAccount::SEED_PREFIX, event_id.as_bytes()],
        bump = event.bump,
        constraint = event.organizer == organizer.key() @ ErrorCode::Unauthorized
    )]
    pub event: Account<'info, EventAccount>,
    
    pub organizer: Signer<'info>,
}

pub fn update_event_status(
    ctx: Context<UpdateEventStatus>,
    _event_id: String,
    new_status: u8,
) -> Result<()> {
    require!(new_status <= 2, ErrorCode::InvalidEventStatus);
    
    let event = &mut ctx.accounts.event;
    event.status = new_status;
    
    msg!("Event status updated to: {}", new_status);
    
    Ok(())
}

/// Add check-in operator for an event
#[derive(Accounts)]
#[instruction(event_id: String, operator: Pubkey)]
pub struct AddCheckInOperator<'info> {
    #[account(
        seeds = [PlatformConfig::SEED_PREFIX],
        bump = platform_config.bump
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    
    #[account(
        seeds = [EventAccount::SEED_PREFIX, event_id.as_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, EventAccount>,
    
    #[account(
        init,
        payer = admin,
        space = CheckInAuthority::SIZE,
        seeds = [CheckInAuthority::SEED_PREFIX, event_id.as_bytes(), operator.as_ref()],
        bump
    )]
    pub checkin_authority: Account<'info, CheckInAuthority>,
    
    #[account(
        mut,
        constraint = admin.key() == platform_config.event_admin @ ErrorCode::Unauthorized
    )]
    pub admin: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn add_checkin_operator(
    ctx: Context<AddCheckInOperator>,
    event_id: String,
    operator: Pubkey,
) -> Result<()> {
    let checkin_authority = &mut ctx.accounts.checkin_authority;
    
    checkin_authority.event_id = event_id;
    checkin_authority.operator = operator;
    checkin_authority.is_active = true;
    checkin_authority.bump = ctx.bumps.checkin_authority;
    
    msg!("Check-in operator added: {}", operator);
    
    Ok(())
}

/// Remove check-in operator for an event
#[derive(Accounts)]
#[instruction(event_id: String, operator: Pubkey)]
pub struct RemoveCheckInOperator<'info> {
    #[account(
        seeds = [PlatformConfig::SEED_PREFIX],
        bump = platform_config.bump
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    
    #[account(
        seeds = [EventAccount::SEED_PREFIX, event_id.as_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, EventAccount>,
    
    #[account(
        mut,
        seeds = [CheckInAuthority::SEED_PREFIX, event_id.as_bytes(), operator.as_ref()],
        bump = checkin_authority.bump
    )]
    pub checkin_authority: Account<'info, CheckInAuthority>,
    
    #[account(
        constraint = admin.key() == platform_config.event_admin @ ErrorCode::Unauthorized
    )]
    pub admin: Signer<'info>,
}

pub fn remove_checkin_operator(
    ctx: Context<RemoveCheckInOperator>,
    _event_id: String,
    _operator: Pubkey,
) -> Result<()> {
    let checkin_authority = &mut ctx.accounts.checkin_authority;
    checkin_authority.is_active = false;
    
    msg!("Check-in operator removed");
    
    Ok(())
}

