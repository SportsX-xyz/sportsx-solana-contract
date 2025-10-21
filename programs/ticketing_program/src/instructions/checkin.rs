use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::ErrorCode;

/// Check-in a ticket
#[derive(Accounts)]
#[instruction(event_id: String)]
pub struct CheckInTicket<'info> {
    #[account(
        seeds = [EventAccount::SEED_PREFIX, event_id.as_bytes()],
        bump = event.bump
    )]
    pub event: Account<'info, EventAccount>,
    
    #[account(
        seeds = [CheckInAuthority::SEED_PREFIX, event_id.as_bytes(), operator.key().as_ref()],
        bump = checkin_authority.bump,
        constraint = checkin_authority.is_active @ ErrorCode::CheckInOperatorNotAuthorized
    )]
    pub checkin_authority: Account<'info, CheckInAuthority>,
    
    #[account(
        mut,
        constraint = !ticket.is_checked_in @ ErrorCode::AlreadyCheckedIn
    )]
    pub ticket: Account<'info, TicketAccount>,
    
    pub operator: Signer<'info>,
    
    // PoF integration: pass as remaining_accounts in order:
    // [0] ticket_owner_pof_wallet (mut), [1] pof_global_state, [2] pof_program
}

pub fn check_in_ticket<'info>(
    ctx: Context<'_, '_, '_, 'info, CheckInTicket<'info>>,
    _event_id: String,
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // Verify check-in time window
    // Allow check-in from some time before event start until event end
    let check_in_window_start = ctx.accounts.event.start_time - 3600; // 1 hour before
    let check_in_window_end = ctx.accounts.event.end_time;
    
    require!(
        current_time >= check_in_window_start && current_time <= check_in_window_end,
        ErrorCode::InvalidCheckInTime
    );
    
    // Mark ticket as checked in
    let ticket = &mut ctx.accounts.ticket;
    ticket.is_checked_in = true;
    
    // CPI to PoF program to add check-in points
    // Rule: +100 points for checking in
    if ctx.remaining_accounts.len() >= 3 {
        match crate::instructions::update_pof_points(
            &ctx.remaining_accounts[0],
            &ctx.remaining_accounts[1],
            &ctx.accounts.operator.to_account_info(),
            &ctx.remaining_accounts[2],
            100, // +100 points for check-in
            None,
        ) {
            Ok(_) => msg!("PoF points added: +100 for check-in"),
            Err(e) => msg!("PoF update failed (non-critical): {:?}", e),
        }
    }
    
    msg!("Ticket checked in successfully");
    
    Ok(())
}

