import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import type { LimeClient } from "./client.js";
import { marketPda, positionPda, vaultAuthorityPda, vaultPda } from "./pda.js";
import type { OnchainCollateral, PositionSide } from "./types.js";

const SCALE = 1_000_000;

export class SolanaCollateral implements OnchainCollateral {
  constructor(private readonly client: LimeClient) {}

  async initMarketVault(
    marketId: string,
    vaultTokenAccount: string,
  ): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketIdBigInt);
    const [settlementAuthority] = vaultAuthorityPda(
      this.client.addresses.settlementProgramId,
      marketIdBigInt,
    );

    return this.client.vaultProgram.methods
      .initMarketVault(
        new BN(marketId),
        settlementAuthority,
      )
      .accounts({
        payer: this.client.provider.wallet.publicKey,
        usdcMint: this.client.addresses.usdcMint,
        market,
        marketVault,
        vaultTokenAccount: new PublicKey(vaultTokenAccount),
      })
      .rpc();
  }

  async lockCollateral(
    marketId: string,
    amount: number,
    side: PositionSide = "long",
  ): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketIdBigInt);
    const [userPosition] = positionPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
    );
    const userAta = getAssociatedTokenAddressSync(
      this.client.addresses.usdcMint,
      this.client.provider.wallet.publicKey,
    );

    const amountUnits = BigInt(Math.round(amount * SCALE));
    const vaultAccount = await (this.client.vaultProgram as any).account.marketVault.fetch(
      marketVault,
    );

    return this.client.vaultProgram.methods
      .depositCollateral(
        new BN(marketId),
        side === "short" ? { short: {} } : { long: {} },
        new BN(amountUnits.toString()),
      )
      .accounts({
        user: this.client.provider.wallet.publicKey,
        usdcMint: this.client.addresses.usdcMint,
        market,
        userAta,
        marketVault,
        vaultTokenAccount: vaultAccount.vaultTokenAccount,
        userPosition,
      })
      .rpc();
  }

  async releaseCollateral(marketId: string, amount: number): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketIdBigInt);
    const [userPosition] = positionPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
    );
    const amountUnits = BigInt(Math.round(amount * SCALE));
    return this.client.vaultProgram.methods
      .withdrawCollateral(new BN(amountUnits.toString()))
      .accounts({
        user: this.client.provider.wallet.publicKey,
        market,
        marketVault,
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
