# Fee Collection Deferred

Programs may keep or introduce configuration fields needed to support a future LIME fee, but Trade Execution should not collect protocol fees in this signed-order refactor cycle. Fee collection is deferred because it changes Collateral arithmetic, relayer incentives, payout accounting, and audit scope. The first cycle should make the Programs safe and extensible for fees without making fee monetization part of the critical path.
