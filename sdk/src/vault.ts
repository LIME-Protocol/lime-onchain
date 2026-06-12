import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import type { LimeClient } from "./client.js";
import {
  collateralPda,
  fillPda,
  marketPda,
  positionPda,
  vaultAuthorityPda,
  vaultPda,
  vaultTokenAuthorityPda,
} from "./pda.js";
import type {
  OnchainCollateral,
  OnchainTradeExecution,
  PositionSide,
  TradeExecutionInput,
} from "./types.js";

const SCALE = 1_000_000;

export class SolanaCollateral implements OnchainCollateral, OnchainTradeExecution {
  constructor(private readonly client: LimeClient) {}

  async initMarketVault(
    marketId: string,
    vaultTokenAccount: string,
  ): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketIdBigInt);
    const [vaultAuthority] = vaultTokenAuthorityPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
    );
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
        vaultAuthority,
        marketVault,
        vaultTokenAccount: new PublicKey(vaultTokenAccount),
      })
      .rpc();
  }

  async lockCollateral(
    marketId: string,
    amount: number,
    _side: PositionSide = "long",
  ): Promise<string> {
    return this.depositCollateral(marketId, amount);
  }

  async depositCollateral(
    marketId: string,
    amount: number,
  ): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketIdBigInt);
    const [userCollateral] = collateralPda(
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
        new BN(amountUnits.toString()),
      )
      .accounts({
        user: this.client.provider.wallet.publicKey,
        usdcMint: this.client.addresses.usdcMint,
        market,
        userAta,
        marketVault,
        vaultTokenAccount: vaultAccount.vaultTokenAccount,
        userCollateral,
      })
      .rpc();
  }

  async releaseCollateral(marketId: string, amount: number): Promise<string> {
    return this.withdrawAvailableCollateral(marketId, amount);
  }

  async withdrawAvailableCollateral(marketId: string, amount: number): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketIdBigInt);
    const [vaultAuthority] = vaultTokenAuthorityPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
    );
    const [userCollateral] = collateralPda(
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
      .withdrawAvailableCollateral(
        new BN(marketId),
        new BN(amountUnits.toString()),
      )
      .accounts({
        user: this.client.provider.wallet.publicKey,
        usdcMint: this.client.addresses.usdcMint,
        market,
        marketVault,
        vaultAuthority,
        vaultTokenAccount: vaultAccount.vaultTokenAccount,
        userAta,
        userCollateral,
      })
      .rpc();
  }

  async settleTrade(input: TradeExecutionInput): Promise<string> {
    const marketIdBigInt = BigInt(input.marketId);
    const buyer = new PublicKey(input.buyer);
    const seller = new PublicKey(input.seller);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketIdBigInt);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [buyerCollateral] = collateralPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      buyer,
    );
    const [sellerCollateral] = collateralPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      seller,
    );
    const buyerNonce = input.buyerNonce ?? 0n;
    const sellerNonce = input.sellerNonce ?? 0n;
    const [buyerFill] = fillPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      buyer,
      buyerNonce,
    );
    const [sellerFill] = fillPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      seller,
      sellerNonce,
    );
    const [buyerPosition] = positionPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      buyer,
      "long",
    );
    const [sellerPosition] = positionPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      seller,
      "short",
    );
    const quantityUnits = BigInt(Math.round(input.quantity * SCALE));

    return this.client.vaultProgram.methods
      .settleTrade(
        new BN(input.marketId),
        buyer,
        seller,
        new BN(buyerNonce.toString()),
        new BN(sellerNonce.toString()),
        new BN(quantityUnits.toString()),
        new BN(Math.floor(input.priceScaled).toString()),
      )
      .accounts({
        submitter: this.client.provider.wallet.publicKey,
        market,
        marketVault,
        buyerCollateral,
        sellerCollateral,
        buyerFill,
        sellerFill,
        buyerPosition,
        sellerPosition,
      })
      .rpc();
  }

  async getLockedBalance(marketId: string): Promise<number> {
    const [userCollateral] = collateralPda(
      this.client.addresses.vaultProgramId,
      BigInt(marketId),
      this.client.provider.wallet.publicKey,
    );
    const account = await (this.client.vaultProgram as any).account.userCollateral.fetchNullable(
      userCollateral,
    );
    if (!account) return 0;
    return Number(account.totalDeposited) / SCALE;
  }

  async getTotalLocked(): Promise<number> {
    const accounts = await (this.client.vaultProgram as any).account.userCollateral.all([
      {
        memcmp: {
          offset: 16,
          bytes: this.client.provider.wallet.publicKey.toBase58(),
        },
      },
    ]);
    return (
      accounts.reduce(
        (acc: number, row: { account: { totalDeposited: number | bigint } }) =>
          acc + Number(row.account.totalDeposited),
        0,
      ) / SCALE
    );
  }
}
