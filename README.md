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
- MVP supports `linear` payoff only

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
   - `npm run test:anchor`
5. Build SDK:
   - `npm run build:sdk`
6. Bootstrap devnet deployment + smoke market:
   - `npm run bootstrap:devnet`

## Program layout

- `programs/lime-market`: protocol admin + market lifecycle
- `programs/lime-vault`: collateral lock + position accounting
- `programs/lime-settlement`: authorized resolution + claim/refund

## SDK

`sdk/` exports:

- `SolanaWalletProvider`
- `SolanaMarketService`
- `SolanaCollateral`
- `SolanaSettlement`

These interfaces map to the frontend abstractions in `lime-mvp/src/services/wallet.ts`.

## Devnet runbook

1. Set wallet and RPC if needed:
   - `export SOLANA_KEYPAIR=~/.config/solana/id.json`
   - `export SOLANA_RPC_URL=https://api.devnet.solana.com`
2. To redeploy to fresh program IDs, rotate local program keypairs:
   - `npm run rotate:program-ids`
3. Set the browser wallet that should control admin UI actions:
   - `export PROTOCOL_ADMIN=<phantom-or-solflare-admin-pubkey>`
   - Optional: `export SETTLEMENT_RESOLVER=<resolver-pubkey>`
4. Run bootstrap:
   - `npm run bootstrap:devnet`
5. Copy the printed `VITE_...` vars to the frontend `.env`.

When `PROTOCOL_ADMIN` differs from the deploy wallet, bootstrap initializes protocol
authority to that browser wallet and skips the smoke market, because market creation
requires the protocol admin signature. Create and activate the first market from the
admin UI after copying the new program IDs.
