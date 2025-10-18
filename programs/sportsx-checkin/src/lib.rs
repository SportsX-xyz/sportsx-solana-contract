use anchor_lang::prelude::*;
use sportsx_pof::program::SportsxPof;
use sportsx_pof::{self, WalletPoints, GlobalState};

declare_id!("2ZH4YcsqZTSKY1iAMwPMZUN6rSvTBhSCvso9pUWD9eXX");

/// Number of seconds in 24 hours
const CHECKIN_INTERVAL: i64 = 24 * 60 * 60; // 86400 seconds

/// Points awarded per check-in
const CHECKIN_POINTS: i64 = 10;

#[program]
pub mod sportsx_checkin {
    use super::*;

    /// Initialize a user's check-in record
    pub fn initialize_checkin(ctx: Context<InitializeCheckin>) -> Result<()> {
        let checkin_record = &mut ctx.accounts.checkin_record;
        checkin_record.wallet = ctx.accounts.wallet.key();
        checkin_record.last_checkin = 0; // Never checked in
        checkin_record.total_checkins = 0;
        checkin_record.bump = ctx.bumps.checkin_record;
        
        msg!("Check-in record initialized for wallet: {}", checkin_record.wallet);
        Ok(())
    }

    /// Perform daily check-in and award points via PoF contract
    pub fn daily_checkin(ctx: Context<DailyCheckin>) -> Result<()> {
        let checkin_record = &mut ctx.accounts.checkin_record;
        let current_time = Clock::get()?.unix_timestamp;

        // Check if 24 hours have passed since last check-in
        let time_since_last = current_time - checkin_record.last_checkin;
        require!(
            time_since_last >= CHECKIN_INTERVAL || checkin_record.last_checkin == 0,
            ErrorCode::CheckinTooSoon
        );

        // Update check-in record
        checkin_record.last_checkin = current_time;
        checkin_record.total_checkins += 1;

        msg!(
            "Check-in successful for wallet: {}, Total check-ins: {}", 
            checkin_record.wallet,
            checkin_record.total_checkins
        );

        // Call PoF contract to award points via CPI
        let cpi_program = ctx.accounts.pof_program.to_account_info();
        let cpi_accounts = sportsx_pof::cpi::accounts::UpdatePoints {
            wallet_points: ctx.accounts.wallet_points.to_account_info(),
            global_state: ctx.accounts.global_state.to_account_info(),
            authority: ctx.accounts.checkin_authority.to_account_info(),
        };

        // Create seeds for PDA signing (unified authority)
        let seeds = &[
            b"checkin_authority".as_ref(),
            &[ctx.bumps.checkin_authority],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            cpi_program,
            cpi_accounts,
            signer_seeds,
        );

        // Award points
        sportsx_pof::cpi::update_points(cpi_ctx, CHECKIN_POINTS)?;

        msg!("Awarded {} points via PoF contract", CHECKIN_POINTS);
        Ok(())
    }

    /// Get check-in information
    pub fn get_checkin_info(ctx: Context<GetCheckinInfo>) -> Result<CheckinInfo> {
        let checkin_record = &ctx.accounts.checkin_record;
        let current_time = Clock::get()?.unix_timestamp;
        let time_since_last = current_time - checkin_record.last_checkin;
        let can_checkin = time_since_last >= CHECKIN_INTERVAL || checkin_record.last_checkin == 0;
        let time_until_next = if can_checkin {
            0
        } else {
            CHECKIN_INTERVAL - time_since_last
        };

        Ok(CheckinInfo {
            last_checkin: checkin_record.last_checkin,
            total_checkins: checkin_record.total_checkins,
            can_checkin,
            time_until_next_checkin: time_until_next,
        })
    }
}

#[derive(Accounts)]
pub struct InitializeCheckin<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + CheckinRecord::INIT_SPACE,
        seeds = [b"checkin_record", wallet.key().as_ref()],
        bump
    )]
    pub checkin_record: Account<'info, CheckinRecord>,
    /// CHECK: The wallet we're tracking check-ins for
    pub wallet: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DailyCheckin<'info> {
    #[account(
        mut,
        seeds = [b"checkin_record", wallet.key().as_ref()],
        bump = checkin_record.bump
    )]
    pub checkin_record: Account<'info, CheckinRecord>,
    
    /// The wallet performing check-in
    pub wallet: Signer<'info>,
    
    /// Unified PDA authority for check-in contract to call PoF
    /// CHECK: PDA used for signing CPI calls
    #[account(
        seeds = [b"checkin_authority"],
        bump
    )]
    pub checkin_authority: AccountInfo<'info>,

    // PoF contract accounts
    #[account(mut)]
    pub wallet_points: Account<'info, WalletPoints>,
    
    #[account()]
    pub global_state: Account<'info, GlobalState>,
    
    pub pof_program: Program<'info, SportsxPof>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetCheckinInfo<'info> {
    #[account(
        seeds = [b"checkin_record", checkin_record.wallet.as_ref()],
        bump = checkin_record.bump
    )]
    pub checkin_record: Account<'info, CheckinRecord>,
}

#[account]
#[derive(InitSpace)]
pub struct CheckinRecord {
    pub wallet: Pubkey,
    pub last_checkin: i64,      // Unix timestamp
    pub total_checkins: u64,     // Total number of check-ins
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CheckinInfo {
    pub last_checkin: i64,
    pub total_checkins: u64,
    pub can_checkin: bool,
    pub time_until_next_checkin: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Check-in too soon. Must wait 24 hours between check-ins")]
    CheckinTooSoon,
}

