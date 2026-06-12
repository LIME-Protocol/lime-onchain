# Trade Execution Submitter Pays Account Rent

The Trade Execution submitter pays transaction fees and rent for accounts that must be created during execution, including Fill PDAs and Position Accounts. Buyers and sellers authorize execution through Signed Orders and do not need to sign the Trade Execution transaction itself. This supports permissionless execution by reference Matching Engines, relayers, crankers, or integrators without requiring direct wallet interaction from the trading users at fill time.
