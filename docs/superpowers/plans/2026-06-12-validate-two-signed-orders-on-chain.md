# Validate Two Signed Orders On-Chain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` recommended or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close issue #6 by making Vault Trade Execution derive buyer, seller, Quantity, Execution Price, Fill PDAs, and Position updates from two validated Signed Orders.

**Architecture:** Keep Trade Execution in `lime-vault`, because it owns Collateral, Positions, and Fill PDA accounting. `settle_trade` accepts two `SignedOrder` values plus an executed `quantity`; signatures are verified through two Ed25519 verification instructions in the same transaction. Buyer, seller, price, and nonces come only from the validated orders.

**Tech Stack:** Anchor `0.32.1`, Rust Programs, Solana Ed25519 instruction introspection, TypeScript `ts-mocha` integration tests, transitional local SDK harness.

---

## Summary

- Issue #5 is effectively satisfied for this repo: canonical Signed Order structs, serializer/parser, validation helpers, Fill PDA derivation, and Rust rejection tests already exist.
- Issue #6 replaces trusted raw Trade Execution inputs with two validated Signed Orders.
- The Vault Program derives buyer, seller, execution price, Fill PDAs, and Position updates from the order pair.

## Key Changes

- Modify `programs/lime-vault/src/lib.rs` so `settle_trade` accepts `buyer_order: SignedOrder`, `seller_order: SignedOrder`, and `quantity: u64`.
- Add order-pair validation, Ed25519 instruction introspection, and overfill protection before any Collateral, Fill, or Position mutation.
- Modify `sdk/src/types.ts` and `sdk/src/vault.ts` so the transitional harness derives accounts from `buyerOrder` and `sellerOrder`.
- Update `tests/lime.spec.ts` and `tests/collateral.integration.spec.ts` to exercise the signed-order execution path and reject invalid order pairs.

## Task Plan

- [ ] **Task 1: IDL/API red test**
  - Update `tests/lime.spec.ts` to fail until `settle_trade` args are signed-order based.
  - Expected failure: generated IDL still includes raw execution args or SDK still references old fields.

- [ ] **Task 2: Rust instruction shape**
  - Update `SettleTrade` accounts and `settle_trade` args in `programs/lime-vault/src/lib.rs`.
  - Account seeds must derive from `buyer_order` and `seller_order`, not free-form buyer/seller args.
  - Keep `submitter` permissionless and payer for Fill/Position accounts.

- [ ] **Task 3: Order-pair validation**
  - Validate network, Program scope, Market scope, side pairing, non-self-trade, price crossing, quantity, and expiration before mutation.
  - Use seller limit price as the deterministic execution price.

- [ ] **Task 4: Ed25519 signature introspection**
  - Require two Ed25519 verification instructions in the same transaction.
  - Use `encode_signed_order(&order)` as the signed message.
  - Verify each Ed25519 instruction's public key equals the relevant order owner and message equals the canonical encoded order.

- [ ] **Task 5: Fill accounting hardening**
  - Initialize each `FillState.quantity` to the full signed order quantity.
  - Increment `filled_quantity` by executed quantity.
  - Reject replay or overfill when `filled_quantity + quantity > order.quantity`.

- [ ] **Task 6: Integration tests**
  - Happy path preserves existing Collateral and Position behavior.
  - Expired, mismatched, malformed, uncrossed, signature-mismatched, and overfilled orders fail before state changes.

- [ ] **Task 7: SDK harness alignment**
  - Update local SDK trade execution to derive accounts from orders.
  - Keep `encodeSignedOrder` as the canonical local helper until official `lime-sdk` owns it.

## Test Plan

Run:

```bash
cargo test -p lime-vault
npm run build:sdk
anchor build
npm run test:anchor
```

Expected:

- Rust signed-order unit tests pass.
- Generated IDL reflects the new `settle_trade` interface.
- Integration tests prove valid signed orders preserve existing Trade Execution behavior.
- Invalid, expired, mismatched, unsigned, and replayed orders fail before account state changes.

## Assumptions

- This repo's executable test target is localnet, so `settle_trade` validates orders against `OrderNetwork::Localnet`.
- Devnet/mainnet network configuration is out of scope for issue #6 unless a separate protocol config issue is opened.
- Raw signatures are not passed directly to Vault; they are provided through Solana Ed25519 verification instructions in the same transaction.
