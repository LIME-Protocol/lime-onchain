# Signed Order Signing Domain

Signed Orders must bind the user's authorization to the LIME order protocol, message version, network, Market Program ID, Vault Program ID, Market, owner, side, Price, Quantity, Order Expiration, and Order Nonce. This prevents signatures from being reused across networks, deployments, Markets, owners, sides, or incompatible message versions, and gives the Programs, SDK, and reference Order Book one interoperability contract to serialize and validate.
