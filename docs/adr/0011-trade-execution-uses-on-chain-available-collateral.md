# Trade Execution Uses On-Chain Available Collateral

Trade Execution must validate the user's on-chain Available Collateral at execution time rather than trusting off-chain Reserved Collateral. The Matching Engine may track Reserved Collateral to improve UX and avoid attempting impossible fills, but the Vault Program only changes Positions when the buyer can lock `executed Quantity * Execution Price` and the seller can lock `executed Quantity * (1 - Execution Price)` from Available Collateral atomically. This preserves safety under an untrusted Matching Engine.
