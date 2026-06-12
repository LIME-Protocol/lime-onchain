---
status: superseded by ADR-0003
---

# MVP Matching Engine Trust Model

For the MVP, LIME keeps the Order Book and Reserved Collateral accounting off-chain in the Backend/Matching Engine, while custody, Positions, Resolution, Settlement, Claims, and Refunds remain on-chain. Users authorize intent with wallet-signed Orders, and the trusted Backend/Matching Engine matches those Orders, assigns deterministic Trade IDs for idempotency, pays for on-chain Trade Execution, and submits Trades only within the limits authorized by the Signed Orders. This keeps the first MVP fast and testable without moving matching on-chain, while leaving clear post-MVP work to reduce trust through stronger Program verification, replay protection, fee/cranker design, and Oracle-backed Resolution.
