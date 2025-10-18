use anchor_lang::prelude::*;

declare_id!("E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV");

#[program]
pub mod sportsx_pof {
    use super::*;

    /// Initialize the global state with admin authority
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        global_state.admin = ctx.accounts.admin.key();
        global_state.bump = ctx.bumps.global_state;
        msg!("Global state initialized with admin: {}", global_state.admin);
        Ok(())
    }

    /// Initialize a wallet's point account
    pub fn initialize_wallet(ctx: Context<InitializeWallet>) -> Result<()> {
        let wallet_points = &mut ctx.accounts.wallet_points;
        wallet_points.wallet = ctx.accounts.wallet.key();
        wallet_points.points = 0;
        wallet_points.bump = ctx.bumps.wallet_points;
        msg!("Wallet {} initialized with 0 points", wallet_points.wallet);
        Ok(())
    }

    /// Update wallet points (admin or authorized contract only)
    pub fn update_points(ctx: Context<UpdatePoints>, points_delta: i64) -> Result<()> {
        let wallet_points = &mut ctx.accounts.wallet_points;
        let global_state = &ctx.accounts.global_state;
        let authority = ctx.accounts.authority.key();

        // Check if authority is admin or authorized contract
        require!(
            authority == global_state.admin || 
            global_state.is_authorized(&authority),
            ErrorCode::Unauthorized
        );

        // Update points with overflow protection
        let new_points = wallet_points.points
            .checked_add(points_delta)
            .ok_or(ErrorCode::PointsOverflow)?;
        
        require!(new_points >= 0, ErrorCode::InsufficientPoints);
        
        wallet_points.points = new_points;
        msg!(
            "Wallet {} points updated by {}: {} -> {}",
            wallet_points.wallet,
            points_delta,
            wallet_points.points - points_delta,
            wallet_points.points
        );
        Ok(())
    }

    /// Authorize a contract to update points
    pub fn authorize_contract(ctx: Context<AuthorizeContract>, contract: Pubkey) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        
        require!(
            !global_state.is_authorized(&contract),
            ErrorCode::ContractAlreadyAuthorized
        );

        require!(
            global_state.authorized_contracts.len() < GlobalState::MAX_AUTHORIZED_CONTRACTS,
            ErrorCode::MaxAuthorizedContractsReached
        );

        global_state.authorized_contracts.push(contract);
        msg!("Contract {} authorized", contract);
        Ok(())
    }

    /// Revoke a contract's authorization
    pub fn revoke_contract(ctx: Context<RevokeContract>, contract: Pubkey) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        
        let position = global_state.authorized_contracts
            .iter()
            .position(|&x| x == contract)
            .ok_or(ErrorCode::ContractNotAuthorized)?;

        global_state.authorized_contracts.remove(position);
        msg!("Contract {} revoked", contract);
        Ok(())
    }

    /// Get wallet points (view function - can be called via account fetch)
    pub fn get_points(ctx: Context<GetPoints>) -> Result<i64> {
        let wallet_points = &ctx.accounts.wallet_points;
        msg!("Wallet {} has {} points", wallet_points.wallet, wallet_points.points);
        Ok(wallet_points.points)
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + GlobalState::INIT_SPACE,
        seeds = [b"global_state"],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeWallet<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + WalletPoints::INIT_SPACE,
        seeds = [b"wallet_points", wallet.key().as_ref()],
        bump
    )]
    pub wallet_points: Account<'info, WalletPoints>,
    /// CHECK: This is the wallet we're tracking points for
    pub wallet: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdatePoints<'info> {
    #[account(
        mut,
        seeds = [b"wallet_points", wallet_points.wallet.as_ref()],
        bump = wallet_points.bump
    )]
    pub wallet_points: Account<'info, WalletPoints>,
    #[account(
        seeds = [b"global_state"],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct AuthorizeContract<'info> {
    #[account(
        mut,
        seeds = [b"global_state"],
        bump = global_state.bump,
        has_one = admin
    )]
    pub global_state: Account<'info, GlobalState>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct RevokeContract<'info> {
    #[account(
        mut,
        seeds = [b"global_state"],
        bump = global_state.bump,
        has_one = admin
    )]
    pub global_state: Account<'info, GlobalState>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct GetPoints<'info> {
    #[account(
        seeds = [b"wallet_points", wallet_points.wallet.as_ref()],
        bump = wallet_points.bump
    )]
    pub wallet_points: Account<'info, WalletPoints>,
}

#[account]
#[derive(InitSpace)]
pub struct GlobalState {
    pub admin: Pubkey,
    #[max_len(10)]
    pub authorized_contracts: Vec<Pubkey>,
    pub bump: u8,
}

impl GlobalState {
    pub const MAX_AUTHORIZED_CONTRACTS: usize = 10;

    pub fn is_authorized(&self, contract: &Pubkey) -> bool {
        self.authorized_contracts.contains(contract)
    }
}

#[account]
#[derive(InitSpace)]
pub struct WalletPoints {
    pub wallet: Pubkey,
    pub points: i64,
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized: Only admin or authorized contracts can update points")]
    Unauthorized,
    #[msg("Points overflow occurred")]
    PointsOverflow,
    #[msg("Insufficient points for this operation")]
    InsufficientPoints,
    #[msg("Contract is already authorized")]
    ContractAlreadyAuthorized,
    #[msg("Contract is not authorized")]
    ContractNotAuthorized,
    #[msg("Maximum number of authorized contracts reached")]
    MaxAuthorizedContractsReached,
}

