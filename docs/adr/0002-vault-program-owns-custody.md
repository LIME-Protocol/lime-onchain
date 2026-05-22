# Vault Program Owns Custody

LIME keeps token custody authority in the Vault Program rather than the Settlement Program. The Vault Program owns the Market Vault token authority, handles deposits, Available Collateral withdrawals, and settlement-authorized transfers; the Settlement Program calculates Resolution-based payouts or refunds and requests transfers from the Vault Program via CPI. This keeps custody and collateral accounting in one module while allowing Settlement to remain focused on Resolution, Claim, and Refund rules.
