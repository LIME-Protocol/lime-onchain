import { AnchorProvider, BN, Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import type { AnchorWalletLike, ProgramAddresses, SolanaConfig } from "./types.js";

const emptyIdl = {
  version: "0.1.0",
  name: "lime_placeholder",
  instructions: [],
};

export class LimeClient {
  readonly provider: AnchorProvider;
  readonly addresses: ProgramAddresses;
  readonly marketProgram: Program;
  readonly vaultProgram: Program;
  readonly settlementProgram: Program;

  constructor(
    connection: AnchorProvider["connection"],
    wallet: AnchorWalletLike,
    config: SolanaConfig,
  ) {
    this.provider = new AnchorProvider(connection, wallet as any, {
      commitment: "confirmed",
    });
    this.addresses = {
      marketProgramId: new PublicKey(config.marketProgramId),
      vaultProgramId: new PublicKey(config.vaultProgramId),
      settlementProgramId: new PublicKey(config.settlementProgramId),
      usdcMint: new PublicKey(config.usdcMint),
    };

    this.marketProgram = new Program(
      { ...emptyIdl, address: this.addresses.marketProgramId.toBase58() } as any,
      this.provider,
    );
    this.vaultProgram = new Program(
      { ...emptyIdl, address: this.addresses.vaultProgramId.toBase58() } as any,
      this.provider,
    );
    this.settlementProgram = new Program(
      { ...emptyIdl, address: this.addresses.settlementProgramId.toBase58() } as any,
      this.provider,
    );
  }

  toBn(value: bigint | number | string): BN {
    return new BN(value.toString());
  }
}
