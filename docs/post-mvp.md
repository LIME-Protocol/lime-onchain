# Post-MVP Roadmap Notes

These items are intentionally outside the first MVP integration path, but the domain language should preserve space for them.

## Third-Party Market Creation

The MVP keeps Market creation permissioned under LIME/Admin control. Post-MVP, LIME can evaluate opening Market creation to external creators.

Open design questions:

- Should Market creators be allowlisted, permissionless, or subject to approval?
- Do Market creators need to stake Collateral or pay creation fees?
- Who reviews Resolution Sources for ambiguity or manipulation risk?
- Can external creators choose collateral mints, fees, and bounds?
- What moderation or dispute process exists for poorly specified Markets?

## Minimum Participants Activation

The MVP uses manual LIME/Admin activation and does not enforce Minimum Participants. Post-MVP, LIME can define participant-based activation if it improves market quality.

Open design questions:

- Does a participant mean a user who deposited Collateral, placed an Order, or received a Position?
- Should Long and Short sides both need minimum participation?
- Does Minimum Participants protect liquidity, market validity, or both?
- What happens to Collateral and Orders if a Market never reaches Minimum Participants?

## Exit Position

The MVP allows users to open or increase Long Positions and Short Positions, but it does not model explicit reduction or closure of existing Positions before Resolution.

Post-MVP, LIME should define an **Exit Position** flow that lets users reduce or close exposure before Resolution without treating Position Collateral as directly withdrawable.

Open design questions:

- Should exiting a Long Position require matching against new Buy/Sell liquidity, or should it net against the user's own Short Position first?
- How is realized PnL represented before Resolution?
- When does Position Collateral become Available Collateral after an exit?
- Should Exit Position be implemented as a distinct order type, a distinct trade type, or a higher-level Matching Engine action?

## Net Exposure

The MVP may display net exposure in the frontend, but the domain model preserves Long Positions and Short Positions as separate Positions.

Post-MVP, LIME should define whether net exposure is only a UI/read-model concept or whether it can affect collateral requirements, risk, and exit behavior.

Open design questions:

- Is net exposure calculated per user per Market as `Long Position - Short Position`?
- Can opposing Long and Short Positions reduce collateral requirements?
- If collateral requirements can be reduced, which Program or off-chain service is responsible for enforcing that safely?

## Tokenized Long And Short Positions

The MVP represents exposure with on-chain Position Accounts rather than transferable position tokens.

Post-MVP, LIME can evaluate minting transferable position tokens for each Market:

- Long Position Token
- Short Position Token

These would be conceptually similar to binary prediction market Yes/No tokens, but adapted to LIME's continuous payoff curve. Long and Short Position Tokens would be complementary claims whose payouts sum to the Market Quantity at Settlement.

Open design questions:

- Should position tokens use SPL Token or Token-2022?
- Are Long and Short Position Tokens transferable between users before Resolution?
- Does token transfer require updating any off-chain Order Book or read model?
- At Settlement, does the user burn tokens to Claim Payout?
- How are token mints created, named, and closed per Market?
- What compliance or UX implications come from making Positions wallet-held transferable assets?

## Position Collateral Release

The MVP allows users to Withdraw Available Collateral and Cancel Orders to release Reserved Collateral. It does not allow direct withdrawal of Position Collateral while exposure is open.

Post-MVP, LIME should define the safe conditions under which Position Collateral can become Available Collateral before Resolution.

Open design questions:

- Does Position Collateral release require a matched exit trade?
- Can release happen through self-netting when a user holds both Long and Short Positions?
- What prevents users from releasing collateral that still backs payout obligations?

## Order Cancellation

The first MVP flow does not need to support cancelling open Orders. Post-MVP, LIME should define how users cancel Orders and how Reserved Collateral returns to Available Collateral.

Open design questions:

- Is Order Cancellation purely off-chain in the Matching Engine?
- Does cancellation need an audit trail or event stream for the frontend?
- How are race conditions handled when an Order is cancelled at the same time it is matched?

## Available Collateral Withdrawals

The first MVP flow does not need to support withdrawing Available Collateral, but this should be added soon after the happy path works.

Open design questions:

- Does the Matching Engine need to approve or attest available balance before withdrawal?
- How does the frontend show Available Collateral versus Reserved Collateral versus Position Collateral?
- Does withdrawal require checking pending off-chain Orders to avoid withdrawing collateral that should be reserved?

## Refunds And Invalid Markets

The first MVP flow covers normal Resolution and Claim, not invalidation or Refunds.

Open design questions:

- Who can Cancel a Market and under what conditions?
- How does the frontend communicate Refund availability distinctly from Payout availability?
- Can users Refund individually after cancellation, or does cancellation require a protocol-level settlement step first?

## Market Controls

The first MVP flow does not cover operational controls such as Pause Market, Resume Market, or Cancel Market.

Open design questions:

- Which actor can pause or cancel a Market?
- What happens to open Orders when a Market is paused?
- What happens to Reserved Collateral when a Market is cancelled?

## Partial Fills

The first MVP flow can assume a full match. Partial fills should be designed after the first happy path is stable.

Open design questions:

- How does the Matching Engine represent remaining Order quantity?
- Is Reserved Collateral released incrementally as fills happen?
- Can one Order produce many Trades against many counterparties?

## Market Orders And Slippage

The MVP supports only Limit Orders. Market Orders require slippage controls and clearer UX around execution price, especially in low-liquidity Markets.

Open design questions:

- What slippage control does a Market Order require?
- Should Market Orders be represented as aggressive Limit Orders with explicit bounds?
- How does the frontend explain execution risk in low-liquidity Markets?

## External Oracle Resolution

The MVP can use a LIME-controlled operational Resolver so the first Resolution and Claim flow can be tested without blocking on oracle integration.

Post-MVP, LIME should move Market Resolution toward an external Oracle or equivalent impartial mechanism.

Open design questions:

- Which Oracle provider or mechanism should supply observed values?
- How is the Resolution Source represented so users can evaluate it before trading?
- Does the protocol need one Resolver, multiple Resolvers, or a quorum?
- What dispute or correction mechanism exists if an Oracle submits an incorrect Resolution?

## Structured Resolution Sources

The MVP can treat Resolution Source as human-readable text. Post-MVP Oracle integration will likely require a structured Resolution Source schema.

Open design questions:

- What source kinds are supported: price feed, event outcome, API result, manual attestation, or other?
- Which fields are required for price feeds: base asset, quote asset, venue, timestamp, aggregation window?
- How does the frontend display structured Resolution Sources in user-friendly language?
- Can old text-based Resolution Sources coexist with structured ones?

## More Trust-Minimized Trade Execution

The MVP requires Signed Orders, but can still use a trusted Backend or Matching Engine authority to submit Trade Execution on-chain. Post-MVP, LIME should further reduce the authority's ability to execute anything outside user intent.

Open design questions:

- How much of a Signed Order should the Program verify on-chain during Trade Execution?
- How are signatures, nonces, expirations, and replay protection represented in the durable protocol format?
- Does each Trade need a deterministic ID to prevent duplicate execution?
- Can the Program verify enough of the Order constraints on-chain without making matching itself on-chain?

## Program-Level Trade Replay Protection

The MVP requires deterministic Trade IDs and backend idempotency. Post-MVP, LIME should decide whether the Program also stores or validates consumed Trade IDs.

Open design questions:

- Should each Trade ID create an on-chain receipt?
- Can replay protection be enforced without unbounded on-chain storage?
- Should replay protection be per Market, per user pair, or global?

## Trade Execution Fees And Crankers

The MVP lets the Backend or Matching Engine pay for on-chain Trade Execution. Post-MVP, LIME should decide whether this remains a platform cost or becomes an explicit fee/cranker system.

Open design questions:

- Should users pay an explicit execution fee?
- Should the Matching Engine recover transaction costs through protocol fees?
- Can independent crankers submit Trade Execution transactions?
- If independent crankers exist, what prevents duplicate execution or malicious ordering?
- Are fees charged per Order, per Trade, or as part of Market-level protocol fees?
