import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
import type { LimeClient } from "./client.js";
import { positionPda, vaultPda } from "./pda.js";
import type { OnchainCollateral } from "./types.js";

const SCALE = 1_000_000;

export class SolanaCollateral implements OnchainCollateral {
  constructor(private readonly client: LimeClient) {}

  async lockCollateral(marketId: string, amount: number): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [userPosition] = positionPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
    );
    const userAta = getAssociatedTokenAddressSync(
      this.client.addresses.usdcMint,
      this.client.provider.wallet.publicKey,
    );

    const quantity = BigInt(Math.floor(amount * SCALE));
    return this.client.vaultProgram.methods
      .depositCollateral(
        new BN(marketId),
        { long: {} },
        new BN(quantity.toString()),
      )
      .accounts({
        user: this.client.provider.wallet.publicKey,
        usdcMint: this.client.addresses.usdcMint,
        userAta,
        marketVault,
        vaultTokenAccount: marketVault,
        userPosition,
      })
      .rpc();
  }

  async releaseCollateral(marketId: string): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [userPosition] = positionPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
    );
    const userAta = getAssociatedTokenAddressSync(
      this.client.addresses.usdcMint,
      this.client.provider.wallet.publicKey,
    );
    return this.client.vaultProgram.methods
      .withdrawCollateral(new BN(0))
      .accounts({
        user: this.client.provider.wallet.publicKey,
        usdcMint: this.client.addresses.usdcMint,
        userAta,
        marketVault,
        vaultTokenAccount: marketVault,
        userPosition,
      })
      .rpc();
  }

  async getLockedBalance(marketId: string): Promise<number> {
    const [userPosition] = positionPda(
      this.client.addresses.vaultProgramId,
      BigInt(marketId),
      this.client.provider.wallet.publicKey,
    );
    const account = await (this.client.vaultProgram as any).account.userPosition.fetchNullable(
      userPosition,
    );
    if (!account) return 0;
    return Number(account.collateralLocked) / SCALE;
  }

  async getTotalLocked(): Promise<number> {
    const accounts = await (this.client.vaultProgram as any).account.userPosition.all([
      {
        memcmp: {
          offset: 16,
          bytes: this.client.provider.wallet.publicKey.toBase58(),
        },
      },
    ]);
    return (
      accounts.reduce(
        (acc: number, row: { account: { collateralLocked: number | bigint } }) =>
          acc + Number(row.account.collateralLocked),
        0,
      ) / SCALE
    );
  }
}
