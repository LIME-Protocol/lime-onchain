import { PublicKey } from "@solana/web3.js";
import type { PositionSide } from "./types.js";

function marketIdBuffer(marketId: bigint): Buffer {
  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64LE(marketId);
  return buffer;
}

export function marketPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("market"), marketIdBuffer(marketId)], programId);
}

export function protocolPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("protocol")], programId);
}

export function vaultPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("vault"), marketIdBuffer(marketId)], programId);
}

export function vaultTokenAuthorityPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority"), marketIdBuffer(marketId)],
    programId,
  );
}

function positionSideSeed(side: PositionSide): Buffer {
  return Buffer.from(side === "short" ? "short" : "long");
}

export function collateralPda(
  programId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("collateral"), marketIdBuffer(marketId), owner.toBuffer()],
    programId,
  );
}

export function positionPda(
  programId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
  side: PositionSide,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position"), marketIdBuffer(marketId), owner.toBuffer(), positionSideSeed(side)],
    programId,
  );
}

export function resolutionPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("resolution"), marketIdBuffer(marketId)], programId);
}

export function claimPda(
  programId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
  side: PositionSide,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("claim"), marketIdBuffer(marketId), owner.toBuffer(), positionSideSeed(side)],
    programId,
  );
}

export function refundPda(
  programId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
  side: PositionSide,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("refund"), marketIdBuffer(marketId), owner.toBuffer(), positionSideSeed(side)],
    programId,
  );
}

export function vaultAuthorityPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority"), marketIdBuffer(marketId)],
    programId,
  );
}
