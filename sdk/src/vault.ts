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
  SignedOrderInput,
  TradeExecutionInput,
} from "./types.js";

const SCALE = 1_000_000;
const SYSVAR_INSTRUCTIONS_PUBKEY = new PublicKey("Sysvar1nstructions1111111111111111111111111");

function anchorNetwork(network: SignedOrderInput["network"]): Record<string, Record<string, never>> {
  if (network === "mainnet-beta") return { mainnetBeta: {} };
  if (network === "devnet") return { devnet: {} };
  if (network === "localnet") return { localnet: {} };
  throw new Error(`Unsupported signed order network: ${network satisfies never}`);
}

function anchorSide(side: SignedOrderInput["side"]): Record<string, Record<string, never>> {
  if (side === "buy") return { buy: {} };
  if (side === "sell") return { sell: {} };
  throw new Error(`Unsupported signed order side: ${side satisfies never}`);
}

function anchorSignedOrder(order: SignedOrderInput): Record<string, unknown> {
  return {
    version: order.version,
    network: anchorNetwork(order.network),
    marketProgramId: order.marketProgramId,
    vaultProgramId: order.vaultProgramId,
    marketId: new BN(order.marketId.toString()),
    owner: order.owner,
    side: anchorSide(order.side),
    priceScaled: new BN(order.priceScaled.toString()),
    quantity: new BN(order.quantity.toString()),
    expirationTs: new BN(order.expirationTs.toString()),
    nonce: new BN(order.nonce.toString()),
  };
}

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
    const marketIdBigInt = input.buyerOrder.marketId;
    if (input.sellerOrder.marketId !== marketIdBigInt) {
      throw new Error("Signed order market mismatch");
    }
    const buyer = input.buyerOrder.owner;
    const seller = input.sellerOrder.owner;
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
    const [buyerFill] = fillPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      buyer,
      input.buyerOrder.nonce,
    );
    const [sellerFill] = fillPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      seller,
      input.sellerOrder.nonce,
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

    return this.client.vaultProgram.methods
      .settleTrade(
        anchorSignedOrder(input.buyerOrder),
        anchorSignedOrder(input.sellerOrder),
        new BN(input.quantity.toString()),
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
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
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
