import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
import type { LimeClient } from "./client.js";
import { claimPda, resolutionPda } from "./pda.js";
import type { OnchainSettlement } from "./types.js";

export class SolanaSettlement implements OnchainSettlement {
  constructor(private readonly client: LimeClient) {}

  async resolveMarket(marketId: string, observedValue: number): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [resolution] = resolutionPda(this.client.addresses.settlementProgramId, marketIdBigInt);

    return this.client.settlementProgram.methods
      .submitResolution(
        new BN(marketId),
        new BN(Math.floor(observedValue)),
        new BN(0),
        new BN(1_000_000),
      )
      .accounts({
        resolver: this.client.provider.wallet.publicKey,
        resolution,
      })
      .rpc();
  }

  async claimPayout(marketId: string): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [claimReceipt] = claimPda(
      this.client.addresses.settlementProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
    );
    const userAta = getAssociatedTokenAddressSync(
      this.client.addresses.usdcMint,
      this.client.provider.wallet.publicKey,
    );

    return this.client.settlementProgram.methods
      .claimPayout(new BN(marketId))
      .accounts({
        user: this.client.provider.wallet.publicKey,
        usdcMint: this.client.addresses.usdcMint,
        userAta,
        claimReceipt,
      })
      .rpc();
  }

  async getPayoutStatus(marketId: string): Promise<"pending" | "claimable" | "claimed"> {
    const marketIdBigInt = BigInt(marketId);
    const [resolution] = resolutionPda(this.client.addresses.settlementProgramId, marketIdBigInt);
    const [claimReceipt] = claimPda(
      this.client.addresses.settlementProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
    );

    const resolutionAccount = await (this.client.settlementProgram as any).account.resolution.fetchNullable(
      resolution,
    );
    if (!resolutionAccount) return "pending";
    const claimAccount = await (this.client.settlementProgram as any).account.claimReceipt.fetchNullable(
      claimReceipt,
    );
    if (!claimAccount) return "claimable";
    return claimAccount.claimed ? "claimed" : "claimable";
  }
}
