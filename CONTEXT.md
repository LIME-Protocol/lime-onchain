# LIME On-Chain

LIME On-Chain is the Solana layer for capped continuous payoff markets. It defines the domain language for market lifecycle, collateral, positions, resolution, and payout.

## Language

**Market**:
A negotiable opportunity in LIME with bounds, a resolution schedule, settlement source, lifecycle status, collateral, and user positions. Users trade markets.
_Avoid_: Contract, smart contract

**Market Creator**:
The actor that creates a Market. In the MVP, Market creation is permissioned and controlled by LIME/Admin.
_Avoid_: Any user as creator in MVP

**Program**:
A Solana on-chain executable that manages part of the LIME protocol. Programs are the Solana equivalent of what EVM developers often call smart contracts.
_Avoid_: Contract

**Contract**:
Reserved only for informal discussion with EVM-native audiences; in LIME domain language, prefer **Market** for the traded object and **Program** for on-chain executable code.
_Avoid_: Using contract to mean both market and program

**Position**:
A user's exposure in a Market for one side of the payoff curve. A user may hold both a **Long Position** and a **Short Position** in the same Market as part of their strategy.
_Avoid_: Treating Position as one net side per user per Market

**Position Account**:
The MVP on-chain representation of a Position. Position Accounts track Long and Short exposure without minting transferable position tokens.
_Avoid_: Position token in the MVP

**Long Position**:
A Position that benefits as the resolved value moves toward the upper bound of the Market payoff curve.
_Avoid_: Buy position

**Short Position**:
A Position that benefits as the resolved value moves toward the lower bound of the Market payoff curve.
_Avoid_: Sell position

**Buy**:
An action or order direction that increases long exposure when filled. Use Buy for order flow, not for naming Positions.
_Avoid_: Long order when describing the action

**Sell**:
An action or order direction that increases short exposure when filled. Use Sell for order flow, not for naming Positions.
_Avoid_: Short order when describing the action

**Collateral**:
Value locked to guarantee that Market payouts can be honored. Collateral is not itself a Position; it backs Positions and settlement obligations.
_Avoid_: Balance, stake

**Market Collateral**:
Collateral locked for a Market as a whole, independent of how a specific user's Long or Short Positions are displayed.
_Avoid_: Position

**Position Collateral**:
The portion of Collateral attributed to a specific Long Position or Short Position. Position Collateral backs that side's payout obligations.
_Avoid_: Market balance

**Cost Basis**:
The collateral amount a user contributed to acquire Position Quantity. Cost Basis is useful for PnL and accounting, but Settlement payout is based on Quantity and Resolution, not Cost Basis.
_Avoid_: Payout basis

**Available Collateral**:
Market Collateral that belongs to a user and is not reserved by open Orders or attributed to Positions. Available Collateral can be used for new Orders.
_Avoid_: Free balance

**Reserved Collateral**:
Market Collateral set aside for open Orders that are eligible to match. Reserved Collateral prevents matchable Orders from producing Trades that cannot be executed on-chain.
_Avoid_: Position Collateral

**Vault**:
The on-chain custody mechanism that holds Collateral for settlement. Use Vault for the domain custody concept, not for every low-level Solana account involved in custody.
_Avoid_: Token account

**Market Vault**:
The Vault for a specific Market. Market Collateral is held by the Market Vault.
_Avoid_: Market balance, collateral account

**Resolution**:
The result data for a Market after its outcome is observed. A Resolution includes the observed value and determines the payoff relationship between Long Positions and Short Positions.
_Avoid_: Settlement

**Settlement**:
The process that applies a Market's Resolution to Positions and makes payouts or refunds available. Settlement follows Resolution; it is not the observed outcome itself.
_Avoid_: Resolution

**Payout**:
The value owed to a Position after Resolution and Settlement rules are applied.
_Avoid_: Refund, withdrawal

**Claim**:
The user's action to receive an available Payout.
_Avoid_: Settlement, withdrawal

**Refund**:
The return of Collateral when a Market is invalidated or cancelled rather than resolved normally.
_Avoid_: Payout

**Withdraw**:
The user's action to remove Available Collateral from a Market Vault. Withdraw applies only to Available Collateral, not to Reserved Collateral or Position Collateral.
_Avoid_: Withdraw position, claim

**Cancel Order**:
The user's action to cancel an open Order before it becomes a Trade. Cancelling an Order releases its Reserved Collateral back into Available Collateral.
_Avoid_: Withdraw order

**Exit Position**:
A future market action that reduces or closes an existing Long Position or Short Position before Resolution. Exit Position is distinct from Withdraw because Position Collateral backs open exposure and cannot be removed directly.
_Avoid_: Withdraw position

**Resolution Source**:
The declared source or method used to observe the outcome of a Market. The Resolution Source explains where the observed value should come from.
_Avoid_: Settlement source

**Resolver**:
The authority or actor that submits a Market's Resolution on-chain based on the Resolution Source.
_Avoid_: Settler, oracle when referring to the actor

**Market Lifecycle**:
The sequence of states a Market moves through from creation to final protocol closure. Lifecycle state describes what the Market allows at that moment.
_Avoid_: Treating user Claims as lifecycle states

**Preliminary**:
A Market that has been created but is not officially open for trading.
_Avoid_: Draft

**Active**:
A Market that is open for trading.
_Avoid_: Open

**Paused**:
A Market where trading is temporarily suspended without ending the Market.
_Avoid_: Cancelled

**Pending Resolution**:
A Market where trading has ended and the protocol is waiting for a Resolution.
_Avoid_: Settling

**Resolved**:
A Market with a recorded Resolution.
_Avoid_: Settled

**Settled**:
A Market whose protocol-level Settlement process is finalized. Individual user Claims may still be pending after the Market is Settled.
_Avoid_: All users claimed

**Cancelled**:
A Market that has been invalidated or stopped before normal Resolution. Cancelled Markets return Collateral through Refunds rather than normal Payouts.
_Avoid_: Paused

**Matching Engine**:
The off-chain system that matches Buy orders with Sell orders for a Market. The Matching Engine determines Trades, but it is not the source of truth for Collateral, Positions, Resolution, or Payouts.
_Avoid_: Settlement engine

**Backend**:
The off-chain API surface used by the frontend. The Backend may host the Matching Engine in the MVP, but not every Backend responsibility is matching.
_Avoid_: Matching Engine when referring to all off-chain APIs

**Order Book**:
The off-chain collection of open Signed Orders for a Market. The Order Book is managed by the Matching Engine and is not stored on-chain in the MVP.
_Avoid_: On-chain market state

**Trade**:
An execution produced when compatible Buy and Sell orders match in a Market. A Trade changes Long and Short Positions and their Position Collateral.
_Avoid_: Settlement

**Trade ID**:
A deterministic identifier for a Trade, derived from the matched Signed Orders and fill details. Trade ID supports idempotency and replay protection.
_Avoid_: Transaction signature

**Trade Execution**:
The act of recording a Trade and updating the affected Positions and Collateral. Trade Execution happens while a Market is Active and is distinct from post-Resolution Settlement.
_Avoid_: Trade settlement

**Trade Execution Authority**:
The actor authorized to submit Trade Execution on-chain. In the MVP, this may be a trusted Backend or Matching Engine authority, provided user Orders authorize the Market, side, Price, and Quantity being executed.
_Avoid_: Resolver, user wallet

**Order**:
An off-chain intent to Buy or Sell exposure in a Market at a specified quantity and price condition. An Order is not a Position; it affects Positions only after it matches into a Trade and that Trade is recorded through Trade Execution.
_Avoid_: Position, transaction

**Limit Order**:
A Signed Order that can only match at a Price acceptable to the user-defined limit. Limit Order is the only Order type in the MVP.
_Avoid_: Market order

**Fill-or-none**:
The MVP matching rule that an Order either matches for its full Quantity or does not match at all.
_Avoid_: Partial fill

**Signed Order**:
An Order authorized by the user's wallet. A Signed Order gives the Matching Engine permission to match and execute only within its declared Market, side, Price, Quantity, expiration, and replay-protection constraints.
_Avoid_: Backend-only order

**Order Expiration**:
The time after which a Signed Order is no longer valid for matching or Trade Execution.
_Avoid_: Market resolution time

**Order Nonce**:
A replay-protection value that makes a Signed Order uniquely consumable within its intended scope.
_Avoid_: Order id when uniqueness depends on signature scope

**Price**:
The fraction of maximum payout that the Long side pays for exposure in a Market. Price is bounded between 0 and 1; implementation may represent it as a scaled integer.
_Avoid_: Spot price

**Execution Price**:
The Price at which a Trade is recorded after matching compatible Orders. In the MVP, when a Buy Limit matches a Sell Limit, Execution Price is the Sell Limit Price.
_Avoid_: Midpoint price

**Quantity**:
The amount of payoff exposure traded in an Order or Trade. Quantity determines the maximum payout exposure before applying Price or Resolution.
_Avoid_: Shares when it implies an equity-like asset

**Long Position Token**:
A possible post-MVP transferable token representing Long Position Quantity in a Market.
_Avoid_: Yes token

**Short Position Token**:
A possible post-MVP transferable token representing Short Position Quantity in a Market.
_Avoid_: No token

## Flagged Ambiguities

**Position cardinality**:
Resolved domain rule: a user may hold simultaneous Long and Short Positions in the same Market. The current on-chain model should not be treated as domain truth if it enforces only one side per user per Market.

**Position representation**:
Resolved MVP rule: represent Long and Short exposure with on-chain Position Accounts, not transferable SPL position tokens. Post-MVP may introduce Long Position Tokens and Short Position Tokens.

**Settlement source**:
Resolved domain rule: use **Resolution Source** for the source of observed outcome data. The current `settlement_source` code field should be treated as implementation naming debt, not domain language.

**Settled state**:
Resolved domain rule: Settled is a protocol-level Market lifecycle state. It does not mean every user has already Claimed their Payout or Refund.

**Trade settlement**:
Resolved domain rule: use **Trade Execution** for recording matched Trades. Reserve **Settlement** for the post-Resolution process that makes Payouts or Refunds available. The current `settle_trade` code name should be treated as implementation naming debt.

**Order collateral**:
Resolved domain rule: an Order should be matchable only after the required Collateral is deposited and reserved. When a matched Trade is recorded through Trade Execution, Reserved Collateral becomes Position Collateral.

**Reserved Collateral source of truth**:
Resolved MVP rule: Reserved Collateral is tracked by the off-chain Matching Engine, while Market Collateral custody remains on-chain. Available Collateral is derived from deposited Market Collateral minus Reserved Collateral and Position Collateral.

**Sell semantics**:
Resolved MVP rule: a filled Sell Order opens or increases a Short Position. It does not close or reduce an existing Long Position.

**Withdraw semantics**:
Resolved MVP rule: users may Withdraw Available Collateral, and Cancel Orders to release Reserved Collateral. Users may not directly Withdraw Position Collateral while exposure is open; releasing Position Collateral before Resolution requires an Exit Position flow.

**Exit Position scope**:
Resolved MVP rule: Exit Position is outside the initial MVP scope. Preserve the term for post-MVP design, but do not treat Buy or Sell Orders as closing existing Positions in the MVP.

**Resolver scope**:
Resolved MVP rule: the initial Resolver may be a LIME-controlled operational authority. Long term, Resolution should be administered through an external Oracle to improve market impartiality.

**Resolution Source format**:
Resolved MVP rule: Resolution Source may be a human-readable description of the observation criterion. A structured schema can be introduced later for Oracle integration.

**Trade Execution authority scope**:
Resolved MVP rule: the Backend or Matching Engine may be trusted to submit Trade Execution on-chain, but Trades must be based on Signed Orders. User custody starts from wallet-authorized Orders and on-chain Collateral deposits; long term, stronger verification should limit what the authority can execute.

**Signed Order fields**:
Resolved MVP rule: a Signed Order should identify Market, owner, side, Price, Quantity, Order Expiration, Order Nonce, maximum Collateral authorized, network, and order type. The first MVP may support only limit Orders.

**Payout basis**:
Resolved domain rule: Settlement payout is based on Position Quantity and the Market's Resolution payoff ratio. Cost Basis or collateral contributed should not be used as the payout basis.

**Order persistence**:
Resolved MVP rule: Signed Orders and the Order Book are persisted off-chain by the Matching Engine or backend. On-chain state records custody, Positions, Resolution, Settlement, Claims, and Refunds, not open Orders.

**Trade ID scope**:
Resolved MVP rule: each Trade should have a deterministic Trade ID. The Matching Engine must prevent duplicate execution by Trade ID; Program-level replay protection can be strengthened after the first MVP path.

**Backend and Matching Engine boundary**:
Resolved MVP rule: Backend is the general off-chain API used by the frontend; Matching Engine is the Backend module or service responsible for Order Book, Reserved Collateral, matching, and Trade generation. They may run in the same process in the MVP.

**Market creation scope**:
Resolved MVP rule: Market creation is permissioned and controlled by LIME/Admin. Permissionless or third-party Market creation is post-MVP.

**Market initial status**:
Resolved MVP rule: every Market starts as Preliminary and requires explicit LIME/Admin activation before Orders or Trade Execution are allowed.

**Minimum Participants**:
A future Market activation rule based on participant count. In the MVP, Minimum Participants is not an active domain rule and Markets use manual LIME/Admin activation.
_Avoid_: Treating participant count as defined before the participant concept is resolved

**Order type scope**:
Resolved MVP rule: Limit Order is the only supported Order type in the MVP. Market Order and slippage-based execution are post-MVP concerns.

**Fill scope**:
Resolved MVP rule: the first MVP uses Fill-or-none matching. Partial fills are post-MVP.

**MVP price matching**:
Resolved MVP rule: a Buy Limit can match a Sell Limit when the Buy Price is greater than or equal to the Sell Price. The resulting Execution Price is the Sell Price.

## Example Dialogue

Developer: "Does the user trade a contract or a market?"

Domain expert: "The user trades a Market. The on-chain logic that manages it is a Solana Program."

Developer: "Who creates Markets in the MVP?"

Domain expert: "LIME/Admin creates Markets. Permissionless Market creation is a post-MVP design topic."

Developer: "Can a newly created Market accept Orders immediately?"

Domain expert: "No. It starts as Preliminary and must be explicitly activated before trading."

Developer: "So if we say contract, we risk confusing the economic market with the executable program?"

Domain expert: "Exactly. Use Market for the user-facing/traded concept and Program for the Solana executable."

Developer: "Can a user be both long and short in the same Market?"

Domain expert: "Yes. Long and Short are separate Positions. Buy and Sell describe order actions; Long Position and Short Position describe exposure."

Developer: "Are Positions tokenized in the MVP?"

Domain expert: "No. The MVP uses Position Accounts. Long and Short Position Tokens are a possible post-MVP extension."

Developer: "Is collateral the same thing as a position?"

Domain expert: "No. Collateral is the value locked to back payouts. A Position is exposure, and Position Collateral is the collateral attributed to that exposure."

Developer: "Where does Market Collateral live?"

Domain expert: "In the Market Vault. The low-level token account is an implementation detail of the Solana Program."

Developer: "Is settlement the same thing as resolution?"

Domain expert: "No. Resolution records the observed outcome for a Market. Settlement applies that Resolution so users can Claim Payouts or receive Refunds."

Developer: "Who decides the observed value?"

Domain expert: "The Resolver submits the Resolution using the Market's Resolution Source."

Developer: "Is the Resolver always controlled by LIME?"

Domain expert: "Only for the MVP. Long term, market Resolution should move to an external Oracle for impartiality."

Developer: "Does the Resolution Source need to be machine-readable in the MVP?"

Domain expert: "No. For the MVP it can be a human-readable observation criterion. Structured source schemas belong to Oracle integration later."

Developer: "Who signs Trade Execution?"

Domain expert: "For the MVP, a trusted Backend or Matching Engine authority can submit Trade Execution, but it must execute within wallet-signed Orders."

Developer: "Does Settled mean every user already claimed?"

Domain expert: "No. Settled means the Market is finalized at the protocol level. User Claims can still be pending."

Developer: "Does the Matching Engine settle the Market?"

Domain expert: "No. The Matching Engine produces Trades. Trade Execution records those Trades. Settlement happens later, after Resolution."

Developer: "Where do open Orders live?"

Domain expert: "In the off-chain Order Book managed by the Matching Engine. They are not stored on-chain in the MVP."

Developer: "Is the Backend the same thing as the Matching Engine?"

Domain expert: "Not conceptually. The Backend is the API surface; the Matching Engine is the part that manages Orders and produces Trades. They may run together in the MVP."

Developer: "How do we prevent the same matched Trade from executing twice?"

Domain expert: "Each Trade gets a deterministic Trade ID. The Matching Engine must enforce idempotency by Trade ID, and on-chain protection can be strengthened after the first MVP."

Developer: "When does an Order become exposure?"

Domain expert: "Only after it matches into a Trade and Trade Execution records it. Until then, it is just off-chain intent."

Developer: "What must a Signed Order authorize?"

Domain expert: "At minimum: Market, owner, side, Price, Quantity, expiration, nonce, maximum Collateral, network, and order type."

Developer: "Can an Order match before collateral exists?"

Domain expert: "No. Collateral should be deposited first, reserved for open Orders, then converted into Position Collateral when Trade Execution records the matched Trade."

Developer: "Does every open Order need an on-chain reservation?"

Domain expert: "No, not for MVP. The Matching Engine tracks Reserved Collateral off-chain, while on-chain Market Collateral remains the custody limit."

Developer: "What does Price mean in a Trade?"

Domain expert: "Price is the fraction of maximum payout paid by the Long side. The Short side backs the complement."

Developer: "Does Settlement pay based on collateral contributed?"

Domain expert: "No. Settlement pays based on Position Quantity and the Resolution payoff ratio. Collateral contributed is Cost Basis, not payout basis."

Developer: "At what Price does a matched Trade execute?"

Domain expert: "For the MVP, Buy can match Sell when Buy Price is at least Sell Price, and the Execution Price is the Sell Price."

Developer: "Does Sell mean selling an existing Long Position?"

Domain expert: "No, not in the MVP. Sell means opening or increasing Short exposure."

Developer: "Can a user withdraw from a Position?"

Domain expert: "No. They can Withdraw Available Collateral or Cancel an Order to release Reserved Collateral. Position Collateral stays locked unless a separate Exit Position flow reduces the exposure."
