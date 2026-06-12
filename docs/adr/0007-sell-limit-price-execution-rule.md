# Sell Limit Price Execution Rule

When a Buy Limit crosses a Sell Limit, the Program validates that the Sell Price is less than or equal to the Buy Price and uses the Sell Limit Price as the deterministic Execution Price. This keeps Execution Price derivable from validated Signed Orders rather than trusted Matching Engine input, and preserves the MVP Collateral arithmetic where the Long side locks `Quantity * Execution Price` and the Short side locks `Quantity * (1 - Execution Price)`.
