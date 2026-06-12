use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    sysvar::instructions::{load_current_index_checked, load_instruction_at_checked},
};
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};
use lime_market::{Market, MarketStatus};

declare_id!("BY7MggeDqzyGgJnCQ34pF5pJA6kGUtNvFhaW4VHbFnLm");

const SCALE: u64 = 1_000_000;
pub const LIME_SIGNED_ORDER_DOMAIN: &[u8; 17] = b"LIME_SIGNED_ORDER";
pub const LIME_SIGNED_ORDER_DOMAIN_LEN: usize = 17;
pub const SIGNED_ORDER_MESSAGE_LEN: usize =
    1 + LIME_SIGNED_ORDER_DOMAIN_LEN + 1 + 1 + 32 + 32 + 8 + 32 + 1 + 8 + 8 + 8 + 16;

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
        buyer_order: SignedOrder,
        seller_order: SignedOrder,
        quantity: u64,
    ) -> Result<()> {
        let market = &ctx.accounts.market;
        let market_id = buyer_order.market_id;
        let buyer = buyer_order.owner;
        let seller = seller_order.owner;
        validate_order_pair(
            &buyer_order,
            &seller_order,
            quantity,
            *market.to_account_info().owner,
            *ctx.program_id,
            Clock::get()?.unix_timestamp,
        )?;
        assert_signed_order_ed25519_verified(
            &ctx.accounts.instructions.to_account_info(),
            &buyer_order,
        )?;
        assert_signed_order_ed25519_verified(
            &ctx.accounts.instructions.to_account_info(),
            &seller_order,
        )?;

        require!(market.market_id == market_id, VaultError::MarketMismatch);
        require!(
            market.status == MarketStatus::Active,
            VaultError::MarketNotTradable
        );
        require!(
            ctx.accounts.market_vault.market_id == market_id,
            VaultError::MarketMismatch
        );
        require!(
            seller_order.market_id == market_id,
            VaultError::SignedOrderMarketMismatch
        );

        let buyer_position = &mut ctx.accounts.buyer_position;
        let seller_position = &mut ctx.accounts.seller_position;
        let buyer_collateral = &mut ctx.accounts.buyer_collateral;
        let seller_collateral = &mut ctx.accounts.seller_collateral;
        require!(
            buyer_collateral.market_id == market_id,
            VaultError::MarketMismatch
        );
        require!(
            seller_collateral.market_id == market_id,
            VaultError::MarketMismatch
        );
        require!(
            buyer_collateral.owner == buyer,
            VaultError::PositionOwnerMismatch
        );
        require!(
            seller_collateral.owner == seller,
            VaultError::PositionOwnerMismatch
        );
        require!(buyer != seller, VaultError::SelfTradeDisabled);

        let buyer_fill = &mut ctx.accounts.buyer_fill;
        let seller_fill = &mut ctx.accounts.seller_fill;
        if buyer_fill.owner == Pubkey::default() {
            buyer_fill.market_id = market_id;
            buyer_fill.owner = buyer;
            buyer_fill.nonce = buyer_order.nonce;
            buyer_fill.quantity = buyer_order.quantity;
            buyer_fill.filled_quantity = 0;
            buyer_fill.bump = ctx.bumps.buyer_fill;
        }
        if seller_fill.owner == Pubkey::default() {
            seller_fill.market_id = market_id;
            seller_fill.owner = seller;
            seller_fill.nonce = seller_order.nonce;
            seller_fill.quantity = seller_order.quantity;
            seller_fill.filled_quantity = 0;
            seller_fill.bump = ctx.bumps.seller_fill;
        }

        require!(
            buyer_fill.market_id == market_id,
            VaultError::MarketMismatch
        );
        require!(
            seller_fill.market_id == market_id,
            VaultError::MarketMismatch
        );
        require!(buyer_fill.owner == buyer, VaultError::PositionOwnerMismatch);
        require!(
            seller_fill.owner == seller,
            VaultError::PositionOwnerMismatch
        );
        require!(
            buyer_fill.nonce == buyer_order.nonce,
            VaultError::FillNonceMismatch
        );
        require!(
            seller_fill.nonce == seller_order.nonce,
            VaultError::FillNonceMismatch
        );
        assert_fill_capacity(buyer_fill, &buyer_order, quantity)?;
        assert_fill_capacity(seller_fill, &seller_order, quantity)?;

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

        require!(
            buyer_position.market_id == market_id,
            VaultError::MarketMismatch
        );
        require!(
            seller_position.market_id == market_id,
            VaultError::MarketMismatch
        );
        require!(
            buyer_position.owner == buyer,
            VaultError::PositionOwnerMismatch
        );
        require!(
            seller_position.owner == seller,
            VaultError::PositionOwnerMismatch
        );
        require!(
            buyer_position.side == PositionSide::Long,
            VaultError::InvalidSideForTrade
        );
        require!(
            seller_position.side == PositionSide::Short,
            VaultError::InvalidSideForTrade
        );

        let price_scaled = seller_order.price_scaled;
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
    pub market: Box<Account<'info, Market>>,
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
    pub market_vault: Box<Account<'info, MarketVault>>,
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
#[instruction(buyer_order: SignedOrder, seller_order: SignedOrder, quantity: u64)]
pub struct SettleTrade<'info> {
    #[account(mut)]
    pub submitter: Signer<'info>,
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"vault", buyer_order.market_id.to_le_bytes().as_ref()],
        bump = market_vault.bump
    )]
    pub market_vault: Account<'info, MarketVault>,
    #[account(
        mut,
        seeds = [
            b"collateral",
            buyer_order.market_id.to_le_bytes().as_ref(),
            buyer_order.owner.as_ref()
        ],
        bump = buyer_collateral.bump
    )]
    pub buyer_collateral: Box<Account<'info, UserCollateral>>,
    #[account(
        mut,
        seeds = [
            b"collateral",
            seller_order.market_id.to_le_bytes().as_ref(),
            seller_order.owner.as_ref()
        ],
        bump = seller_collateral.bump
    )]
    pub seller_collateral: Box<Account<'info, UserCollateral>>,
    /// CHECK: Solana instructions sysvar used to verify prior Ed25519 instructions.
    #[account(
        address = anchor_lang::solana_program::sysvar::instructions::ID,
        constraint = buyer_order.market_id == seller_order.market_id @ VaultError::SignedOrderMarketMismatch,
        constraint = buyer_order.market_id == market.market_id @ VaultError::MarketMismatch,
        constraint = buyer_order.market_program_id == *market.to_account_info().owner @ VaultError::SignedOrderProgramMismatch,
        constraint = seller_order.market_program_id == *market.to_account_info().owner @ VaultError::SignedOrderProgramMismatch,
        constraint = buyer_order.vault_program_id == crate::ID @ VaultError::SignedOrderProgramMismatch,
        constraint = seller_order.vault_program_id == crate::ID @ VaultError::SignedOrderProgramMismatch,
        constraint = signed_order_not_expired(&buyer_order) @ VaultError::SignedOrderExpired,
        constraint = signed_order_not_expired(&seller_order) @ VaultError::SignedOrderExpired,
        constraint = signed_order_pair_shape_is_valid(&buyer_order, &seller_order, quantity) @ VaultError::InvalidSignedOrderPair,
        constraint = seller_order.price_scaled <= buyer_order.price_scaled @ VaultError::SignedOrderNotCrossed,
        constraint = signed_order_owner_signature_exists(&instructions.to_account_info(), &buyer_order) @ VaultError::SignedOrderSignatureMissing,
        constraint = signed_order_owner_signature_exists(&instructions.to_account_info(), &seller_order) @ VaultError::SignedOrderSignatureMissing,
        constraint = signed_order_full_signature_exists(&instructions.to_account_info(), &buyer_order) @ VaultError::SignedOrderSignatureMismatch,
        constraint = signed_order_full_signature_exists(&instructions.to_account_info(), &seller_order) @ VaultError::SignedOrderSignatureMismatch
    )]
    pub instructions: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = submitter,
        space = 8 + FillState::INIT_SPACE,
        seeds = [
            b"fill",
            buyer_order.market_id.to_le_bytes().as_ref(),
            buyer_order.owner.as_ref(),
            buyer_order.nonce.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub buyer_fill: Box<Account<'info, FillState>>,
    #[account(
        init_if_needed,
        payer = submitter,
        space = 8 + FillState::INIT_SPACE,
        seeds = [
            b"fill",
            seller_order.market_id.to_le_bytes().as_ref(),
            seller_order.owner.as_ref(),
            seller_order.nonce.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub seller_fill: Box<Account<'info, FillState>>,
    #[account(
        init_if_needed,
        payer = submitter,
        space = 8 + UserPosition::INIT_SPACE,
        seeds = [
            b"position",
            buyer_order.market_id.to_le_bytes().as_ref(),
            buyer_order.owner.as_ref(),
            PositionSide::Long.seed()
        ],
        bump
    )]
    pub buyer_position: Box<Account<'info, UserPosition>>,
    #[account(
        init_if_needed,
        payer = submitter,
        space = 8 + UserPosition::INIT_SPACE,
        seeds = [
            b"position",
            seller_order.market_id.to_le_bytes().as_ref(),
            seller_order.owner.as_ref(),
            PositionSide::Short.seed()
        ],
        bump
    )]
    pub seller_position: Box<Account<'info, UserPosition>>,
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    pub fn as_byte(&self) -> u8 {
        match self {
            OrderSide::Buy => 0,
            OrderSide::Sell => 1,
        }
    }

    pub fn try_from_byte(value: u8) -> Result<Self> {
        match value {
            0 => Ok(OrderSide::Buy),
            1 => Ok(OrderSide::Sell),
            _ => err!(VaultError::InvalidSignedOrderSide),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace, PartialEq, Eq)]
pub enum OrderNetwork {
    MainnetBeta,
    Devnet,
    Localnet,
}

impl OrderNetwork {
    pub fn as_byte(&self) -> u8 {
        match self {
            OrderNetwork::MainnetBeta => 0,
            OrderNetwork::Devnet => 1,
            OrderNetwork::Localnet => 2,
        }
    }

    pub fn try_from_byte(value: u8) -> Result<Self> {
        match value {
            0 => Ok(OrderNetwork::MainnetBeta),
            1 => Ok(OrderNetwork::Devnet),
            2 => Ok(OrderNetwork::Localnet),
            _ => err!(VaultError::InvalidSignedOrderNetwork),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace, PartialEq, Eq)]
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

pub fn encode_signed_order(order: &SignedOrder) -> Result<[u8; SIGNED_ORDER_MESSAGE_LEN]> {
    validate_signed_order_values(order)?;

    let mut buffer = [0u8; SIGNED_ORDER_MESSAGE_LEN];
    let mut offset = 0;
    buffer[offset] = LIME_SIGNED_ORDER_DOMAIN_LEN as u8;
    offset += 1;
    buffer[offset..offset + LIME_SIGNED_ORDER_DOMAIN_LEN].copy_from_slice(LIME_SIGNED_ORDER_DOMAIN);
    offset += LIME_SIGNED_ORDER_DOMAIN_LEN;
    buffer[offset] = order.version;
    offset += 1;
    buffer[offset] = order.network.as_byte();
    offset += 1;
    buffer[offset..offset + 32].copy_from_slice(order.market_program_id.as_ref());
    offset += 32;
    buffer[offset..offset + 32].copy_from_slice(order.vault_program_id.as_ref());
    offset += 32;
    buffer[offset..offset + 8].copy_from_slice(&order.market_id.to_le_bytes());
    offset += 8;
    buffer[offset..offset + 32].copy_from_slice(order.owner.as_ref());
    offset += 32;
    buffer[offset] = order.side.as_byte();
    offset += 1;
    buffer[offset..offset + 8].copy_from_slice(&order.price_scaled.to_le_bytes());
    offset += 8;
    buffer[offset..offset + 8].copy_from_slice(&order.quantity.to_le_bytes());
    offset += 8;
    buffer[offset..offset + 8].copy_from_slice(&order.expiration_ts.to_le_bytes());
    offset += 8;
    buffer[offset..offset + 16].copy_from_slice(&order.nonce.to_le_bytes());
    Ok(buffer)
}

pub fn parse_signed_order(message: &[u8]) -> Result<SignedOrder> {
    require!(
        message.len() == SIGNED_ORDER_MESSAGE_LEN,
        VaultError::InvalidSignedOrderLength
    );
    require!(
        message[0] as usize == LIME_SIGNED_ORDER_DOMAIN_LEN,
        VaultError::InvalidSignedOrderDomain
    );
    require!(
        &message[1..1 + LIME_SIGNED_ORDER_DOMAIN_LEN] == LIME_SIGNED_ORDER_DOMAIN,
        VaultError::InvalidSignedOrderDomain
    );

    let mut offset = 1 + LIME_SIGNED_ORDER_DOMAIN_LEN;
    let version = message[offset];
    offset += 1;
    let network = OrderNetwork::try_from_byte(message[offset])?;
    offset += 1;
    let market_program_id =
        Pubkey::new_from_array(message[offset..offset + 32].try_into().unwrap());
    offset += 32;
    let vault_program_id = Pubkey::new_from_array(message[offset..offset + 32].try_into().unwrap());
    offset += 32;
    let market_id = u64::from_le_bytes(message[offset..offset + 8].try_into().unwrap());
    offset += 8;
    let owner = Pubkey::new_from_array(message[offset..offset + 32].try_into().unwrap());
    offset += 32;
    let side = OrderSide::try_from_byte(message[offset])?;
    offset += 1;
    let price_scaled = u64::from_le_bytes(message[offset..offset + 8].try_into().unwrap());
    offset += 8;
    let quantity = u64::from_le_bytes(message[offset..offset + 8].try_into().unwrap());
    offset += 8;
    let expiration_ts = i64::from_le_bytes(message[offset..offset + 8].try_into().unwrap());
    offset += 8;
    let nonce = u128::from_le_bytes(message[offset..offset + 16].try_into().unwrap());

    Ok(SignedOrder {
        version,
        network,
        market_program_id,
        vault_program_id,
        market_id,
        owner,
        side,
        price_scaled,
        quantity,
        expiration_ts,
        nonce,
    })
}

pub fn validate_signed_order(
    order: &SignedOrder,
    expected_network: OrderNetwork,
    expected_market_program_id: Pubkey,
    expected_vault_program_id: Pubkey,
    expected_market_id: u64,
    now_ts: i64,
) -> Result<()> {
    validate_signed_order_values(order)?;
    require!(order.version == 1, VaultError::InvalidSignedOrderVersion);
    require!(
        order.network == expected_network,
        VaultError::InvalidSignedOrderNetwork
    );
    require!(
        order.market_program_id == expected_market_program_id,
        VaultError::SignedOrderProgramMismatch
    );
    require!(
        order.vault_program_id == expected_vault_program_id,
        VaultError::SignedOrderProgramMismatch
    );
    require!(
        order.market_id == expected_market_id,
        VaultError::SignedOrderMarketMismatch
    );
    require!(order.expiration_ts > now_ts, VaultError::SignedOrderExpired);
    Ok(())
}

fn validate_signed_order_values(order: &SignedOrder) -> Result<()> {
    require!(order.price_scaled <= SCALE, VaultError::InvalidPrice);
    require!(order.quantity > 0, VaultError::InvalidAmount);
    require!(order.nonce > 0, VaultError::InvalidSignedOrderNonce);
    Ok(())
}

fn validate_order_pair(
    buyer_order: &SignedOrder,
    seller_order: &SignedOrder,
    quantity: u64,
    expected_market_program_id: Pubkey,
    expected_vault_program_id: Pubkey,
    now_ts: i64,
) -> Result<()> {
    require!(quantity > 0, VaultError::InvalidAmount);
    validate_signed_order(
        buyer_order,
        OrderNetwork::Localnet,
        expected_market_program_id,
        expected_vault_program_id,
        buyer_order.market_id,
        now_ts,
    )?;
    validate_signed_order(
        seller_order,
        OrderNetwork::Localnet,
        expected_market_program_id,
        expected_vault_program_id,
        buyer_order.market_id,
        now_ts,
    )?;
    require!(
        buyer_order.side == OrderSide::Buy,
        VaultError::InvalidSignedOrderPair
    );
    require!(
        seller_order.side == OrderSide::Sell,
        VaultError::InvalidSignedOrderPair
    );
    require!(
        buyer_order.owner != seller_order.owner,
        VaultError::SelfTradeDisabled
    );
    require!(
        seller_order.price_scaled <= buyer_order.price_scaled,
        VaultError::SignedOrderNotCrossed
    );
    require!(
        quantity <= buyer_order.quantity && quantity <= seller_order.quantity,
        VaultError::SignedOrderOverfilled
    );
    Ok(())
}

fn assert_fill_capacity(fill: &FillState, order: &SignedOrder, quantity: u64) -> Result<()> {
    require!(fill.quantity == order.quantity, VaultError::InvalidSignedOrderPair);
    let new_filled_quantity = fill
        .filled_quantity
        .checked_add(quantity)
        .ok_or(VaultError::MathOverflow)?;
    require!(
        new_filled_quantity <= order.quantity,
        VaultError::SignedOrderOverfilled
    );
    Ok(())
}

fn assert_signed_order_ed25519_verified(
    instructions_account: &AccountInfo,
    order: &SignedOrder,
) -> Result<()> {
    let message = encode_signed_order(order)?;
    let current_index = load_current_index_checked(instructions_account)
        .map_err(|_| error!(VaultError::SignedOrderSignatureMissing))?;
    let mut saw_owner_signature = false;

    for index in 0..current_index {
        let instruction = load_instruction_at_checked(index as usize, instructions_account)
            .map_err(|_| error!(VaultError::SignedOrderSignatureMissing))?;
        if instruction.program_id != pubkey!("Ed25519SigVerify111111111111111111111111111") {
            continue;
        }
        match ed25519_instruction_matches(&instruction.data, order.owner, &message) {
            Ed25519Match::Full => return Ok(()),
            Ed25519Match::OwnerOnly => saw_owner_signature = true,
            Ed25519Match::None => {}
        }
    }

    if saw_owner_signature {
        err!(VaultError::SignedOrderSignatureMismatch)
    } else {
        err!(VaultError::SignedOrderSignatureMissing)
    }
}

fn signed_order_not_expired(order: &SignedOrder) -> bool {
    Clock::get()
        .map(|clock| order.expiration_ts > clock.unix_timestamp)
        .unwrap_or(false)
}

fn signed_order_pair_shape_is_valid(
    buyer_order: &SignedOrder,
    seller_order: &SignedOrder,
    quantity: u64,
) -> bool {
    quantity > 0
        && buyer_order.version == 1
        && seller_order.version == 1
        && buyer_order.network == OrderNetwork::Localnet
        && seller_order.network == OrderNetwork::Localnet
        && buyer_order.side == OrderSide::Buy
        && seller_order.side == OrderSide::Sell
        && buyer_order.owner != seller_order.owner
        && buyer_order.price_scaled <= SCALE
        && seller_order.price_scaled <= SCALE
        && buyer_order.quantity > 0
        && seller_order.quantity > 0
        && buyer_order.nonce > 0
        && seller_order.nonce > 0
        && quantity <= buyer_order.quantity
        && quantity <= seller_order.quantity
}

fn signed_order_owner_signature_exists(
    instructions_account: &AccountInfo,
    order: &SignedOrder,
) -> bool {
    signed_order_signature_match(instructions_account, order, false)
}

fn signed_order_full_signature_exists(
    instructions_account: &AccountInfo,
    order: &SignedOrder,
) -> bool {
    signed_order_signature_match(instructions_account, order, true)
}

fn signed_order_signature_match(
    instructions_account: &AccountInfo,
    order: &SignedOrder,
    require_message: bool,
) -> bool {
    let Ok(message) = encode_signed_order(order) else {
        return false;
    };
    let Ok(current_index) = load_current_index_checked(instructions_account) else {
        return false;
    };

    for index in 0..current_index {
        let Ok(instruction) = load_instruction_at_checked(index as usize, instructions_account)
        else {
            return false;
        };
        if instruction.program_id != pubkey!("Ed25519SigVerify111111111111111111111111111") {
            continue;
        }
        match ed25519_instruction_matches(&instruction.data, order.owner, &message) {
            Ed25519Match::Full => return true,
            Ed25519Match::OwnerOnly if !require_message => return true,
            Ed25519Match::OwnerOnly | Ed25519Match::None => {}
        }
    }
    false
}

enum Ed25519Match {
    Full,
    OwnerOnly,
    None,
}

fn ed25519_instruction_matches(data: &[u8], owner: Pubkey, expected_message: &[u8]) -> Ed25519Match {
    const ED25519_INSTRUCTION_HEADER_LEN: usize = 16;
    const ED25519_CURRENT_INSTRUCTION: u16 = u16::MAX;

    if data.len() < ED25519_INSTRUCTION_HEADER_LEN || data[0] != 1 {
        return Ed25519Match::None;
    }

    let public_key_offset = u16::from_le_bytes([data[6], data[7]]) as usize;
    let public_key_instruction_index = u16::from_le_bytes([data[8], data[9]]);
    let message_offset = u16::from_le_bytes([data[10], data[11]]) as usize;
    let message_size = u16::from_le_bytes([data[12], data[13]]) as usize;
    let message_instruction_index = u16::from_le_bytes([data[14], data[15]]);

    if public_key_instruction_index != ED25519_CURRENT_INSTRUCTION
        || message_instruction_index != ED25519_CURRENT_INSTRUCTION
    {
        return Ed25519Match::None;
    }

    let Some(public_key_end) = public_key_offset.checked_add(32) else {
        return Ed25519Match::None;
    };
    let Some(message_end) = message_offset.checked_add(message_size) else {
        return Ed25519Match::None;
    };
    if public_key_end > data.len() || message_end > data.len() {
        return Ed25519Match::None;
    }
    if &data[public_key_offset..public_key_end] != owner.as_ref() {
        return Ed25519Match::None;
    }
    if &data[message_offset..message_end] == expected_message {
        Ed25519Match::Full
    } else {
        Ed25519Match::OwnerOnly
    }
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
    #[msg("Signed Order message length is invalid")]
    InvalidSignedOrderLength,
    #[msg("Signed Order domain is invalid")]
    InvalidSignedOrderDomain,
    #[msg("Signed Order version is invalid")]
    InvalidSignedOrderVersion,
    #[msg("Signed Order network is invalid")]
    InvalidSignedOrderNetwork,
    #[msg("Signed Order side is invalid")]
    InvalidSignedOrderSide,
    #[msg("Signed Order Program scope does not match")]
    SignedOrderProgramMismatch,
    #[msg("Signed Order Market scope does not match")]
    SignedOrderMarketMismatch,
    #[msg("Signed Order is expired")]
    SignedOrderExpired,
    #[msg("Signed Order nonce is invalid")]
    InvalidSignedOrderNonce,
    #[msg("Signed Order pair is invalid")]
    InvalidSignedOrderPair,
    #[msg("Signed Orders do not cross")]
    SignedOrderNotCrossed,
    #[msg("Signed Order signature verification instruction is missing")]
    SignedOrderSignatureMissing,
    #[msg("Signed Order signature verification instruction does not match")]
    SignedOrderSignatureMismatch,
    #[msg("Signed Order fill exceeds remaining quantity")]
    SignedOrderOverfilled,
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

#[cfg(test)]
mod signed_order_tests {
    use super::*;
    use std::str::FromStr;

    fn sample_order() -> SignedOrder {
        SignedOrder {
            version: 1,
            network: OrderNetwork::Localnet,
            market_program_id: Pubkey::from_str("G2YAvLwHFmd4wgs45QScmBYpFthkEjhU34VKQ3HKMagk")
                .unwrap(),
            vault_program_id: crate::ID,
            market_id: 42,
            owner: Pubkey::from_str("11111111111111111111111111111112").unwrap(),
            side: OrderSide::Buy,
            price_scaled: 620_000,
            quantity: 5_000_000,
            expiration_ts: 1_800_000_000,
            nonce: 0x0102030405060708090a0b0c0d0e0f10,
        }
    }

    #[test]
    fn encodes_signed_order_to_fixed_layout_bytes() {
        let order = sample_order();
        let encoded = encode_signed_order(&order).unwrap();

        assert_eq!(encoded.len(), SIGNED_ORDER_MESSAGE_LEN);
        assert_eq!(encoded[0], LIME_SIGNED_ORDER_DOMAIN_LEN as u8);
        assert_eq!(
            &encoded[1..1 + LIME_SIGNED_ORDER_DOMAIN_LEN],
            LIME_SIGNED_ORDER_DOMAIN
        );
        assert_eq!(encoded[18], 1);
        assert_eq!(encoded[19], 2);
        assert_eq!(u64::from_le_bytes(encoded[84..92].try_into().unwrap()), 42);
        assert_eq!(encoded[124], 0);
        assert_eq!(
            u64::from_le_bytes(encoded[125..133].try_into().unwrap()),
            620_000
        );
        assert_eq!(
            u64::from_le_bytes(encoded[133..141].try_into().unwrap()),
            5_000_000
        );
        assert_eq!(
            i64::from_le_bytes(encoded[141..149].try_into().unwrap()),
            1_800_000_000
        );
        assert_eq!(
            u128::from_le_bytes(encoded[149..165].try_into().unwrap()),
            0x0102030405060708090a0b0c0d0e0f10
        );
    }

    #[test]
    fn parses_signed_order_from_canonical_bytes() {
        let order = sample_order();
        let encoded = encode_signed_order(&order).unwrap();
        let parsed = parse_signed_order(&encoded).unwrap();

        assert_eq!(parsed, order);
    }

    #[test]
    fn validates_signed_order_scope_and_values() {
        let order = sample_order();

        validate_signed_order(
            &order,
            OrderNetwork::Localnet,
            order.market_program_id,
            order.vault_program_id,
            order.market_id,
            1_700_000_000,
        )
        .unwrap();
    }

    #[test]
    fn rejects_invalid_signed_order_fields() {
        let order = sample_order();
        let mut encoded = encode_signed_order(&order).unwrap();

        encoded[0] = 0;
        assert_eq!(
            parse_signed_order(&encoded).unwrap_err(),
            error!(VaultError::InvalidSignedOrderDomain)
        );

        let mut encoded = encode_signed_order(&order).unwrap();
        encoded[19] = 9;
        assert_eq!(
            parse_signed_order(&encoded).unwrap_err(),
            error!(VaultError::InvalidSignedOrderNetwork)
        );

        let mut encoded = encode_signed_order(&order).unwrap();
        encoded[124] = 9;
        assert_eq!(
            parse_signed_order(&encoded).unwrap_err(),
            error!(VaultError::InvalidSignedOrderSide)
        );

        let mut invalid = order;
        invalid.market_id = 7;
        assert_eq!(
            validate_signed_order(
                &invalid,
                OrderNetwork::Localnet,
                order.market_program_id,
                order.vault_program_id,
                order.market_id,
                1_700_000_000,
            )
            .unwrap_err(),
            error!(VaultError::SignedOrderMarketMismatch)
        );

        let mut invalid = order;
        invalid.price_scaled = SCALE + 1;
        assert_eq!(
            validate_signed_order(
                &invalid,
                OrderNetwork::Localnet,
                order.market_program_id,
                order.vault_program_id,
                order.market_id,
                1_700_000_000,
            )
            .unwrap_err(),
            error!(VaultError::InvalidPrice)
        );

        let mut invalid = order;
        invalid.quantity = 0;
        assert_eq!(
            validate_signed_order(
                &invalid,
                OrderNetwork::Localnet,
                order.market_program_id,
                order.vault_program_id,
                order.market_id,
                1_700_000_000,
            )
            .unwrap_err(),
            error!(VaultError::InvalidAmount)
        );

        let mut invalid = order;
        invalid.expiration_ts = 1_699_999_999;
        assert_eq!(
            validate_signed_order(
                &invalid,
                OrderNetwork::Localnet,
                order.market_program_id,
                order.vault_program_id,
                order.market_id,
                1_700_000_000,
            )
            .unwrap_err(),
            error!(VaultError::SignedOrderExpired)
        );

        let mut invalid = order;
        invalid.nonce = 0;
        assert_eq!(
            validate_signed_order(
                &invalid,
                OrderNetwork::Localnet,
                order.market_program_id,
                order.vault_program_id,
                order.market_id,
                1_700_000_000,
            )
            .unwrap_err(),
            error!(VaultError::InvalidSignedOrderNonce)
        );
    }
}
