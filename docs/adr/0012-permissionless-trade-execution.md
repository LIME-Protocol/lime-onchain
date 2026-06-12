# Permissionless Trade Execution

Trade Execution is permissionless: any actor may submit a transaction that executes two compatible Signed Orders. The Vault Program must reject invalid signatures, expired Orders, replay or overfill attempts, invalid crossing, invalid Execution Price, and insufficient Available Collateral before changing Positions. This lets the reference Matching Engine, third-party relayers, crankers, or integrators submit valid matches without making backend authorization part of the protocol trust model.
