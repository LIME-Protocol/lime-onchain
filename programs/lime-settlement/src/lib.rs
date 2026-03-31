use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

declare_id!("3YMsnQEW4koSRwLJw1gUeyf6S53GxNFQWSGjRr3NMjeo");

const SCALE: u64 = 1_000_000;

#[program]
pub mod lime_settlement {
    use super::*;

    pub fn submit_resolution(
        ctx: Context<SubmitResolution>,
        market_id: u64,
        observed_value: u64,
        lower_bound: u64,
        upper_bound: u64,
    ) -> Result<()> {
        require!(lower_bound < upper_bound, SettlementError::InvalidBounds);
        let resolution = &mut ctx.accounts.resolution;
        resolution.market_id = market_id;
        resolution.observed_value = observed_value;
        resolution.lower_bound = lower_bound;
        resolution.upper_bound = upper_bound;
        resolution.payoff_ratio = calculate_payoff(observed_value, lower_bound, upper_bound);
        resolution.resolver = ctx.accounts.resolver.key();
        resolution.resolved_at = Clock::get()?.unix_timestamp;
        resolution.bump = ctx.bumps.resolution;
        Ok(())
    }

    pub fn calculate_payouts(
        ctx: Context<CalculatePayouts>,
        market_id: u64,
    ) -> Result<()> {
        let receipt = &mut ctx.accounts.claim_receipt;
        let resolution = &ctx.accounts.resolution;
        let position = &ctx.accounts.user_position;
        require!(resolution.market_id == market_id, SettlementError::MarketMismatch);
        require!(position.market_id == market_id, SettlementError::MarketMismatch);
        require!(!receipt.claimed, SettlementError::AlreadyClaimed);

        let gross = match position.side {
            PositionSide::Long => position
                .quantity
                .checked_mul(resolution.payoff_ratio)
                .ok_or(SettlementError::MathOverflow)?
                .checked_div(SCALE)
                .ok_or(SettlementError::MathOverflow)?,
            PositionSide::Short => position
                .quantity
                .checked_mul(
                    SCALE
                        .checked_sub(resolution.payoff_ratio)
                        .ok_or(SettlementError::MathOverflow)?,
                )
                .ok_or(SettlementError::MathOverflow)?
                .checked_div(SCALE)
                .ok_or(SettlementError::MathOverflow)?,
        };

        receipt.market_id = market_id;
        receipt.user = position.owner;
        receipt.amount = gross;
        receipt.claimed = false;
        receipt.bump = ctx.bumps.claim_receipt;
        Ok(())
    }

    pub fn claim_payout(ctx: Context<ClaimPayout>, market_id: u64) -> Result<()> {
        let receipt = &mut ctx.accounts.claim_receipt;
        require!(!receipt.claimed, SettlementError::AlreadyClaimed);
        require!(receipt.market_id == market_id, SettlementError::MarketMismatch);

        let market_bytes = market_id.to_le_bytes();
        let signer_seeds: &[&[u8]] = &[
            b"vault_authority",
            market_bytes.as_ref(),
            &[ctx.accounts.vault_authority.bump],
        ];
        token::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    mint: ctx.accounts.usdc_mint.to_account_info(),
                    to: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                &[signer_seeds],
            ),
            receipt.amount,
            ctx.accounts.usdc_mint.decimals,
        )?;
        receipt.claimed = true;
        Ok(())
    }

    pub fn refund(ctx: Context<Refund>, market_id: u64, refund_amount: u64) -> Result<()> {
        let market_bytes = market_id.to_le_bytes();
        let signer_seeds: &[&[u8]] = &[
            b"vault_authority",
            market_bytes.as_ref(),
            &[ctx.accounts.vault_authority.bump],
        ];
        token::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    mint: ctx.accounts.usdc_mint.to_account_info(),
                    to: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                &[signer_seeds],
            ),
            refund_amount,
            ctx.accounts.usdc_mint.decimals,
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct SubmitResolution<'info> {
    #[account(mut)]
    pub resolver: Signer<'info>,
    #[account(
        init,
        payer = resolver,
        space = 8 + Resolution::INIT_SPACE,
        seeds = [b"resolution", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub resolution: Account<'info, Resolution>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct CalculatePayouts<'info> {
    #[account(mut)]
    pub resolver: Signer<'info>,
    #[account(
        mut,
        seeds = [b"resolution", market_id.to_le_bytes().as_ref()],
        bump = resolution.bump
    )]
    pub resolution: Account<'info, Resolution>,
    pub user_position: Account<'info, UserPosition>,
    #[account(
        init_if_needed,
        payer = resolver,
        space = 8 + ClaimReceipt::INIT_SPACE,
        seeds = [b"claim", market_id.to_le_bytes().as_ref(), user_position.owner.as_ref()],
        bump
    )]
    pub claim_receipt: Account<'info, ClaimReceipt>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct ClaimPayout<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"claim", market_id.to_le_bytes().as_ref(), user.key().as_ref()],
        bump = claim_receipt.bump,
        constraint = claim_receipt.user == user.key() @ SettlementError::Unauthorized
    )]
    pub claim_receipt: Account<'info, ClaimReceipt>,
    #[account(
        seeds = [b"vault_authority", market_id.to_le_bytes().as_ref()],
        bump = vault_authority.bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct Refund<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"vault_authority", market_id.to_le_bytes().as_ref()],
        bump = vault_authority.bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Resolution {
    pub market_id: u64,
    pub observed_value: u64,
    pub lower_bound: u64,
    pub upper_bound: u64,
    pub payoff_ratio: u64,
    pub resolver: Pubkey,
    pub resolved_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ClaimReceipt {
    pub market_id: u64,
    pub user: Pubkey,
    pub amount: u64,
    pub claimed: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct VaultAuthority {
    pub market_id: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct UserPosition {
    pub market_id: u64,
    pub owner: Pubkey,
    pub side: PositionSide,
    pub quantity: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum PositionSide {
    Long,
    Short,
}

#[error_code]
pub enum SettlementError {
    #[msg("Lower bound must be lower than upper bound")]
    InvalidBounds,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Already claimed")]
    AlreadyClaimed,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Market mismatch")]
    MarketMismatch,
}

fn calculate_payoff(observed: u64, lower: u64, upper: u64) -> u64 {
    if observed <= lower {
        return 0;
    }
    if observed >= upper {
        return SCALE;
    }
    (observed - lower) * SCALE / (upper - lower)
}
