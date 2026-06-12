# Ed25519 Signed Order Introspection Hardening Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` recommended or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close issue #7 by hardening Vault Trade Execution's Ed25519 instruction introspection so each Signed Order is matched to a defensively parsed native Ed25519 verification instruction.

**Architecture:** Keep verification inside `lime-vault`, before Trade Execution mutates Fill, Collateral, or Position accounts. Replace the current loose Ed25519 scan with a small parser that validates the native instruction layout: one signature, zero padding, signature/pubkey/message offsets in bounds, all offset instruction indexes pointing to the current Ed25519 instruction, exact pubkey match, exact canonical message match, and exact message length.

**Tech Stack:** Anchor `0.32.1`, Rust Program helpers/tests, Solana native Ed25519 instruction layout, TypeScript `ts-mocha` localnet integration tests.

---

## Summary

Issue #7 is partly covered by the issue #6 work already on `main`: `settle_trade` already requires Ed25519 instructions and checks owner/message matches. This issue narrows the remaining work to audit-grade parsing and adversarial tests for forged or malformed Ed25519 instruction metadata.

## Key Changes

- In `programs/lime-vault/src/lib.rs`, harden `ed25519_instruction_matches` to parse and validate the full native Ed25519 instruction header.
- Preserve two-stage error semantics: missing usable owner signature maps to `SignedOrderSignatureMissing`; matching owner with wrong message maps to `SignedOrderSignatureMismatch`.
- Add Rust unit tests for malformed instruction data that cannot be represented as successful transactions.
- Extend TypeScript integration helper options and adversarial localnet tests for wrong pubkey, wrong message, missing instruction, and swapped signatures.

## Task Plan

- [ ] **Task 1: Rust parser red tests**
  - Add unit tests in `programs/lime-vault/src/lib.rs` under `signed_order_tests`.
  - Cover valid Ed25519 instruction data plus forged or malformed data.

- [ ] **Task 2: Defensive Ed25519 parser**
  - Parse signature offset/index, pubkey offset/index, and message offset/size/index.
  - Require indexes to be `u16::MAX`, ranges in bounds, signature length 64, pubkey length 32, and message length `SIGNED_ORDER_MESSAGE_LEN`.

- [ ] **Task 3: Integration helper options**
  - Extend `tests/collateral.integration.spec.ts` helper `settleSignedTrade` with explicit adversarial signature options.

- [ ] **Task 4: Integration adversarial tests**
  - Add missing seller instruction, wrong signer pubkey, wrong message, and swapped signature-message cases.

- [ ] **Task 5: Verification and cleanup**
  - Run `cargo test -p lime-vault`, `npm run build:sdk`, `anchor build`, and `npm run test:anchor`.
  - Confirm no new SBF stack offset warning appears for `SettleTrade`.

## Public Interface Impact

- No intended IDL argument changes.
- No Signed Order byte layout changes.
- No SDK public shape changes.
- Only Vault's internal Ed25519 introspection helper and adversarial tests change.

## Assumptions

- Native Ed25519 program verification is still responsible for cryptographic signature validity.
- Vault only accepts Ed25519 instruction data whose pubkey, signature, and message live in the same Ed25519 instruction (`u16::MAX` indexes).
- Issue #7 should not broaden into Trade Execution semantics; order-pair validation, Fill accounting, and Collateral arithmetic remain covered by issue #6.
