import type { PublicKey } from "@solana/web3.js";

export type WalletChain = "solana-mainnet" | "solana-devnet" | "solana-localnet";

export interface WalletProvider {
  connect(): Promise<string>;
  disconnect(): Promise<void>;
  getAddress(): string | null;
  isConnected(): boolean;
  signMessage(message: string): Promise<string>;
  getChain(): WalletChain;
}

export interface OnchainCollateral {
  lockCollateral(marketId: string, amount: number): Promise<string>;
  releaseCollateral(marketId: string): Promise<string>;
  getLockedBalance(marketId: string): Promise<number>;
  getTotalLocked(): Promise<number>;
}

export interface OnchainSettlement {
  resolveMarket(marketId: string, observedValue: number): Promise<string>;
  claimPayout(marketId: string): Promise<string>;
  getPayoutStatus(marketId: string): Promise<"pending" | "claimable" | "claimed">;
}

export interface SolanaConfig {
  network: "mainnet-beta" | "devnet" | "localnet";
  marketProgramId: string;
  vaultProgramId: string;
  settlementProgramId: string;
  usdcMint: string;
}

export interface AnchorWalletLike {
  publicKey: PublicKey;
  signTransaction: (tx: any) => Promise<any>;
  signAllTransactions?: (txs: any[]) => Promise<any[]>;
  signMessage?: (message: Uint8Array) => Promise<Uint8Array>;
}

export interface ProgramAddresses {
  marketProgramId: PublicKey;
  vaultProgramId: PublicKey;
  settlementProgramId: PublicKey;
  usdcMint: PublicKey;
}

export interface MarketInput {
  marketId: bigint;
  lowerBound: bigint;
  upperBound: bigint;
  resolutionTs: bigint;
  settlementSource: string;
  minParticipants: number;
}
