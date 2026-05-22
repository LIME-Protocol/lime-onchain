# MVP End-to-End Flow

The first MVP test should prove the narrow happy path through the protocol.

## First Testable Flow

1. User A deposits Collateral into a Market.
2. User A signs and places a Buy Order.
3. User B deposits Collateral into the same Market.
4. User B signs and places a Sell Order.
5. The Matching Engine matches the Orders into a Trade.
6. The trusted Backend or Matching Engine authority submits Trade Execution on-chain.
7. User A receives or increases a Long Position.
8. User B receives or increases a Short Position.
9. The Resolver submits a Resolution.
10. Settlement makes Payouts available.
11. Users Claim their Payouts.

## Purpose

This flow proves the spine of the MVP:

- Collateral custody
- Off-chain matching
- On-chain Trade Execution
- Long and Short Position accounting
- Resolution
- Settlement
- Claim

## Position Representation

The MVP represents exposure with on-chain Position Accounts, not transferable position tokens.

For each Market, users can hold:

- Long Position Account
- Short Position Account

These Position Accounts track Position Quantity and Cost Basis/accounting data. Settlement payout is based on Position Quantity and the Market Resolution payoff ratio, not on the collateral amount contributed by the user.

The economic relationship is:

- Long payout = `quantity * payoffRatio`
- Short payout = `quantity * (1 - payoffRatio)`
- Long payout + Short payout = `quantity`

Collateral contributed at Trade Execution funds the Market Vault and records Cost Basis, but Cost Basis is not the payout basis.

## Frontend Boundary

The frontend does not perform matching.

For the MVP, the frontend:

- Sends Order actions to the backend/Matching Engine API.
- Signs on-chain transactions when user custody changes, such as Deposit, Withdraw, and Claim.
- Reads on-chain state for Collateral, Positions, Resolution, Settlement, and Claim status.
- Reads off-chain state from the backend/Matching Engine for Order Book, open Orders, and Reserved Collateral.

The browser should not be the source of truth for the Order Book.

The Backend is the frontend-facing API surface. The Matching Engine is the Backend module or service responsible for Order Book, Reserved Collateral, matching, and Trade generation. They may run in the same process for the MVP.

## Order Persistence

Signed Orders and the Order Book are persisted off-chain by the backend/Matching Engine.

The backend should store at least:

- Signed Order payload
- Signature
- Status: open, matched, cancelled, or expired
- Reserved Collateral
- Creation and update timestamps
- Trade ID when matched

Open Orders are not stored on-chain in the MVP.

## Trade ID

Each matched Trade should have a deterministic Trade ID, derived from the matched Signed Orders and fill details.

For the MVP, the Matching Engine must use Trade ID to prevent duplicate execution. Program-level replay protection can be strengthened after the first MVP flow is stable.

## Trade Execution Authority

For the MVP, the Backend or Matching Engine may be a trusted authority that submits Trade Execution on-chain.

The Backend or Matching Engine pays for the on-chain Trade Execution transaction in the MVP. Users do not need to be online at match time after they have signed their Orders.

User custody still depends on:

- Users depositing Collateral on-chain.
- Users signing Orders for a specific Market, side, Price, Quantity, expiration, and replay-protection constraints.
- The Trade Execution authority only executing Trades within those authorized Order limits.

Long term, LIME should reduce trust in the Trade Execution authority with stronger verification, idempotency, and replay protection.

## Signed Order Fields

The MVP Signed Order should include at least:

- `marketId`
- `owner`
- `side`
- `price`
- `quantity`
- `expiration`
- `nonce`
- `maxCollateral`
- `network`
- `orderType`

## Matching Rule

The first MVP should use Fill-or-none matching:

- An Order either matches for its full Quantity or does not match.
- Partial fills are outside the first MVP flow.

Price matching rule:

- Buy Limit Price is the maximum Price the buyer accepts.
- Sell Limit Price is the minimum Price the seller accepts.
- A match is valid when `buy.price >= sell.price`.
- Execution Price is `sell.price`.

## Test Users

The first automated MVP test should use two distinct users:

- **User A**: buyer, receives or increases a Long Position.
- **User B**: seller, receives or increases a Short Position.

Using distinct users proves separate collateral custody, account ownership, and Position accounting. Manual testing by the project partners should happen after this baseline flow is stable.

## Test Market

The automated MVP test should use a deterministic smoke-test Market:

- Resolution Source: `MVP smoke test observed value`
- Lower Bound: `0`
- Upper Bound: `1,000,000`
- Observed Value: `500,000`
- Payoff: linear

Manual partner testing can use a more realistic price Market, such as a BTC/USD price at a specified timestamp with explicit lower and upper bounds.

## Explicitly Out Of First Flow

The first MVP flow should not try to cover every lifecycle path. Keep the first test narrow so it can become the baseline integration test.

Out of the first flow:

- Cancel Order
- Withdraw Available Collateral
- Refund
- Pause Market
- Cancel Market
- Multiple partial fills
- Exit Position
- Netting Long and Short Positions
