use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::ErrorCode;

/// Initialize platform configuration
#[derive(Accounts)]
pub struct InitializePlatform<'info> {
    #[account(
        init,
        payer = deployer,
        space = PlatformConfig::SIZE,
        seeds = [PlatformConfig::SEED_PREFIX],
        bump
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    
    #[account(
        init,
        payer = deployer,
        space = NonceTracker::SIZE,
        seeds = [NonceTracker::SEED_PREFIX],
        bump
    )]
    pub nonce_tracker: Account<'info, NonceTracker>,
    
    #[account(mut)]
    pub deployer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_platform(
    ctx: Context<InitializePlatform>,
    initial_fee_receiver: Pubkey,
    initial_fee_usdc: u64,
    backend_authority: Pubkey,
    event_admin: Pubkey,
) -> Result<()> {
    let config = &mut ctx.accounts.platform_config;
    
    config.fee_receiver = initial_fee_receiver;
    config.fee_amount_usdc = initial_fee_usdc;
    config.update_authority = ctx.accounts.deployer.key();
    config.backend_authority = backend_authority;
    config.event_admin = event_admin;
    config.is_paused = false;
    config.bump = ctx.bumps.platform_config;
    
    // Initialize nonce tracker (arrays default to zeros, which is fine)
    let nonce_tracker = &mut ctx.accounts.nonce_tracker;
    nonce_tracker.next_index = 0;
    
    msg!("Platform initialized with deployer as authority: {}", ctx.accounts.deployer.key());
    
    Ok(())
}

/// Update platform configuration
#[derive(Accounts)]
pub struct UpdatePlatformConfig<'info> {
    #[account(
        mut,
        seeds = [PlatformConfig::SEED_PREFIX],
        bump = platform_config.bump,
        constraint = platform_config.update_authority == authority.key() @ ErrorCode::Unauthorized
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    
    pub authority: Signer<'info>,
}

pub fn update_platform_config(
    ctx: Context<UpdatePlatformConfig>,
    new_fee_receiver: Option<Pubkey>,
    new_fee_usdc: Option<u64>,
    new_backend_authority: Option<Pubkey>,
    new_event_admin: Option<Pubkey>,
) -> Result<()> {
    let config = &mut ctx.accounts.platform_config;
    
    if let Some(receiver) = new_fee_receiver {
        config.fee_receiver = receiver;
        msg!("Fee receiver updated to: {}", receiver);
    }
    
    if let Some(fee) = new_fee_usdc {
        config.fee_amount_usdc = fee;
        msg!("Fee amount updated to: {}", fee);
    }
    
    if let Some(backend_auth) = new_backend_authority {
        config.backend_authority = backend_auth;
        msg!("Backend authority updated to: {}", backend_auth);
    }
    
    if let Some(admin) = new_event_admin {
        config.event_admin = admin;
        msg!("Event admin updated to: {}", admin);
    }
    
    Ok(())
}

/// Toggle platform pause status
#[derive(Accounts)]
pub struct TogglePause<'info> {
    #[account(
        mut,
        seeds = [PlatformConfig::SEED_PREFIX],
        bump = platform_config.bump,
        constraint = platform_config.update_authority == authority.key() @ ErrorCode::Unauthorized
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    
    pub authority: Signer<'info>,
}

pub fn toggle_pause(ctx: Context<TogglePause>) -> Result<()> {
    let config = &mut ctx.accounts.platform_config;
    config.is_paused = !config.is_paused;
    
    msg!("Platform pause status: {}", config.is_paused);
    
    Ok(())
}

/// Transfer platform authority to a new address (e.g., multisig)
#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    #[account(
        mut,
        seeds = [PlatformConfig::SEED_PREFIX],
        bump = platform_config.bump,
        constraint = platform_config.update_authority == current_authority.key() @ ErrorCode::Unauthorized
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    
    pub current_authority: Signer<'info>,
}

pub fn transfer_authority(
    ctx: Context<TransferAuthority>,
    new_authority: Pubkey,
) -> Result<()> {
    let config = &mut ctx.accounts.platform_config;
    
    config.update_authority = new_authority;
    msg!("Authority transferred to: {}", new_authority);
    
    Ok(())
}

