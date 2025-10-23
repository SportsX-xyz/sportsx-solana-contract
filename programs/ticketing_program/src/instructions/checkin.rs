use anchor_lang::prelude::*;
// Token 2022 extension fields will be implemented later
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
    
    /// CHECK: Ticket NFT mint address
    #[account(mut)]
    pub ticket_mint: AccountInfo<'info>,
    
    /// CHECK: Ticket owner's NFT token account
    #[account(mut)]
    pub ticket_owner_token_account: AccountInfo<'info>,
    
    pub operator: Signer<'info>,
    
    /// Ticket authority PDA for signing PoF CPI calls
    #[account(
        seeds = [TicketAuthority::SEED_PREFIX],
        bump = ticket_authority.bump
    )]
    pub ticket_authority: Account<'info, TicketAuthority>,
    
    /// CHECK: Token program (supports both SPL Token and Token 2022)
    pub token_program: AccountInfo<'info>,
    
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
    
    // Mark ticket as checked in (both in PDA and NFT extension field)
    let ticket = &mut ctx.accounts.ticket;
    ticket.is_checked_in = true;
    
    // Update Token 2022 extension field for check-in status
    // TODO: Implement Token 2022 extension field update
    // The extension field would store:
    // - check_in_status: bool (update to true)
    // - check_in_time: i64 (current timestamp)
    // - check_in_operator: Pubkey (operator who checked in)
    
    // For now, we keep the PDA field for compatibility
    // Future implementation would involve calling Token 2022 program to update extension fields
    msg!(
        "Ticket marked as checked in (PDA field updated, Token 2022 extension field TODO) - Operator: {}, Time: {}", 
        ctx.accounts.operator.key(),
        current_time
    );
    
    // CPI to PoF program to add check-in points
    // Rule: +100 points for checking in
    // remaining_accounts: [0] ticket_owner_wallet_points, [1] pof_global_state, [2] pof_program
    if ctx.remaining_accounts.len() >= 3 {
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
            100, // +100 points for check-in
            signer_seeds,
        ) {
            Ok(_) => msg!("PoF points added: +100 (check-in)"),
            Err(e) => msg!("PoF update failed (non-critical): {:?}", e),
        }
    }
    
    msg!("Ticket checked in successfully");
    
    Ok(())
}

