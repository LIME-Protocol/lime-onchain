use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};
use lime_market::{Market, MarketStatus};

declare_id!("BY7MggeDqzyGgJnCQ34pF5pJA6kGUtNvFhaW4VHbFnLm");

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
        vault.vault_authority = ctx.accounts.vault_authority.key();
        vault.settlement_authority = settlement_authority;
        vault.total_collateral = 0;
        vault.bump = ctx.bumps.market_vault;
        vault.vault_authority_bump = ctx.bumps.vault_authority;
        Ok(())
    }

    pub fn deposit_collateral(
        ctx: Context<DepositCollateral>,
        market_id: u64,
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

        let collateral = &mut ctx.accounts.user_collateral;
        if collateral.owner == Pubkey::default() {
            collateral.market_id = market_id;
            collateral.owner = ctx.accounts.user.key();
            collateral.bump = ctx.bumps.user_collateral;
        }

        collateral.available_collateral = collateral
            .available_collateral
            .checked_add(amount)
            .ok_or(VaultError::MathOverflow)?;
        collateral.total_deposited = collateral
            .total_deposited
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

    pub fn withdraw_available_collateral(
        ctx: Context<WithdrawAvailableCollateral>,
        market_id: u64,
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        require!(
            ctx.accounts.market_vault.market_id == market_id,
            VaultError::MarketMismatch
        );
        require!(
            ctx.accounts.user_collateral.market_id == market_id,
            VaultError::MarketMismatch
        );
        require!(
            ctx.accounts.user_collateral.available_collateral >= amount,
            VaultError::InsufficientAvailableCollateral
        );

        ctx.accounts.user_collateral.available_collateral = ctx
            .accounts
            .user_collateral
            .available_collateral
            .checked_sub(amount)
            .ok_or(VaultError::MathOverflow)?;
        ctx.accounts.market_vault.total_collateral = ctx
            .accounts
            .market_vault
            .total_collateral
            .checked_sub(amount)
            .ok_or(VaultError::MathOverflow)?;

        transfer_from_vault(
            &ctx.accounts.market_vault,
            &ctx.accounts.vault_authority,
            &ctx.accounts.vault_token_account,
            &ctx.accounts.usdc_mint,
            &ctx.accounts.user_ata,
            &ctx.accounts.token_program,
            amount,
            ctx.accounts.usdc_mint.decimals,
        )
    }

    pub fn transfer_for_settlement(
        ctx: Context<TransferForSettlement>,
        market_id: u64,
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        require!(
            ctx.accounts.market_vault.market_id == market_id,
            VaultError::MarketMismatch
        );
        require!(
            ctx.accounts.settlement_authority.key()
                == ctx.accounts.market_vault.settlement_authority,
            VaultError::UnauthorizedSettlement
        );

        ctx.accounts.market_vault.total_collateral = ctx
            .accounts
            .market_vault
            .total_collateral
            .checked_sub(amount)
            .ok_or(VaultError::MathOverflow)?;

        transfer_from_vault(
            &ctx.accounts.market_vault,
            &ctx.accounts.vault_authority,
            &ctx.accounts.vault_token_account,
            &ctx.accounts.usdc_mint,
            &ctx.accounts.recipient_ata,
            &ctx.accounts.token_program,
            amount,
            ctx.accounts.usdc_mint.decimals,
        )
    }

    pub fn settle_trade(
        ctx: Context<SettleTrade>,
        market_id: u64,
        buyer: Pubkey,
        seller: Pubkey,
        buyer_nonce: u128,
        seller_nonce: u128,
        quantity: u64,
        price_scaled: u64,
    ) -> Result<()> {
        require!(quantity > 0, VaultError::InvalidAmount);
        require!(price_scaled <= SCALE, VaultError::InvalidPrice);
        let market = &ctx.accounts.market;
        require!(market.market_id == market_id, VaultError::MarketMismatch);
        require!(market.status == MarketStatus::Active, VaultError::MarketNotTradable);
        require!(
            ctx.accounts.market_vault.market_id == market_id,
            VaultError::MarketMismatch
        );

        let buyer_position = &mut ctx.accounts.buyer_position;
        let seller_position = &mut ctx.accounts.seller_position;
        let buyer_collateral = &mut ctx.accounts.buyer_collateral;
        let seller_collateral = &mut ctx.accounts.seller_collateral;
        require!(buyer_collateral.market_id == market_id, VaultError::MarketMismatch);
        require!(seller_collateral.market_id == market_id, VaultError::MarketMismatch);
        require!(buyer_collateral.owner == buyer, VaultError::PositionOwnerMismatch);
        require!(seller_collateral.owner == seller, VaultError::PositionOwnerMismatch);
        require!(buyer != seller, VaultError::SelfTradeDisabled);

        let buyer_fill = &mut ctx.accounts.buyer_fill;
        let seller_fill = &mut ctx.accounts.seller_fill;
        if buyer_fill.owner == Pubkey::default() {
            buyer_fill.market_id = market_id;
            buyer_fill.owner = buyer;
            buyer_fill.nonce = buyer_nonce;
            buyer_fill.quantity = quantity;
            buyer_fill.filled_quantity = 0;
            buyer_fill.bump = ctx.bumps.buyer_fill;
        }
        if seller_fill.owner == Pubkey::default() {
            seller_fill.market_id = market_id;
            seller_fill.owner = seller;
            seller_fill.nonce = seller_nonce;
            seller_fill.quantity = quantity;
            seller_fill.filled_quantity = 0;
            seller_fill.bump = ctx.bumps.seller_fill;
        }

        require!(buyer_fill.market_id == market_id, VaultError::MarketMismatch);
        require!(seller_fill.market_id == market_id, VaultError::MarketMismatch);
        require!(buyer_fill.owner == buyer, VaultError::PositionOwnerMismatch);
        require!(seller_fill.owner == seller, VaultError::PositionOwnerMismatch);
        require!(buyer_fill.nonce == buyer_nonce, VaultError::FillNonceMismatch);
        require!(seller_fill.nonce == seller_nonce, VaultError::FillNonceMismatch);

        if buyer_position.owner == Pubkey::default() {
            buyer_position.market_id = market_id;
            buyer_position.owner = buyer;
            buyer_position.side = PositionSide::Long;
            buyer_position.bump = ctx.bumps.buyer_position;
        }
        if seller_position.owner == Pubkey::default() {
            seller_position.market_id = market_id;
            seller_position.owner = seller;
            seller_position.side = PositionSide::Short;
            seller_position.bump = ctx.bumps.seller_position;
        }

        require!(buyer_position.market_id == market_id, VaultError::MarketMismatch);
        require!(seller_position.market_id == market_id, VaultError::MarketMismatch);
        require!(buyer_position.owner == buyer, VaultError::PositionOwnerMismatch);
        require!(seller_position.owner == seller, VaultError::PositionOwnerMismatch);
        require!(buyer_position.side == PositionSide::Long, VaultError::InvalidSideForTrade);
        require!(seller_position.side == PositionSide::Short, VaultError::InvalidSideForTrade);

        let buyer_notional = quantity
            .checked_mul(price_scaled)
            .ok_or(VaultError::MathOverflow)?
            .checked_div(SCALE)
            .ok_or(VaultError::MathOverflow)?;
        let seller_notional = quantity
            .checked_sub(buyer_notional)
            .ok_or(VaultError::MathOverflow)?;

        require!(
            buyer_collateral.available_collateral >= buyer_notional,
            VaultError::InsufficientAvailableCollateral
        );
        require!(
            seller_collateral.available_collateral >= seller_notional,
            VaultError::InsufficientAvailableCollateral
        );

        buyer_collateral.available_collateral = buyer_collateral
            .available_collateral
            .checked_sub(buyer_notional)
            .ok_or(VaultError::MathOverflow)?;
        seller_collateral.available_collateral = seller_collateral
            .available_collateral
            .checked_sub(seller_notional)
            .ok_or(VaultError::MathOverflow)?;

        buyer_position.quantity = buyer_position
            .quantity
            .checked_add(quantity)
            .ok_or(VaultError::MathOverflow)?;
        buyer_position.cost_basis = buyer_position
            .cost_basis
            .checked_add(buyer_notional)
            .ok_or(VaultError::MathOverflow)?;

        seller_position.quantity = seller_position
            .quantity
            .checked_add(quantity)
            .ok_or(VaultError::MathOverflow)?;
        seller_position.cost_basis = seller_position
            .cost_basis
            .checked_add(seller_notional)
            .ok_or(VaultError::MathOverflow)?;

        buyer_fill.filled_quantity = buyer_fill
            .filled_quantity
            .checked_add(quantity)
            .ok_or(VaultError::MathOverflow)?;
        seller_fill.filled_quantity = seller_fill
            .filled_quantity
            .checked_add(quantity)
            .ok_or(VaultError::MathOverflow)?;

        emit!(TradeSettled {
            market_vault: ctx.accounts.market_vault.key(),
            buyer,
            seller,
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
        seeds = [b"vault_authority", market_id.to_le_bytes().as_ref()],
        bump
    )]
    /// CHECK: PDA authority for the vault token account.
    pub vault_authority: UncheckedAccount<'info>,
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
        constraint = vault_token_account.owner == vault_authority.key() @ VaultError::InvalidVaultAuthority
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
        constraint = vault_token_account.owner == market_vault.vault_authority @ VaultError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserCollateral::INIT_SPACE,
        seeds = [b"collateral", market_id.to_le_bytes().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_collateral: Account<'info, UserCollateral>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct WithdrawAvailableCollateral<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub market: Account<'info, Market>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"vault", market_id.to_le_bytes().as_ref()],
        bump = market_vault.bump,
        constraint = market_vault.market_id == market_id @ VaultError::MarketMismatch,
        constraint = market_vault.market == market.key() @ VaultError::MarketMismatch,
        constraint = market_vault.token_mint == usdc_mint.key() @ VaultError::InvalidMint
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(
        seeds = [b"vault_authority", market_id.to_le_bytes().as_ref()],
        bump = market_vault.vault_authority_bump,
        address = market_vault.vault_authority
    )]
    /// CHECK: PDA authority for the vault token account.
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        address = market_vault.vault_token_account,
        constraint = vault_token_account.mint == usdc_mint.key() @ VaultError::InvalidMint,
        constraint = vault_token_account.owner == vault_authority.key() @ VaultError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = user_ata.owner == user.key() @ VaultError::InvalidTokenOwner,
        constraint = user_ata.mint == usdc_mint.key() @ VaultError::InvalidMint
    )]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"collateral", market_id.to_le_bytes().as_ref(), user.key().as_ref()],
        bump = user_collateral.bump
    )]
    pub user_collateral: Account<'info, UserCollateral>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct TransferForSettlement<'info> {
    pub settlement_authority: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"vault", market_id.to_le_bytes().as_ref()],
        bump = market_vault.bump,
        constraint = market_vault.market_id == market_id @ VaultError::MarketMismatch,
        constraint = market_vault.token_mint == usdc_mint.key() @ VaultError::InvalidMint
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(
        seeds = [b"vault_authority", market_id.to_le_bytes().as_ref()],
        bump = market_vault.vault_authority_bump,
        address = market_vault.vault_authority
    )]
    /// CHECK: PDA authority for the vault token account.
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        address = market_vault.vault_token_account,
        constraint = vault_token_account.mint == usdc_mint.key() @ VaultError::InvalidMint,
        constraint = vault_token_account.owner == vault_authority.key() @ VaultError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = recipient_ata.mint == usdc_mint.key() @ VaultError::InvalidMint
    )]
    pub recipient_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(market_id: u64, buyer: Pubkey, seller: Pubkey, buyer_nonce: u128, seller_nonce: u128)]
pub struct SettleTrade<'info> {
    #[account(mut)]
    pub submitter: Signer<'info>,
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"vault", market_id.to_le_bytes().as_ref()],
        bump = market_vault.bump
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(
        mut,
        seeds = [b"collateral", market_id.to_le_bytes().as_ref(), buyer.as_ref()],
        bump = buyer_collateral.bump
    )]
    pub buyer_collateral: Account<'info, UserCollateral>,
    #[account(
        mut,
        seeds = [b"collateral", market_id.to_le_bytes().as_ref(), seller.as_ref()],
        bump = seller_collateral.bump
    )]
    pub seller_collateral: Account<'info, UserCollateral>,
    #[account(
        init_if_needed,
        payer = submitter,
        space = 8 + FillState::INIT_SPACE,
        seeds = [
            b"fill",
            market_id.to_le_bytes().as_ref(),
            buyer.as_ref(),
            buyer_nonce.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub buyer_fill: Account<'info, FillState>,
    #[account(
        init_if_needed,
        payer = submitter,
        space = 8 + FillState::INIT_SPACE,
        seeds = [
            b"fill",
            market_id.to_le_bytes().as_ref(),
            seller.as_ref(),
            seller_nonce.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub seller_fill: Account<'info, FillState>,
    #[account(
        init_if_needed,
        payer = submitter,
        space = 8 + UserPosition::INIT_SPACE,
        seeds = [
            b"position",
            market_id.to_le_bytes().as_ref(),
            buyer.as_ref(),
            PositionSide::Long.seed()
        ],
        bump
    )]
    pub buyer_position: Account<'info, UserPosition>,
    #[account(
        init_if_needed,
        payer = submitter,
        space = 8 + UserPosition::INIT_SPACE,
        seeds = [
            b"position",
            market_id.to_le_bytes().as_ref(),
            seller.as_ref(),
            PositionSide::Short.seed()
        ],
        bump
    )]
    pub seller_position: Account<'info, UserPosition>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct MarketVault {
    pub market_id: u64,
    pub market: Pubkey,
    pub token_mint: Pubkey,
    pub vault_token_account: Pubkey,
    pub vault_authority: Pubkey,
    pub settlement_authority: Pubkey,
    pub total_collateral: u64,
    pub bump: u8,
    pub vault_authority_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct UserCollateral {
    pub market_id: u64,
    pub owner: Pubkey,
    pub available_collateral: u64,
    pub total_deposited: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct UserPosition {
    pub market_id: u64,
    pub owner: Pubkey,
    pub side: PositionSide,
    pub quantity: u64,
    pub cost_basis: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct FillState {
    pub market_id: u64,
    pub owner: Pubkey,
    pub nonce: u128,
    pub quantity: u64,
    pub filled_quantity: u64,
    pub bump: u8,
}

#[event]
pub struct FillStateSchema {
    pub state: FillState,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum PositionSide {
    Long,
    Short,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum OrderNetwork {
    MainnetBeta,
    Devnet,
    Localnet,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct SignedOrder {
    pub version: u8,
    pub network: OrderNetwork,
    pub market_program_id: Pubkey,
    pub vault_program_id: Pubkey,
    pub market_id: u64,
    pub owner: Pubkey,
    pub side: OrderSide,
    pub price_scaled: u64,
    pub quantity: u64,
    pub expiration_ts: i64,
    pub nonce: u128,
}

#[event]
pub struct SignedOrderSchema {
    pub order: SignedOrder,
}

impl PositionSide {
    pub fn seed(&self) -> &'static [u8] {
        match self {
            PositionSide::Long => b"long",
            PositionSide::Short => b"short",
        }
    }
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
    #[msg("Fill PDA nonce does not match the submitted order nonce")]
    FillNonceMismatch,
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
    #[msg("Insufficient available collateral")]
    InsufficientAvailableCollateral,
    #[msg("Position/account owner mismatch")]
    PositionOwnerMismatch,
    #[msg("Self trades are disabled for MVP")]
    SelfTradeDisabled,
    #[msg("Settlement authority is not authorized")]
    UnauthorizedSettlement,
}

#[allow(clippy::too_many_arguments)]
fn transfer_from_vault<'info>(
    market_vault: &Account<'info, MarketVault>,
    vault_authority: &UncheckedAccount<'info>,
    vault_token_account: &Account<'info, TokenAccount>,
    usdc_mint: &Account<'info, Mint>,
    recipient_ata: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let market_bytes = market_vault.market_id.to_le_bytes();
    let signer_seeds: &[&[u8]] = &[
        b"vault_authority",
        market_bytes.as_ref(),
        &[market_vault.vault_authority_bump],
    ];

    token::transfer_checked(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            TransferChecked {
                from: vault_token_account.to_account_info(),
                mint: usdc_mint.to_account_info(),
                to: recipient_ata.to_account_info(),
                authority: vault_authority.to_account_info(),
            },
            &[signer_seeds],
        ),
        amount,
        decimals,
    )
}
