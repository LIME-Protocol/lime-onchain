use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};
use lime_market::{Market, MarketStatus};
use lime_vault::{MarketVault, PositionSide as VaultPositionSide, UserPosition};

declare_id!("3YMsnQEW4koSRwLJw1gUeyf6S53GxNFQWSGjRr3NMjeo");

const SCALE: u64 = 1_000_000;

#[program]
pub mod lime_settlement {
    use super::*;

    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        resolver: Pubkey,
    ) -> Result<()> {
        let protocol = &mut ctx.accounts.protocol_config;
        protocol.admin = ctx.accounts.admin.key();
        protocol.resolver = resolver;
        protocol.bump = ctx.bumps.protocol_config;
        Ok(())
    }

    pub fn update_resolver(
        ctx: Context<UpdateProtocolConfig>,
        resolver: Pubkey,
    ) -> Result<()> {
        require!(
            ctx.accounts.admin.key() == ctx.accounts.protocol_config.admin,
            SettlementError::Unauthorized
        );
        ctx.accounts.protocol_config.resolver = resolver;
        Ok(())
    }

    pub fn init_market_settlement(
        ctx: Context<InitMarketSettlement>,
        market_id: u64,
    ) -> Result<()> {
        let protocol = &ctx.accounts.protocol_config;
        require!(ctx.accounts.admin.key() == protocol.admin, SettlementError::Unauthorized);

        let market = &ctx.accounts.market;
        require!(market.market_id == market_id, SettlementError::MarketMismatch);

        let vault = &ctx.accounts.market_vault;
        require!(vault.market_id == market_id, SettlementError::MarketMismatch);
        require!(vault.market == market.key(), SettlementError::MarketMismatch);

        let vault_authority = &mut ctx.accounts.vault_authority;
        vault_authority.market_id = market_id;
        vault_authority.token_mint = vault.token_mint;
        vault_authority.vault_token_account = vault.vault_token_account;
        vault_authority.bump = ctx.bumps.vault_authority;

        require!(
            vault.settlement_authority == vault_authority.key(),
            SettlementError::InvalidVaultAuthority
        );

        Ok(())
    }

    pub fn submit_resolution(
        ctx: Context<SubmitResolution>,
        market_id: u64,
        observed_value: u64,
    ) -> Result<()> {
        let protocol = &ctx.accounts.protocol_config;
        require!(
            ctx.accounts.resolver.key() == protocol.resolver
                || ctx.accounts.resolver.key() == protocol.admin,
            SettlementError::Unauthorized
        );

        let market = &ctx.accounts.market;
        require!(market.market_id == market_id, SettlementError::MarketMismatch);
        require!(
            market.status == MarketStatus::PendingResolution,
            SettlementError::InvalidMarketStatus
        );

        let vault_authority = &ctx.accounts.vault_authority;
        require!(vault_authority.market_id == market_id, SettlementError::MarketMismatch);

        let resolution = &mut ctx.accounts.resolution;
        resolution.market_id = market_id;
        resolution.observed_value = observed_value;
        resolution.lower_bound = market.lower_bound;
        resolution.upper_bound = market.upper_bound;
        resolution.payoff_ratio = calculate_payoff(observed_value, market.lower_bound, market.upper_bound);
        resolution.resolver = ctx.accounts.resolver.key();
        resolution.resolved_at = Clock::get()?.unix_timestamp;
        resolution.vault_authority = vault_authority.key();
        resolution.bump = ctx.bumps.resolution;
        Ok(())
    }

    pub fn claim_payout(ctx: Context<ClaimPayout>, market_id: u64) -> Result<()> {
        let resolution = &ctx.accounts.resolution;
        require!(resolution.market_id == market_id, SettlementError::MarketMismatch);

        let position = &ctx.accounts.user_position;
        require!(position.market_id == market_id, SettlementError::MarketMismatch);
        require!(position.owner == ctx.accounts.user.key(), SettlementError::Unauthorized);

        let receipt = &mut ctx.accounts.claim_receipt;
        require!(!receipt.claimed, SettlementError::AlreadyClaimed);

        let amount = calculate_position_payout(
            position.collateral_locked,
            position.side,
            resolution.payoff_ratio,
        )?;

        receipt.market_id = market_id;
        receipt.user = ctx.accounts.user.key();
        receipt.amount = amount;
        receipt.claimed = false;
        receipt.bump = ctx.bumps.claim_receipt;

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
            amount,
            ctx.accounts.usdc_mint.decimals,
        )?;

        receipt.claimed = true;
        Ok(())
    }

    pub fn refund(ctx: Context<Refund>, market_id: u64) -> Result<()> {
        let protocol = &ctx.accounts.protocol_config;
        require!(ctx.accounts.admin.key() == protocol.admin, SettlementError::Unauthorized);

        let market = &ctx.accounts.market;
        require!(market.market_id == market_id, SettlementError::MarketMismatch);
        require!(market.status == MarketStatus::Cancelled, SettlementError::InvalidMarketStatus);

        let position = &ctx.accounts.user_position;
        require!(position.market_id == market_id, SettlementError::MarketMismatch);

        let receipt = &mut ctx.accounts.refund_receipt;
        require!(!receipt.refunded, SettlementError::AlreadyRefunded);

        let amount = position.collateral_locked;
        receipt.market_id = market_id;
        receipt.user = position.owner;
        receipt.amount = amount;
        receipt.refunded = false;
        receipt.bump = ctx.bumps.refund_receipt;

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
            amount,
            ctx.accounts.usdc_mint.decimals,
        )?;

        receipt.refunded = true;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + ProtocolConfig::INIT_SPACE,
        seeds = [b"protocol"],
        bump
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateProtocolConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct InitMarketSettlement<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    pub market: Account<'info, Market>,
    pub market_vault: Account<'info, MarketVault>,
    #[account(
        init,
        payer = admin,
        space = 8 + VaultAuthority::INIT_SPACE,
        seeds = [b"vault_authority", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct SubmitResolution<'info> {
    #[account(mut)]
    pub resolver: Signer<'info>,
    #[account(seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    pub market: Account<'info, Market>,
    #[account(
        seeds = [b"vault_authority", market_id.to_le_bytes().as_ref()],
        bump = vault_authority.bump
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
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
pub struct ClaimPayout<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = user_ata.owner == user.key() @ SettlementError::Unauthorized,
        constraint = user_ata.mint == usdc_mint.key() @ SettlementError::InvalidMint
    )]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"resolution", market_id.to_le_bytes().as_ref()],
        bump = resolution.bump
    )]
    pub resolution: Account<'info, Resolution>,
    #[account(
        mut,
        constraint = user_position.owner == user.key() @ SettlementError::Unauthorized,
        constraint = user_position.market_id == market_id @ SettlementError::MarketMismatch
    )]
    pub user_position: Account<'info, UserPosition>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + ClaimReceipt::INIT_SPACE,
        seeds = [b"claim", market_id.to_le_bytes().as_ref(), user.key().as_ref()],
        bump
    )]
    pub claim_receipt: Account<'info, ClaimReceipt>,
    #[account(
        seeds = [b"vault_authority", market_id.to_le_bytes().as_ref()],
        bump = vault_authority.bump,
        constraint = resolution.vault_authority == vault_authority.key() @ SettlementError::InvalidVaultAuthority,
        constraint = vault_authority.token_mint == usdc_mint.key() @ SettlementError::InvalidMint
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
    #[account(
        mut,
        address = vault_authority.vault_token_account,
        constraint = vault_token_account.owner == vault_authority.key() @ SettlementError::InvalidVaultAuthority,
        constraint = vault_token_account.mint == usdc_mint.key() @ SettlementError::InvalidMint
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct Refund<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    pub market: Account<'info, Market>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = user_ata.owner == user_position.owner @ SettlementError::Unauthorized,
        constraint = user_ata.mint == usdc_mint.key() @ SettlementError::InvalidMint
    )]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = user_position.market_id == market_id @ SettlementError::MarketMismatch
    )]
    pub user_position: Account<'info, UserPosition>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + RefundReceipt::INIT_SPACE,
        seeds = [b"refund", market_id.to_le_bytes().as_ref(), user_position.owner.as_ref()],
        bump
    )]
    pub refund_receipt: Account<'info, RefundReceipt>,
    #[account(
        seeds = [b"vault_authority", market_id.to_le_bytes().as_ref()],
        bump = vault_authority.bump,
        constraint = vault_authority.token_mint == usdc_mint.key() @ SettlementError::InvalidMint
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
    #[account(
        mut,
        address = vault_authority.vault_token_account,
        constraint = vault_token_account.owner == vault_authority.key() @ SettlementError::InvalidVaultAuthority,
        constraint = vault_token_account.mint == usdc_mint.key() @ SettlementError::InvalidMint
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub admin: Pubkey,
    pub resolver: Pubkey,
    pub bump: u8,
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
    pub vault_authority: Pubkey,
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
pub struct RefundReceipt {
    pub market_id: u64,
    pub user: Pubkey,
    pub amount: u64,
    pub refunded: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct VaultAuthority {
    pub market_id: u64,
    pub token_mint: Pubkey,
    pub vault_token_account: Pubkey,
    pub bump: u8,
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
    #[msg("Invalid market status for this operation")]
    InvalidMarketStatus,
    #[msg("Invalid vault authority")]
    InvalidVaultAuthority,
    #[msg("Invalid token mint")]
    InvalidMint,
    #[msg("Already refunded")]
    AlreadyRefunded,
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

fn calculate_position_payout(
    collateral_locked: u64,
    side: VaultPositionSide,
    payoff_ratio: u64,
) -> Result<u64> {
    let numerator = match side {
        VaultPositionSide::Long => u128::from(collateral_locked)
            .checked_mul(u128::from(payoff_ratio))
            .ok_or(SettlementError::MathOverflow)?,
        VaultPositionSide::Short => u128::from(collateral_locked)
            .checked_mul(u128::from(SCALE.saturating_sub(payoff_ratio)))
            .ok_or(SettlementError::MathOverflow)?,
    };

    let payout = numerator
        .checked_div(u128::from(SCALE))
        .ok_or(SettlementError::MathOverflow)?;

    u64::try_from(payout).map_err(|_| SettlementError::MathOverflow.into())
}
