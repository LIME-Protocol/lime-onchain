import { PublicKey } from "@solana/web3.js";
import type { SignedOrderInput, SignedOrderNetwork, SignedOrderSide } from "./types.js";

export type { SignedOrderInput } from "./types.js";

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
