# Signed Order Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the signed-order foundation that lets LIME Programs validate user-authorized Orders before Trade Execution changes Positions or Collateral.

**Architecture:** The Vault Program owns Signed Order validation, Fill PDA accounting, and Trade Execution because it already owns Collateral and Position updates. The local SDK code in this repo is only a transitional harness; public SDK APIs should be extracted to `LIME-Protocol/lime-sdk`, but this repo keeps test vectors and local helpers to prove byte-for-byte compatibility. Trade Execution remains backwards-compatible only until the signed-order path is implemented and tests prove the replacement.

**Tech Stack:** Anchor `0.30.1`, Rust Programs, TypeScript `ts-mocha` tests, `@coral-xyz/anchor`, `@solana/web3.js`.

---

## File Structure

- `programs/lime-vault/src/lib.rs`: add Signed Order domain constants, `OrderSide`, `Network`, `SignedOrder`, `FillState`, serialization helpers, Fill PDA accounts, and later signed Trade Execution.
- `tests/signed-order.spec.ts`: add byte-level test vectors for canonical Signed Order serialization and Fill PDA derivation.
- `tests/lime.spec.ts`: update baseline IDL assertions once the Vault IDL exposes the new accounts/types.
- `sdk/src/signed-order.ts`: transitional local helper mirroring the canonical serializer until `lime-sdk` owns it.
- `sdk/src/index.ts`: export the transitional signed-order helper.
- `sdk/src/types.ts`: add transitional TypeScript types for Signed Order inputs.
- `docs/adr/*.md`: already contains the decisions that define the implementation.

---

### Task 1: TypeScript Canonical Serializer and Test Vectors

**Files:**
- Create: `sdk/src/signed-order.ts`
- Modify: `sdk/src/index.ts`
- Modify: `sdk/src/types.ts`
- Create: `tests/signed-order.spec.ts`

- [ ] **Step 1: Write the failing serializer tests**

Create `tests/signed-order.spec.ts`:

```ts
import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";
import {
  LIME_SIGNED_ORDER_DOMAIN,
  encodeSignedOrder,
  fillPda,
  type SignedOrderInput,
} from "../sdk/src/signed-order";

describe("signed order serialization", () => {
  const marketProgramId = new PublicKey("G2YAvLwHFmd4wgs45QScmBYpFthkEjhU34VKQ3HKMagk");
  const vaultProgramId = new PublicKey("BY7MggeDqzyGgJnCQ34pF5pJA6kGUtNvFhaW4VHbFnLm");
  const owner = new PublicKey("11111111111111111111111111111112");

  const order: SignedOrderInput = {
    version: 1,
    network: "localnet",
    marketProgramId,
    vaultProgramId,
    marketId: 42n,
    owner,
    side: "buy",
    priceScaled: 620_000n,
    quantity: 5_000_000n,
    expirationTs: 1_800_000_000n,
    nonce: 0x0102030405060708090a0b0c0d0e0f10n,
  };

  it("encodes a fixed-layout signed order", () => {
    const encoded = encodeSignedOrder(order);

    expect(encoded.subarray(0, LIME_SIGNED_ORDER_DOMAIN.length).toString("utf8")).to.equal(
      LIME_SIGNED_ORDER_DOMAIN,
    );
    expect(encoded.length).to.equal(1 + 17 + 1 + 1 + 32 + 32 + 8 + 32 + 1 + 8 + 8 + 8 + 16);
    expect(encoded[18]).to.equal(1);
    expect(encoded[19]).to.equal(2);
    expect(encoded.readBigUInt64LE(84)).to.equal(42n);
    expect(encoded[124]).to.equal(0);
    expect(encoded.readBigUInt64LE(125)).to.equal(620_000n);
    expect(encoded.readBigUInt64LE(133)).to.equal(5_000_000n);
    expect(encoded.readBigInt64LE(141)).to.equal(1_800_000_000n);
    expect(encoded.readBigUInt64LE(149)).to.equal(0x090a0b0c0d0e0f10n);
    expect(encoded.readBigUInt64LE(157)).to.equal(0x0102030405060708n);
  });

  it("derives fill PDAs from market, owner, and u128 nonce", () => {
    const [pda] = fillPda(vaultProgramId, order.marketId, owner, order.nonce);

    expect(pda).to.be.instanceOf(PublicKey);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
npm run test:anchor -- tests/signed-order.spec.ts
```

Expected: FAIL because `sdk/src/signed-order.ts` does not exist.

- [ ] **Step 3: Add transitional TypeScript types**

Modify `sdk/src/types.ts`:

```ts
export type SignedOrderSide = "buy" | "sell";
export type SignedOrderNetwork = "mainnet-beta" | "devnet" | "localnet";

export interface SignedOrderInput {
  version: number;
  network: SignedOrderNetwork;
  marketProgramId: PublicKey;
  vaultProgramId: PublicKey;
  marketId: bigint;
  owner: PublicKey;
  side: SignedOrderSide;
  priceScaled: bigint;
  quantity: bigint;
  expirationTs: bigint;
  nonce: bigint;
}
```

- [ ] **Step 4: Implement the serializer and Fill PDA helper**

Create `sdk/src/signed-order.ts`:

```ts
import { PublicKey } from "@solana/web3.js";
import type { SignedOrderInput, SignedOrderNetwork, SignedOrderSide } from "./types";

export const LIME_SIGNED_ORDER_DOMAIN = "LIME_SIGNED_ORDER";
const DOMAIN_BYTES = Buffer.from(LIME_SIGNED_ORDER_DOMAIN, "utf8");
const DOMAIN_LEN = 17;

function networkByte(network: SignedOrderNetwork): number {
  if (network === "mainnet-beta") return 0;
  if (network === "devnet") return 1;
  if (network === "localnet") return 2;
  throw new Error(`Unsupported network: ${network satisfies never}`);
}

function sideByte(side: SignedOrderSide): number {
  if (side === "buy") return 0;
  if (side === "sell") return 1;
  throw new Error(`Unsupported side: ${side satisfies never}`);
}

function writeU128Le(buffer: Buffer, value: bigint, offset: number): void {
  if (value < 0n || value > (1n << 128n) - 1n) {
    throw new Error("u128 out of range");
  }
  buffer.writeBigUInt64LE(value & ((1n << 64n) - 1n), offset);
  buffer.writeBigUInt64LE(value >> 64n, offset + 8);
}

function u64Le(value: bigint): Buffer {
  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64LE(value);
  return buffer;
}

function u128Le(value: bigint): Buffer {
  const buffer = Buffer.alloc(16);
  writeU128Le(buffer, value, 0);
  return buffer;
}

export function encodeSignedOrder(order: SignedOrderInput): Buffer {
  if (DOMAIN_BYTES.length !== DOMAIN_LEN) {
    throw new Error("Invalid signed order domain length");
  }

  const buffer = Buffer.alloc(1 + DOMAIN_LEN + 1 + 1 + 32 + 32 + 8 + 32 + 1 + 8 + 8 + 8 + 16);
  let offset = 0;
  buffer.writeUInt8(DOMAIN_LEN, offset);
  offset += 1;
  DOMAIN_BYTES.copy(buffer, offset);
  offset += DOMAIN_LEN;
  buffer.writeUInt8(order.version, offset);
  offset += 1;
  buffer.writeUInt8(networkByte(order.network), offset);
  offset += 1;
  order.marketProgramId.toBuffer().copy(buffer, offset);
  offset += 32;
  order.vaultProgramId.toBuffer().copy(buffer, offset);
  offset += 32;
  buffer.writeBigUInt64LE(order.marketId, offset);
  offset += 8;
  order.owner.toBuffer().copy(buffer, offset);
  offset += 32;
  buffer.writeUInt8(sideByte(order.side), offset);
  offset += 1;
  buffer.writeBigUInt64LE(order.priceScaled, offset);
  offset += 8;
  buffer.writeBigUInt64LE(order.quantity, offset);
  offset += 8;
  buffer.writeBigInt64LE(order.expirationTs, offset);
  offset += 8;
  writeU128Le(buffer, order.nonce, offset);
  return buffer;
}

export function fillPda(
  vaultProgramId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
  nonce: bigint,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fill"), u64Le(marketId), owner.toBuffer(), u128Le(nonce)],
    vaultProgramId,
  );
}
```

- [ ] **Step 5: Export the helper**

Modify `sdk/src/index.ts`:

```ts
export * from "./client.js";
export * from "./market.js";
export * from "./pda.js";
export * from "./settlement.js";
export * from "./signed-order.js";
export * from "./types.js";
export * from "./vault.js";
export * from "./wallet.js";
```

- [ ] **Step 6: Run tests to verify they pass**

Run:

```bash
npm run test:anchor -- tests/signed-order.spec.ts
```

Expected: PASS.

---

### Task 2: Rust Signed Order Types and Serialization Parity

**Files:**
- Modify: `programs/lime-vault/src/lib.rs`
- Modify: `tests/lime.spec.ts`

- [ ] **Step 1: Add failing IDL assertions for Signed Order types**

Modify `tests/lime.spec.ts` in the IDL test:

```ts
const vaultTypeNames = new Set(vaultIdl.types.map((type: any) => type.name));
expect(vaultTypeNames.has("SignedOrder")).to.equal(true);
expect(vaultTypeNames.has("OrderSide")).to.equal(true);
expect(vaultTypeNames.has("OrderNetwork")).to.equal(true);
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
anchor build && npm run test:anchor -- tests/lime.spec.ts
```

Expected: FAIL because the IDL does not expose the Signed Order types yet.

- [ ] **Step 3: Add Rust types**

Modify `programs/lime-vault/src/lib.rs` after `PositionSide`:

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum OrderNetwork {
    MainnetBeta,
    Devnet,
    Localnet,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct SignedOrder {
    pub version: u8,
    pub network: OrderNetwork,
    pub market_program_id: Pubkey,
    pub vault_program_id: Pubkey,
    pub market_id: u64,
    pub owner: Pubkey,
    pub side: OrderSide,
    pub price_scaled: u64,
    pub quantity: u64,
    pub expiration_ts: i64,
    pub nonce: u128,
}
```

- [ ] **Step 4: Run build and tests**

Run:

```bash
anchor build && npm run test:anchor -- tests/lime.spec.ts
```

Expected: PASS with new IDL types available.

---

### Task 3: Fill PDA Account

**Files:**
- Modify: `programs/lime-vault/src/lib.rs`
- Modify: `sdk/src/pda.ts`
- Modify: `tests/lime.spec.ts`

- [ ] **Step 1: Add failing Fill PDA derivation test**

Add to `tests/lime.spec.ts` helper section:

```ts
function nonceBuffer(nonce: bigint): Buffer {
  const buffer = Buffer.alloc(16);
  buffer.writeBigUInt64LE(nonce & ((1n << 64n) - 1n), 0);
  buffer.writeBigUInt64LE(nonce >> 64n, 8);
  return buffer;
}

function fillPda(programId: PublicKey, marketId: bigint, owner: PublicKey, nonce: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fill"), marketIdBuffer(marketId), owner.toBuffer(), nonceBuffer(nonce)],
    programId,
  );
}
```

Then assert the PDA in the deterministic PDA test:

```ts
const [fill] = fillPda(vaultProgramId, marketId, provider.wallet.publicKey, 1n);
expect(fill).to.be.instanceOf(PublicKey);
```

- [ ] **Step 2: Add FillState to the Program**

Modify `programs/lime-vault/src/lib.rs` near accounts:

```rust
#[account]
#[derive(InitSpace)]
pub struct FillState {
    pub market_id: u64,
    pub owner: Pubkey,
    pub nonce: u128,
    pub quantity: u64,
    pub filled_quantity: u64,
    pub bump: u8,
}
```

- [ ] **Step 3: Add SDK PDA helper**

Modify `sdk/src/pda.ts`:

```ts
function nonceBuffer(nonce: bigint): Buffer {
  const buffer = Buffer.alloc(16);
  buffer.writeBigUInt64LE(nonce & ((1n << 64n) - 1n), 0);
  buffer.writeBigUInt64LE(nonce >> 64n, 8);
  return buffer;
}

export function fillPda(
  programId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
  nonce: bigint,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fill"), marketIdBuffer(marketId), owner.toBuffer(), nonceBuffer(nonce)],
    programId,
  );
}
```

- [ ] **Step 4: Run build and tests**

Run:

```bash
anchor build && npm run test:anchor -- tests/lime.spec.ts tests/signed-order.spec.ts
```

Expected: PASS.

---

### Task 4: Replace Trusted Trade Execution Skeleton

**Files:**
- Modify: `programs/lime-vault/src/lib.rs`
- Modify: `sdk/src/types.ts`
- Modify: `sdk/src/vault.ts`
- Modify: `tests/lime.spec.ts`

- [ ] **Step 1: Add failing IDL assertion for permissionless signed execution**

Modify `tests/lime.spec.ts`:

```ts
const settleTrade = vaultIdl.instructions.find((i: any) => i.name === "settle_trade");
const settleAccounts = new Map(settleTrade.accounts.map((account: any) => [account.name, account]));
expect(settleAccounts.has("backend_signer")).to.equal(false);
expect(settleAccounts.has("submitter")).to.equal(true);
expect(settleAccounts.has("buyer_fill")).to.equal(true);
expect(settleAccounts.has("seller_fill")).to.equal(true);
```

- [ ] **Step 2: Run build and tests to verify failure**

Run:

```bash
anchor build && npm run test:anchor -- tests/lime.spec.ts
```

Expected: FAIL because `settle_trade` still has `backend_signer`.

- [ ] **Step 3: Rename the payer account and add FillState accounts**

Modify `SettleTrade` in `programs/lime-vault/src/lib.rs` so `backend_signer` becomes `submitter`, and add `buyer_fill` / `seller_fill` as `init_if_needed` accounts using the Fill PDA seeds. Keep the old arguments temporarily until full signature verification lands:

```rust
#[account(mut)]
pub submitter: Signer<'info>,
```

Use `payer = submitter` for Position and Fill PDA initialization.

- [ ] **Step 4: Remove backend authorization check**

Remove:

```rust
require!(
    ctx.accounts.backend_signer.key() == market.admin,
    VaultError::UnauthorizedBackend
);
```

- [ ] **Step 5: Run build and tests**

Run:

```bash
anchor build && npm run test:anchor -- tests/lime.spec.ts
```

Expected: PASS after updating local SDK account names.

---

## Self-Review

- Spec coverage: ADRs 0003-0013 and issue #4 are partially covered by Tasks 1-4. This plan intentionally builds the signed-order foundation and permissionless execution skeleton only; full Ed25519 introspection and adversarial Trade Execution tests should get a second plan after this lands.
- Placeholder scan: no placeholder code remains in the executable tasks.
- Type consistency: `nonce` is `bigint` in TypeScript and `u128` in Rust; `price_scaled` and `quantity` are `u64`/`bigint`; network is encoded as `mainnet-beta = 0`, `devnet = 1`, `localnet = 2`; side is `buy = 0`, `sell = 1`.
