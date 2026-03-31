import { PublicKey } from "@solana/web3.js";

export function marketPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  const marketIdBuffer = Buffer.alloc(8);
  marketIdBuffer.writeBigUInt64LE(marketId);
  return PublicKey.findProgramAddressSync([Buffer.from("market"), marketIdBuffer], programId);
}

export function protocolPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("protocol")], programId);
}

export function vaultPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  const marketIdBuffer = Buffer.alloc(8);
  marketIdBuffer.writeBigUInt64LE(marketId);
  return PublicKey.findProgramAddressSync([Buffer.from("vault"), marketIdBuffer], programId);
}

export function positionPda(
  programId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
): [PublicKey, number] {
  const marketIdBuffer = Buffer.alloc(8);
  marketIdBuffer.writeBigUInt64LE(marketId);
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position"), marketIdBuffer, owner.toBuffer()],
    programId,
  );
}

export function resolutionPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  const marketIdBuffer = Buffer.alloc(8);
  marketIdBuffer.writeBigUInt64LE(marketId);
  return PublicKey.findProgramAddressSync([Buffer.from("resolution"), marketIdBuffer], programId);
}

export function claimPda(
  programId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
): [PublicKey, number] {
  const marketIdBuffer = Buffer.alloc(8);
  marketIdBuffer.writeBigUInt64LE(marketId);
  return PublicKey.findProgramAddressSync([Buffer.from("claim"), marketIdBuffer, owner.toBuffer()], programId);
}
