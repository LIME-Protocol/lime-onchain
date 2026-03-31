import { PublicKey } from "@solana/web3.js";
import type { AnchorWalletLike, SolanaConfig, WalletChain, WalletProvider } from "./types.js";

export class SolanaWalletProvider implements WalletProvider {
  private connectedAddress: string | null = null;

  constructor(
    private readonly wallet: AnchorWalletLike,
    private readonly config: SolanaConfig,
  ) {}

  async connect(): Promise<string> {
    this.connectedAddress = this.wallet.publicKey.toBase58();
    return this.connectedAddress;
  }

  async disconnect(): Promise<void> {
    this.connectedAddress = null;
  }

  getAddress(): string | null {
    return this.connectedAddress;
  }

  isConnected(): boolean {
    return this.connectedAddress !== null;
  }

  async signMessage(message: string): Promise<string> {
    if (!this.wallet.signMessage) {
      throw new Error("Wallet does not support signMessage");
    }
    const signed = await this.wallet.signMessage(Buffer.from(message));
    return Buffer.from(signed).toString("base64");
  }

  getChain(): WalletChain {
    if (this.config.network === "mainnet-beta") return "solana-mainnet";
    if (this.config.network === "devnet") return "solana-devnet";
    return "solana-localnet";
  }

  getPublicKey(): PublicKey {
    return this.wallet.publicKey;
  }
}
