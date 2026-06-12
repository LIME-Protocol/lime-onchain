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

    expect(encoded.subarray(1, 1 + LIME_SIGNED_ORDER_DOMAIN.length).toString("utf8")).to.equal(
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
