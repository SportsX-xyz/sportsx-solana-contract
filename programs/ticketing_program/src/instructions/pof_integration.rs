use anchor_lang::prelude::*;

// PoF Program ID
pub const POF_PROGRAM_ID: Pubkey = pubkey!("E5Arj2VAzHNHwWgFQgb6nHfp1WQA5ShEpdbjYmknpafV");

// PoF Account Types (from IDL)
#[account]
pub struct WalletPoints {
    pub wallet: Pubkey,
    pub points: i64,
    pub bump: u8,
}

#[account]
pub struct GlobalState {
    pub admin: Pubkey,
    pub authorized_contracts: Vec<Pubkey>,
    pub bump: u8,
}

/// Calculate points for ticket purchase based on USDC price
/// Rule: min(50, floor(price_usdc / 10))
/// price_usdc is in micro USDC (1 USDC = 1_000_000)
pub fn calculate_purchase_points(price_usdc: u64) -> i64 {
    let usdc_amount = price_usdc / 1_000_000; // Convert to whole USDC
    let points = (usdc_amount / 10) as i64;
    std::cmp::min(50, points)
}

/// CPI to PoF program to update wallet points
/// 
/// This uses standard Anchor CPI pattern as recommended by PoF team
pub fn update_pof_points<'info>(
    wallet_points: &AccountInfo<'info>,
    global_state: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    pof_program: &AccountInfo<'info>,
    points_delta: i64,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    // Build instruction data: discriminator + points_delta
    let discriminator: [u8; 8] = [52, 203, 177, 240, 97, 102, 160, 48]; // update_points
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&points_delta.to_le_bytes());

    // Build CPI instruction
    let accounts = vec![
        AccountMeta::new(wallet_points.key(), false),
        AccountMeta::new_readonly(global_state.key(), false),
        AccountMeta::new_readonly(authority.key(), true), // authority must sign
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: POF_PROGRAM_ID,
        accounts,
        data,
    };

    // Invoke with signer seeds
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            wallet_points.clone(),
            global_state.clone(),
            authority.clone(),
            pof_program.clone(),
        ],
        signer_seeds,
    )?;

    Ok(())
}

/// Derive PoF wallet points PDA
pub fn derive_wallet_points_pda(wallet: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"wallet_points", wallet.as_ref()],
        &POF_PROGRAM_ID,
    )
}

/// Derive PoF global state PDA
pub fn derive_global_state_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"global_state"],
        &POF_PROGRAM_ID,
    )
}
