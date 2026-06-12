# Vault Program Validates Signed Orders

Signed Order validation and Fill PDA accounting live in the Vault Program for this cycle. Trade Execution must validate signatures, fill state, Execution Price, Quantity, and Collateral constraints atomically with the Collateral and Position updates that the Vault Program already owns. This avoids splitting order validity from custody effects across Programs, reducing CPI complexity and audit surface until a separate order Program has a clear reason to exist.
