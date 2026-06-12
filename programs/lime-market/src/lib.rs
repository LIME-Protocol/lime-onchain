use anchor_lang::prelude::*;

declare_id!("G2YAvLwHFmd4wgs45QScmBYpFthkEjhU34VKQ3HKMagk");

const SETTLEMENT_SOURCE_MAX_LEN: usize = 128;

#[program]
pub mod lime_market {
    use super::*;

    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        fee_bps: u16,
    ) -> Result<()> {
        require!(fee_bps <= 10_000, MarketError::InvalidFeeBps);

        let config = &mut ctx.accounts.protocol_config;
        config.admin = ctx.accounts.admin.key();
        config.fee_bps = fee_bps;
        config.paused = false;
        config.bump = ctx.bumps.protocol_config;
        Ok(())
    }

    pub fn initialize_protocol_for(
        ctx: Context<InitializeProtocol>,
        fee_bps: u16,
        protocol_admin: Pubkey,
    ) -> Result<()> {
        require!(fee_bps <= 10_000, MarketError::InvalidFeeBps);

        let config = &mut ctx.accounts.protocol_config;
        config.admin = protocol_admin;
        config.fee_bps = fee_bps;
        config.paused = false;
        config.bump = ctx.bumps.protocol_config;
        Ok(())
    }

    pub fn update_protocol_admin(
        ctx: Context<UpdateProtocolConfig>,
        new_admin: Pubkey,
    ) -> Result<()> {
        require!(
            ctx.accounts.admin.key() == ctx.accounts.protocol_config.admin,
            MarketError::Unauthorized
        );
        ctx.accounts.protocol_config.admin = new_admin;
        Ok(())
    }

    pub fn pause_protocol(ctx: Context<UpdateProtocolConfig>) -> Result<()> {
        require!(
            ctx.accounts.admin.key() == ctx.accounts.protocol_config.admin,
            MarketError::Unauthorized
        );
        ctx.accounts.protocol_config.paused = true;
        Ok(())
    }

    pub fn resume_protocol(ctx: Context<UpdateProtocolConfig>) -> Result<()> {
        require!(
            ctx.accounts.admin.key() == ctx.accounts.protocol_config.admin,
            MarketError::Unauthorized
        );
        ctx.accounts.protocol_config.paused = false;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_market(
        ctx: Context<CreateMarket>,
        market_id: u64,
        lower_bound: u64,
        upper_bound: u64,
        resolution_ts: i64,
        settlement_source: String,
        payoff_type: PayoffType,
        min_participants: u16,
    ) -> Result<()> {
        let protocol = &ctx.accounts.protocol_config;
        require!(!protocol.paused, MarketError::ProtocolPaused);
        require!(
            ctx.accounts.admin.key() == protocol.admin,
            MarketError::Unauthorized
        );
        require!(resolution_ts > Clock::get()?.unix_timestamp, MarketError::InvalidResolutionTs);
        require!(lower_bound < upper_bound, MarketError::InvalidBounds);
        require!(payoff_type == PayoffType::Linear, MarketError::UnsupportedPayoffType);
        require!(
            settlement_source.len() <= SETTLEMENT_SOURCE_MAX_LEN,
            MarketError::SettlementSourceTooLong
        );

        let market = &mut ctx.accounts.market;
        market.market_id = market_id;
        market.admin = ctx.accounts.admin.key();
        market.lower_bound = lower_bound;
        market.upper_bound = upper_bound;
        market.resolution_ts = resolution_ts;
        market.settlement_source = settlement_source;
        market.payoff_type = payoff_type;
        market.min_participants = min_participants;
        market.participant_count = 0;
        market.status = MarketStatus::Preliminary;
        market.total_long = 0;
        market.total_short = 0;
        market.bump = ctx.bumps.market;

        emit!(MarketCreated {
            market: market.key(),
            market_id,
        });
        Ok(())
    }

    pub fn activate_market(ctx: Context<AdminMarketAction>) -> Result<()> {
        let protocol = &ctx.accounts.protocol_config;
        require!(
            ctx.accounts.admin.key() == protocol.admin,
            MarketError::Unauthorized
        );
        let market = &mut ctx.accounts.market;
        require!(
            market.status == MarketStatus::Preliminary,
            MarketError::InvalidStatusTransition
        );
        require!(
            market.participant_count >= market.min_participants,
            MarketError::InsufficientParticipants
        );
        market.status = MarketStatus::Active;
        Ok(())
    }

    pub fn force_activate_market(ctx: Context<AdminMarketAction>) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let protocol = &ctx.accounts.protocol_config;
        require!(
            ctx.accounts.admin.key() == protocol.admin,
            MarketError::Unauthorized
        );
        require!(
            market.status == MarketStatus::Preliminary,
            MarketError::InvalidStatusTransition
        );
        market.status = MarketStatus::Active;
        Ok(())
    }

    pub fn pause_market(ctx: Context<AdminMarketAction>) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let protocol = &ctx.accounts.protocol_config;
        require!(
            ctx.accounts.admin.key() == protocol.admin,
            MarketError::Unauthorized
        );
        require!(
            market.status == MarketStatus::Active,
            MarketError::InvalidStatusTransition
        );
        market.status = MarketStatus::Paused;
        Ok(())
    }

    pub fn resume_market(ctx: Context<AdminMarketAction>) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let protocol = &ctx.accounts.protocol_config;
        require!(
            ctx.accounts.admin.key() == protocol.admin,
            MarketError::Unauthorized
        );
        require!(
            market.status == MarketStatus::Paused,
            MarketError::InvalidStatusTransition
        );
        market.status = MarketStatus::Active;
        Ok(())
    }

    pub fn close_market(ctx: Context<AdminMarketAction>) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let protocol = &ctx.accounts.protocol_config;
        require!(
            ctx.accounts.admin.key() == protocol.admin,
            MarketError::Unauthorized
        );
        require!(
            market.status == MarketStatus::Active || market.status == MarketStatus::Paused,
            MarketError::InvalidStatusTransition
        );
        market.status = MarketStatus::PendingResolution;
        Ok(())
    }

    pub fn mark_resolved(ctx: Context<AdminMarketAction>) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let protocol = &ctx.accounts.protocol_config;
        require!(
            ctx.accounts.admin.key() == protocol.admin,
            MarketError::Unauthorized
        );
        require!(
            market.status == MarketStatus::PendingResolution,
            MarketError::InvalidStatusTransition
        );
        market.status = MarketStatus::Resolved;
        Ok(())
    }

    pub fn mark_settled(ctx: Context<AdminMarketAction>) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let protocol = &ctx.accounts.protocol_config;
        require!(
            ctx.accounts.admin.key() == protocol.admin,
            MarketError::Unauthorized
        );
        require!(
            market.status == MarketStatus::Resolved || market.status == MarketStatus::Cancelled,
            MarketError::InvalidStatusTransition
        );
        market.status = MarketStatus::Settled;
        Ok(())
    }

    pub fn cancel_market(ctx: Context<AdminMarketAction>) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let protocol = &ctx.accounts.protocol_config;
        require!(
            ctx.accounts.admin.key() == protocol.admin,
            MarketError::Unauthorized
        );
        require!(
            market.status != MarketStatus::Settled && market.status != MarketStatus::Resolved,
            MarketError::InvalidStatusTransition
        );
        market.status = MarketStatus::Cancelled;
        Ok(())
    }

    pub fn increment_participants(ctx: Context<AdminMarketAction>) -> Result<()> {
        let protocol = &ctx.accounts.protocol_config;
        require!(
            ctx.accounts.admin.key() == protocol.admin,
            MarketError::Unauthorized
        );
        let market = &mut ctx.accounts.market;
        market.participant_count = market
            .participant_count
            .checked_add(1)
            .ok_or(MarketError::ParticipantOverflow)?;
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
#[instruction(market_id: u64)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(
        init,
        payer = admin,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,
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
pub struct AdminMarketAction<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(mut)]
    pub market: Account<'info, Market>,
}

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub admin: Pubkey,
    pub fee_bps: u16,
    pub paused: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Market {
    pub market_id: u64,
    pub admin: Pubkey,
    pub lower_bound: u64,
    pub upper_bound: u64,
    pub resolution_ts: i64,
    #[max_len(SETTLEMENT_SOURCE_MAX_LEN)]
    pub settlement_source: String,
    pub payoff_type: PayoffType,
    pub status: MarketStatus,
    pub min_participants: u16,
    pub participant_count: u16,
    pub total_long: u64,
    pub total_short: u64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum PayoffType {
    Linear,
    Sigmoid,
    Step,
    Convex,
    Concave,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum MarketStatus {
    Preliminary,
    Active,
    Paused,
    PendingResolution,
    Resolved,
    Settled,
    Cancelled,
}

#[event]
pub struct MarketCreated {
    pub market: Pubkey,
    pub market_id: u64,
}

#[error_code]
pub enum MarketError {
    #[msg("Unauthorized action")]
    Unauthorized,
    #[msg("Invalid lower/upper bounds")]
    InvalidBounds,
    #[msg("Invalid market status transition")]
    InvalidStatusTransition,
    #[msg("Protocol is paused")]
    ProtocolPaused,
    #[msg("Settlement source exceeds the maximum length")]
    SettlementSourceTooLong,
    #[msg("Minimum participants not reached")]
    InsufficientParticipants,
    #[msg("Fee bps must be <= 10_000")]
    InvalidFeeBps,
    #[msg("Resolution timestamp must be in the future")]
    InvalidResolutionTs,
    #[msg("Only linear payoff type is supported for MVP")]
    UnsupportedPayoffType,
    #[msg("Participant count overflow")]
    ParticipantOverflow,
}
