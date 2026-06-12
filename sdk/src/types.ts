import type { PublicKey } from "@solana/web3.js";

export type WalletChain = "solana-mainnet" | "solana-devnet" | "solana-localnet";
export type SignedOrderSide = "buy" | "sell";
export type SignedOrderNetwork = "mainnet-beta" | "devnet" | "localnet";

export interface WalletProvider {
  connect(): Promise<string>;
  disconnect(): Promise<void>;
  getAddress(): string | null;
  isConnected(): boolean;
  signMessage(message: string): Promise<string>;
  getChain(): WalletChain;
}

export interface OnchainCollateral {
  depositCollateral(marketId: string, amount: number): Promise<string>;
  withdrawAvailableCollateral(marketId: string, amount: number): Promise<string>;
  lockCollateral(marketId: string, amount: number, side?: PositionSide): Promise<string>;
  releaseCollateral(marketId: string, amount: number): Promise<string>;
  getLockedBalance(marketId: string): Promise<number>;
  getTotalLocked(): Promise<number>;
}

export interface OnchainSettlement {
  resolveMarket(marketId: string, observedValue: number): Promise<string>;
  claimPayout(marketId: string, side?: PositionSide): Promise<string>;
  refundIfInvalidated(marketId: string, side?: PositionSide): Promise<string>;
  getPayoutStatus(marketId: string, side?: PositionSide): Promise<"pending" | "claimable" | "claimed">;
}

export type PositionSide = "long" | "short";

export interface TradeExecutionInput {
  marketId: string;
  buyer: string | PublicKey;
  seller: string | PublicKey;
  buyerNonce?: bigint;
  sellerNonce?: bigint;
  quantity: number;
  priceScaled: number;
}

export interface SignedOrderInput {
  version: number;
  network: SignedOrderNetwork;
  marketProgramId: PublicKey;
  vaultProgramId: PublicKey;
  marketId: bigint;
  owner: PublicKey;
  side: SignedOrderSide;
  priceScaled: bigint;
  quantity: bigint;
  expirationTs: bigint;
  nonce: bigint;
}

export interface OnchainTradeExecution {
  settleTrade(input: TradeExecutionInput): Promise<string>;
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
