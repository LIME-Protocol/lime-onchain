use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};
use lime_market::{Market, MarketStatus};

declare_id!("73C6Qi25C8owQGRKgrvfDkTXKLgyawSC5MXwAGHj7iMZ");

const SCALE: u64 = 1_000_000;

#[program]
pub mod lime_vault {
    use super::*;

    pub fn init_market_vault(
        ctx: Context<InitMarketVault>,
        market_id: u64,
        settlement_authority: Pubkey,
    ) -> Result<()> {
        let market = &ctx.accounts.market;
        require!(market.market_id == market_id, VaultError::MarketMismatch);
        require!(
            market.status == MarketStatus::Preliminary || market.status == MarketStatus::Active,
            VaultError::MarketNotTradable
        );

        let vault = &mut ctx.accounts.market_vault;
        vault.market_id = market_id;
        vault.market = market.key();
        vault.token_mint = ctx.accounts.usdc_mint.key();
        vault.vault_token_account = ctx.accounts.vault_token_account.key();
        vault.settlement_authority = settlement_authority;
        vault.total_collateral = 0;
        vault.bump = ctx.bumps.market_vault;
        Ok(())
    }

    pub fn deposit_collateral(
        ctx: Context<DepositCollateral>,
        market_id: u64,
        side: PositionSide,
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        let market = &ctx.accounts.market;
        require!(market.market_id == market_id, VaultError::MarketMismatch);
        require!(
            market.status == MarketStatus::Preliminary || market.status == MarketStatus::Active,
            VaultError::MarketNotTradable
        );

        token::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.user_ata.to_account_info(),
                    mint: ctx.accounts.usdc_mint.to_account_info(),
                    to: ctx.accounts.vault_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
            ctx.accounts.usdc_mint.decimals,
        )?;

        let position = &mut ctx.accounts.user_position;
        if position.owner == Pubkey::default() {
            position.market_id = market_id;
            position.owner = ctx.accounts.user.key();
            position.side = side;
            position.bump = ctx.bumps.user_position;
        } else {
            require!(position.side == side, VaultError::MixedSidePosition);
        }

        position.quantity = position
            .quantity
            .checked_add(amount)
            .ok_or(VaultError::MathOverflow)?;
        position.collateral_locked = position
            .collateral_locked
            .checked_add(amount)
            .ok_or(VaultError::MathOverflow)?;

        ctx.accounts.market_vault.total_collateral = ctx
            .accounts
            .market_vault
            .total_collateral
            .checked_add(amount)
            .ok_or(VaultError::MathOverflow)?;
        Ok(())
    }

    pub fn withdraw_collateral(
        _ctx: Context<WithdrawCollateral>,
        _amount: u64,
    ) -> Result<()> {
        err!(VaultError::WithdrawDisabledUseSettlement)
    }

    pub fn settle_trade(
        ctx: Context<SettleTrade>,
        market_id: u64,
        quantity: u64,
        price_scaled: u64,
    ) -> Result<()> {
        require!(quantity > 0, VaultError::InvalidAmount);
        require!(price_scaled <= SCALE, VaultError::InvalidPrice);
        let market = &ctx.accounts.market;
        require!(market.market_id == market_id, VaultError::MarketMismatch);
        require!(market.status == MarketStatus::Active, VaultError::MarketNotTradable);
        require!(
            ctx.accounts.backend_signer.key() == market.admin,
            VaultError::UnauthorizedBackend
        );
        require!(
            ctx.accounts.market_vault.market_id == market_id,
            VaultError::MarketMismatch
        );

        let buyer = &mut ctx.accounts.buyer_position;
        let seller = &mut ctx.accounts.seller_position;
        require!(buyer.market_id == market_id, VaultError::MarketMismatch);
        require!(seller.market_id == market_id, VaultError::MarketMismatch);
        require!(buyer.side == PositionSide::Long, VaultError::InvalidSideForTrade);
        require!(seller.side == PositionSide::Short, VaultError::InvalidSideForTrade);

        let buyer_notional = quantity
            .checked_mul(price_scaled)
            .ok_or(VaultError::MathOverflow)?
            .checked_div(SCALE)
            .ok_or(VaultError::MathOverflow)?;
        let seller_notional = quantity
            .checked_sub(buyer_notional)
            .ok_or(VaultError::MathOverflow)?;

        buyer.quantity = buyer
            .quantity
            .checked_add(quantity)
            .ok_or(VaultError::MathOverflow)?;
        buyer.collateral_locked = buyer
            .collateral_locked
            .checked_add(buyer_notional)
            .ok_or(VaultError::MathOverflow)?;

        seller.quantity = seller
            .quantity
            .checked_add(quantity)
            .ok_or(VaultError::MathOverflow)?;
        seller.collateral_locked = seller
            .collateral_locked
            .checked_add(seller_notional)
            .ok_or(VaultError::MathOverflow)?;

        let total_added = buyer_notional
            .checked_add(seller_notional)
            .ok_or(VaultError::MathOverflow)?;
        ctx.accounts.market_vault.total_collateral = ctx
            .accounts
            .market_vault
            .total_collateral
            .checked_add(total_added)
            .ok_or(VaultError::MathOverflow)?;

        emit!(TradeSettled {
            market_vault: ctx.accounts.market_vault.key(),
            buyer: buyer.owner,
            seller: seller.owner,
            quantity,
            price_scaled,
        });
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(market_id: u64, settlement_authority: Pubkey)]
pub struct InitMarketVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    pub market: Account<'info, Market>,
    #[account(
        init,
        payer = payer,
        space = 8 + MarketVault::INIT_SPACE,
        seeds = [b"vault", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(
        mut,
        constraint = vault_token_account.mint == usdc_mint.key() @ VaultError::InvalidMint,
        constraint = vault_token_account.owner == settlement_authority @ VaultError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    pub market: Account<'info, Market>,
    #[account(
        mut,
        constraint = user_ata.owner == user.key() @ VaultError::InvalidTokenOwner,
        constraint = user_ata.mint == usdc_mint.key() @ VaultError::InvalidMint
    )]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"vault", market_id.to_le_bytes().as_ref()],
        bump = market_vault.bump,
        constraint = market_vault.market_id == market_id @ VaultError::MarketMismatch,
        constraint = market_vault.market == market.key() @ VaultError::MarketMismatch
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(
        mut,
        address = market_vault.vault_token_account,
        constraint = vault_token_account.mint == usdc_mint.key() @ VaultError::InvalidMint,
        constraint = vault_token_account.owner == market_vault.settlement_authority @ VaultError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserPosition::INIT_SPACE,
        seeds = [b"position", market_id.to_le_bytes().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_position: Account<'info, UserPosition>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct WithdrawCollateral<'info> {
    pub user: Signer<'info>,
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"vault", market_id.to_le_bytes().as_ref()],
        bump = market_vault.bump
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(
        mut,
        seeds = [b"position", market_id.to_le_bytes().as_ref(), user.key().as_ref()],
        bump = user_position.bump
    )]
    pub user_position: Account<'info, UserPosition>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct SettleTrade<'info> {
    pub backend_signer: Signer<'info>,
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"vault", market_id.to_le_bytes().as_ref()],
        bump = market_vault.bump
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(mut)]
    pub buyer_position: Account<'info, UserPosition>,
    #[account(mut)]
    pub seller_position: Account<'info, UserPosition>,
}

#[account]
#[derive(InitSpace)]
pub struct MarketVault {
    pub market_id: u64,
    pub market: Pubkey,
    pub token_mint: Pubkey,
    pub vault_token_account: Pubkey,
    pub settlement_authority: Pubkey,
    pub total_collateral: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct UserPosition {
    pub market_id: u64,
    pub owner: Pubkey,
    pub side: PositionSide,
    pub quantity: u64,
    pub collateral_locked: u64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum PositionSide {
    Long,
    Short,
}

#[event]
pub struct TradeSettled {
    pub market_vault: Pubkey,
    pub buyer: Pubkey,
    pub seller: Pubkey,
    pub quantity: u64,
    pub price_scaled: u64,
}

#[error_code]
pub enum VaultError {
    #[msg("Amount must be > 0")]
    InvalidAmount,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Price must be within [0, SCALE]")]
    InvalidPrice,
    #[msg("Cannot mix long and short in same position account")]
    MixedSidePosition,
    #[msg("Market/account mismatch")]
    MarketMismatch,
    #[msg("Market is not in a tradable state")]
    MarketNotTradable,
    #[msg("Backend signer is not authorized")]
    UnauthorizedBackend,
    #[msg("Invalid token mint")]
    InvalidMint,
    #[msg("Invalid token account owner")]
    InvalidTokenOwner,
    #[msg("Invalid settlement vault authority")]
    InvalidVaultAuthority,
    #[msg("Invalid side for settle_trade account")]
    InvalidSideForTrade,
    #[msg("Direct withdraw is disabled for MVP, use settlement/refund flows")]
    WithdrawDisabledUseSettlement,
}
