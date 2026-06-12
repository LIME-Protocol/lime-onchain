# Signed Orders Support Partial Fills

Signed Orders support Partial Fills at the Program level. Programs must track filled Quantity and reject replay or overfill attempts, while a reference Order Book may still choose a Fill-or-none policy for a simpler first integration. This makes the on-chain protocol useful as infrastructure for real order-book behavior without forcing every integration into full-quantity matching.
