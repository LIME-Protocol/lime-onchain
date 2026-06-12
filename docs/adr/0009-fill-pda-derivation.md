# Fill PDA Derivation

The Fill PDA is derived as `["fill", market_id, owner, nonce_u128]` in the Vault Program. The seed intentionally excludes side, Price, Quantity, and other order fields so a nonce is uniquely consumable for one owner in one Market, and accidental or malicious nonce reuse fails by colliding with existing fill state. This keeps replay and overfill protection on-chain while preserving parallel execution for different Signed Orders.
