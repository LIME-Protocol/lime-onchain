import { AnchorProvider, BN, Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import type { AnchorWalletLike, ProgramAddresses, SolanaConfig } from "./types.js";
import marketIdl from "../../target/idl/lime_market.json";
import vaultIdl from "../../target/idl/lime_vault.json";
import settlementIdl from "../../target/idl/lime_settlement.json";

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
      { ...(marketIdl as any), address: this.addresses.marketProgramId.toBase58() },
      this.provider,
    );
    this.vaultProgram = new Program(
      { ...(vaultIdl as any), address: this.addresses.vaultProgramId.toBase58() },
      this.provider,
    );
    this.settlementProgram = new Program(
      { ...(settlementIdl as any), address: this.addresses.settlementProgramId.toBase58() },
      this.provider,
    );
  }

  toBn(value: bigint | number | string): BN {
    return new BN(value.toString());
  }
}
