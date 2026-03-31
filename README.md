# LIME On-Chain

Solana on-chain layer for LIME's capped continuous payoff markets.

## Scope

- Market lifecycle program (`lime-market`)
- Vault and collateral program (`lime-vault`)
- Settlement and payout program (`lime-settlement`)
- TypeScript SDK (`sdk/`) to plug into `lime-mvp`

## Architecture

- Matching remains off-chain
- Collateral, positions, resolution and payout are on-chain
- Fully collateralized contracts with bounded payoff in `[L, U]`

## Quick start

1. Install dependencies:
   - Node.js + npm
   - Rust + cargo
   - Solana CLI
   - Anchor CLI
2. Install JS dependencies:
   - `npm install`
3. Build programs:
   - `anchor build`
4. Run tests:
   - `anchor test`

## Program layout

- `programs/lime-market`: create/activate/pause/close/cancel markets
- `programs/lime-vault`: lock/release collateral and settle trade accounting
- `programs/lime-settlement`: submit resolution, calculate claims, claim payout

## SDK

`sdk/` exports:

- `SolanaWalletProvider`
- `SolanaMarketService`
- `SolanaCollateral`
- `SolanaSettlement`

These interfaces map to the frontend abstractions in `lime-mvp/src/services/wallet.ts`.
