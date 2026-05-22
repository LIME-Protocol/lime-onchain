import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import type { LimeClient } from "./client.js";
import {
  claimPda,
  marketPda,
  positionPda,
  protocolPda,
  refundPda,
  resolutionPda,
  vaultPda,
  vaultAuthorityPda,
  vaultTokenAuthorityPda,
} from "./pda.js";
import type { OnchainSettlement, PositionSide } from "./types.js";

export class SolanaSettlement implements OnchainSettlement {
  constructor(private readonly client: LimeClient) {}

  async initializeProtocol(resolver: string): Promise<string> {
    const [protocolConfig] = protocolPda(this.client.addresses.settlementProgramId);
    return this.client.settlementProgram.methods
      .initializeProtocol(new PublicKey(resolver))
      .accounts({
        admin: this.client.provider.wallet.publicKey,
        protocolConfig,
      })
      .rpc();
  }

  async initMarketSettlement(marketId: string): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [protocolConfig] = protocolPda(this.client.addresses.settlementProgramId);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketIdBigInt);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [vaultAuthority] = vaultAuthorityPda(this.client.addresses.settlementProgramId, marketIdBigInt);

    return this.client.settlementProgram.methods
      .initMarketSettlement(new BN(marketId))
      .accounts({
        admin: this.client.provider.wallet.publicKey,
        protocolConfig,
        market,
        marketVault,
        vaultAuthority,
      })
      .rpc();
  }

  async resolveMarket(marketId: string, observedValue: number): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [protocolConfig] = protocolPda(this.client.addresses.settlementProgramId);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketIdBigInt);
    const [resolution] = resolutionPda(this.client.addresses.settlementProgramId, marketIdBigInt);
    const [vaultAuthority] = vaultAuthorityPda(this.client.addresses.settlementProgramId, marketIdBigInt);

    return this.client.settlementProgram.methods
      .submitResolution(
        new BN(marketId),
        new BN(Math.floor(observedValue)),
      )
      .accounts({
        resolver: this.client.provider.wallet.publicKey,
        protocolConfig,
        market,
        vaultAuthority,
        resolution,
      })
      .rpc();
  }

  async claimPayout(marketId: string, side: PositionSide = "long"): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [resolution] = resolutionPda(this.client.addresses.settlementProgramId, marketIdBigInt);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [claimReceipt] = claimPda(
      this.client.addresses.settlementProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
      side,
    );
    const [vaultAuthority] = vaultAuthorityPda(this.client.addresses.settlementProgramId, marketIdBigInt);
    const [vaultTokenAuthority] = vaultTokenAuthorityPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [userPosition] = positionPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
      side,
    );
    const authorityAccount = await (this.client.settlementProgram as any).account.vaultAuthority.fetch(
      vaultAuthority,
    );
    const userAta = getAssociatedTokenAddressSync(
      authorityAccount.tokenMint,
      this.client.provider.wallet.publicKey,
    );

    return this.client.settlementProgram.methods
      .claimPayout(new BN(marketId))
      .accounts({
        user: this.client.provider.wallet.publicKey,
        usdcMint: authorityAccount.tokenMint,
        userAta,
        resolution,
        userPosition,
        claimReceipt,
        marketVault,
        vaultAuthority,
        vaultTokenAuthority,
        vaultTokenAccount: authorityAccount.vaultTokenAccount,
        vaultProgram: this.client.addresses.vaultProgramId,
      })
      .rpc();
  }

  async refundIfInvalidated(marketId: string, side: PositionSide = "long"): Promise<string> {
    const marketIdBigInt = BigInt(marketId);
    const [protocolConfig] = protocolPda(this.client.addresses.settlementProgramId);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketIdBigInt);
    const [marketVault] = vaultPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [vaultAuthority] = vaultAuthorityPda(this.client.addresses.settlementProgramId, marketIdBigInt);
    const [vaultTokenAuthority] = vaultTokenAuthorityPda(this.client.addresses.vaultProgramId, marketIdBigInt);
    const [userPosition] = positionPda(
      this.client.addresses.vaultProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
      side,
    );
    const [refundReceipt] = refundPda(
      this.client.addresses.settlementProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
      side,
    );

    const authorityAccount = await (this.client.settlementProgram as any).account.vaultAuthority.fetch(
      vaultAuthority,
    );
    const userAta = getAssociatedTokenAddressSync(
      authorityAccount.tokenMint,
      this.client.provider.wallet.publicKey,
    );

    return this.client.settlementProgram.methods
      .refund(new BN(marketId))
      .accounts({
        admin: this.client.provider.wallet.publicKey,
        protocolConfig,
        market,
        usdcMint: authorityAccount.tokenMint,
        userAta,
        userPosition,
        refundReceipt,
        marketVault,
        vaultAuthority,
        vaultTokenAuthority,
        vaultTokenAccount: authorityAccount.vaultTokenAccount,
        vaultProgram: this.client.addresses.vaultProgramId,
      })
      .rpc();
  }

  async getPayoutStatus(
    marketId: string,
    side: PositionSide = "long",
  ): Promise<"pending" | "claimable" | "claimed"> {
    const marketIdBigInt = BigInt(marketId);
    const [resolution] = resolutionPda(this.client.addresses.settlementProgramId, marketIdBigInt);
    const [claimReceipt] = claimPda(
      this.client.addresses.settlementProgramId,
      marketIdBigInt,
      this.client.provider.wallet.publicKey,
      side,
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
