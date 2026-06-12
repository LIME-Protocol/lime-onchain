# Security-First Development

LIME's product value is moving to the on-chain Programs, SDK, and Oracle integration. Treat Program correctness, adversarial behavior, and audit readiness as first-class development concerns.

## Before changing code

- Read `CONTEXT.md` and the ADRs that touch the area you are changing.
- Check the open GitHub issue for the current security-critical acceptance criteria.
- If work touches Trade Execution, Signed Orders, Ed25519 introspection, Fill PDA accounting, replay protection, Collateral arithmetic, custody, Resolution, Settlement, Claims, or Refunds, assume adversarial review is required.

## During development

- Prefer small commits or small reviewable changes with clear security boundaries.
- Keep authorization, replay protection, and Collateral invariants explicit in code and tests.
- Add adversarial tests for invalid signatures, wrong signer, wrong message, expired Orders, replay, overfill, price out of range, insufficient Collateral, and unauthorized custody or settlement movement when relevant.
- Use security-focused AI/code review during development to catch issues early, especially around Solana account constraints, PDA derivation, Ed25519 instruction introspection, arithmetic, and CPI boundaries.
- Treat the Matching Engine and reference Order Book as untrusted coordinators unless an ADR explicitly says otherwise.

## Before mainnet or commercial launch

- Freeze the Signed Order spec, Program behavior, and adversarial test suite before external audit.
- Prepare reproducible build and test instructions for all public Programs.
- Do not treat the product as commercially ready until the relevant on-chain Programs have passed external audit or an explicit human decision records a different launch risk.
