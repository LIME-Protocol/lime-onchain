# SDK as Market Operator Integration Kit

The official `@lime/solana` SDK is primarily a Market Operator integration kit, with helpers for both frontend wallets and operator backends. It should serialize Signed Orders, support wallet signing, expose custody actions such as Deposit, Withdraw, and Claim, and help operator systems parse Orders, derive PDAs, construct Trade Execution transactions, and read Markets, Positions, and settlement state. This reflects LIME's product surface as on-chain infrastructure consumed by third-party Frontends and Backends.
