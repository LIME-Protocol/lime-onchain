use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

declare_id!("73C6Qi25C8owQGRKgrvfDkTXKLgyawSC5MXwAGHj7iMZ");

const SCALE: u64 = 1_000_000;

#[program]
pub mod lime_vault {
    use super::*;

    pub fn init_market_vault(ctx: Context<InitMarketVault>, market_id: u64) -> Result<()> {
        let vault = &mut ctx.accounts.market_vault;
        vault.market_id = market_id;
        vault.token_mint = ctx.accounts.usdc_mint.key();
        vault.vault_token_account = ctx.accounts.vault_token_account.key();
        vault.total_collateral = 0;
        vault.bump = ctx.bumps.market_vault;
        Ok(())
    }

    pub fn deposit_collateral(
        ctx: Context<DepositCollateral>,
        market_id: u64,
        side: PositionSide,
        quantity: u64,
    ) -> Result<()> {
        require!(quantity > 0, VaultError::InvalidQuantity);

        let collateral_amount = quantity
            .checked_mul(SCALE)
            .ok_or(VaultError::MathOverflow)?;
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
            collateral_amount,
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
            .checked_add(quantity)
            .ok_or(VaultError::MathOverflow)?;
        position.collateral_locked = position
            .collateral_locked
            .checked_add(collateral_amount)
            .ok_or(VaultError::MathOverflow)?;

        ctx.accounts.market_vault.total_collateral = ctx
            .accounts
            .market_vault
            .total_collateral
            .checked_add(collateral_amount)
            .ok_or(VaultError::MathOverflow)?;
        Ok(())
    }

    pub fn withdraw_collateral(
        ctx: Context<WithdrawCollateral>,
        amount: u64,
    ) -> Result<()> {
        let position = &mut ctx.accounts.user_position;
        require!(
            position.collateral_locked >= amount,
            VaultError::InsufficientCollateral
        );

        position.collateral_locked = position
            .collateral_locked
            .checked_sub(amount)
            .ok_or(VaultError::MathOverflow)?;
        ctx.accounts.market_vault.total_collateral = ctx
            .accounts
            .market_vault
            .total_collateral
            .checked_sub(amount)
            .ok_or(VaultError::MathOverflow)?;

        let market_id = ctx.accounts.market_vault.market_id;
        let market_bytes = market_id.to_le_bytes();
        let signer_seeds: &[&[u8]] = &[
            b"vault",
            market_bytes.as_ref(),
            &[ctx.accounts.market_vault.bump],
        ];
        token::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    mint: ctx.accounts.usdc_mint.to_account_info(),
                    to: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.market_vault.to_account_info(),
                },
                &[signer_seeds],
            ),
            amount,
            ctx.accounts.usdc_mint.decimals,
        )?;
        Ok(())
    }

    pub fn settle_trade(
        ctx: Context<SettleTrade>,
        _market_id: u64,
        quantity: u64,
        price_scaled: u64,
    ) -> Result<()> {
        require!(quantity > 0, VaultError::InvalidQuantity);
        require!(price_scaled <= SCALE, VaultError::InvalidPrice);

        let buyer_notional = quantity
            .checked_mul(price_scaled)
            .ok_or(VaultError::MathOverflow)?;
        let seller_notional = quantity
            .checked_mul(SCALE.checked_sub(price_scaled).ok_or(VaultError::MathOverflow)?)
            .ok_or(VaultError::MathOverflow)?;

        let buyer = &mut ctx.accounts.buyer_position;
        let seller = &mut ctx.accounts.seller_position;

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
#[instruction(market_id: u64)]
pub struct InitMarketVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = payer,
        space = 8 + MarketVault::INIT_SPACE,
        seeds = [b"vault", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"vault", market_id.to_le_bytes().as_ref()],
        bump = market_vault.bump
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(mut, address = market_vault.vault_token_account)]
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
    #[account(mut)]
    pub user: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"vault", market_id.to_le_bytes().as_ref()],
        bump = market_vault.bump
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(mut, address = market_vault.vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"position", market_id.to_le_bytes().as_ref(), user.key().as_ref()],
        bump = user_position.bump
    )]
    pub user_position: Account<'info, UserPosition>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SettleTrade<'info> {
    pub backend_signer: Signer<'info>,
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
    pub token_mint: Pubkey,
    pub vault_token_account: Pubkey,
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
    #[msg("Quantity must be > 0")]
    InvalidQuantity,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Insufficient collateral")]
    InsufficientCollateral,
    #[msg("Price must be within [0, SCALE]")]
    InvalidPrice,
    #[msg("Cannot mix long and short in same position account")]
    MixedSidePosition,
}
