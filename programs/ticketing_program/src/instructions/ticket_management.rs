use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::ErrorCode;

/// Create a new ticket type
#[derive(Accounts)]
#[instruction(event_id: String, type_id: String)]
pub struct CreateTicketType<'info> {
    #[account(
        seeds = [EventAccount::SEED_PREFIX, event_id.as_bytes()],
        bump = event.bump,
        constraint = event.organizer == organizer.key() @ ErrorCode::Unauthorized
    )]
    pub event: Account<'info, EventAccount>,
    
    #[account(
        init,
        payer = organizer,
        space = TicketTypeAccount::SIZE,
        seeds = [TicketTypeAccount::SEED_PREFIX, event_id.as_bytes(), type_id.as_bytes()],
        bump
    )]
    pub ticket_type: Account<'info, TicketTypeAccount>,
    
    #[account(mut)]
    pub organizer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn create_ticket_type(
    ctx: Context<CreateTicketType>,
    event_id: String,
    type_id: String,
    tier_name: String,
    price: u64,
    total_supply: u32,
    color: u32,
) -> Result<()> {
    let ticket_type = &mut ctx.accounts.ticket_type;
    
    ticket_type.event_id = event_id;
    ticket_type.type_id = type_id;
    ticket_type.tier_name = tier_name;
    ticket_type.price = price;
    ticket_type.total_supply = total_supply;
    ticket_type.minted = 0;
    ticket_type.color = color;
    ticket_type.bump = ctx.bumps.ticket_type;
    
    msg!("Ticket type created: {}", ticket_type.type_id);
    
    Ok(())
}

/// Batch mint tickets (lazy minting - just increase supply)
#[derive(Accounts)]
#[instruction(event_id: String, type_id: String)]
pub struct BatchMintTickets<'info> {
    #[account(
        seeds = [EventAccount::SEED_PREFIX, event_id.as_bytes()],
        bump = event.bump,
        constraint = event.organizer == organizer.key() @ ErrorCode::Unauthorized
    )]
    pub event: Account<'info, EventAccount>,
    
    #[account(
        mut,
        seeds = [TicketTypeAccount::SEED_PREFIX, event_id.as_bytes(), type_id.as_bytes()],
        bump = ticket_type.bump
    )]
    pub ticket_type: Account<'info, TicketTypeAccount>,
    
    pub organizer: Signer<'info>,
}

pub fn batch_mint_tickets(
    ctx: Context<BatchMintTickets>,
    _event_id: String,
    _type_id: String,
    quantity: u32,
) -> Result<()> {
    let ticket_type = &mut ctx.accounts.ticket_type;
    
    ticket_type.total_supply = ticket_type.total_supply
        .checked_add(quantity)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    msg!("Ticket supply increased by: {}", quantity);
    msg!("New total supply: {}", ticket_type.total_supply);
    
    Ok(())
}

