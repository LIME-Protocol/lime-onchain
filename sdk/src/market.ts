import { BN } from "@coral-xyz/anchor";
import type { LimeClient } from "./client.js";
import { marketPda, protocolPda } from "./pda.js";
import type { MarketInput } from "./types.js";

export class SolanaMarketService {
  constructor(private readonly client: LimeClient) {}

  async createMarket(input: MarketInput): Promise<string> {
    const [protocolConfig] = protocolPda(this.client.addresses.marketProgramId);
    const [market] = marketPda(this.client.addresses.marketProgramId, input.marketId);

    return this.client.marketProgram.methods
      .createMarket(
        new BN(input.marketId.toString()),
        new BN(input.lowerBound.toString()),
        new BN(input.upperBound.toString()),
        new BN(input.resolutionTs.toString()),
        input.settlementSource,
        { linear: {} },
        input.minParticipants,
      )
      .accounts({
        admin: this.client.provider.wallet.publicKey,
        protocolConfig,
        market,
      })
      .rpc();
  }

  async activateMarket(marketId: bigint): Promise<string> {
    const [market] = marketPda(this.client.addresses.marketProgramId, marketId);
    return this.client.marketProgram.methods
      .activateMarket()
      .accounts({ market })
      .rpc();
  }

  async closeMarket(marketId: bigint): Promise<string> {
    const [protocolConfig] = protocolPda(this.client.addresses.marketProgramId);
    const [market] = marketPda(this.client.addresses.marketProgramId, marketId);
    return this.client.marketProgram.methods
      .closeMarket()
      .accounts({
        admin: this.client.provider.wallet.publicKey,
        protocolConfig,
        market,
      })
      .rpc();
  }
}
