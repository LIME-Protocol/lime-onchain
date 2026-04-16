import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import fs from "fs";
import path from "path";
import { claimPda, marketPda, protocolPda, vaultAuthorityPda, vaultPda } from "../sdk/src/pda";

describe("lime-onchain baseline", () => {
  const provider = process.env.ANCHOR_PROVIDER_URL
    ? anchor.AnchorProvider.env()
    : new anchor.AnchorProvider(
        new Connection("https://api.devnet.solana.com", "confirmed"),
        new anchor.Wallet(Keypair.generate()),
        { commitment: "confirmed" },
      );
  anchor.setProvider(provider);

  const rootDir = path.resolve(__dirname, "..");
  const marketIdl = JSON.parse(
    fs.readFileSync(path.join(rootDir, "target/idl/lime_market.json"), "utf8"),
  );
  const vaultIdl = JSON.parse(
    fs.readFileSync(path.join(rootDir, "target/idl/lime_vault.json"), "utf8"),
  );
  const settlementIdl = JSON.parse(
    fs.readFileSync(path.join(rootDir, "target/idl/lime_settlement.json"), "utf8"),
  );

  it("bootstraps provider", async () => {
    expect(provider.wallet.publicKey).to.not.equal(undefined);
  });

  it("loads generated IDLs for all programs", async () => {
    expect(marketIdl.instructions.length).to.be.greaterThan(0);
    expect(vaultIdl.instructions.length).to.be.greaterThan(0);
    expect(settlementIdl.instructions.length).to.be.greaterThan(0);

    const marketInstructionNames = new Set(marketIdl.instructions.map((i: any) => i.name));
    expect(marketInstructionNames.has("create_market")).to.equal(true);
    expect(marketInstructionNames.has("activate_market")).to.equal(true);
    expect(marketInstructionNames.has("mark_settled")).to.equal(true);

    const vaultInstructionNames = new Set(vaultIdl.instructions.map((i: any) => i.name));
    expect(vaultInstructionNames.has("init_market_vault")).to.equal(true);
    expect(vaultInstructionNames.has("deposit_collateral")).to.equal(true);

    const settlementInstructionNames = new Set(
      settlementIdl.instructions.map((i: any) => i.name),
    );
    expect(settlementInstructionNames.has("init_market_settlement")).to.equal(true);
    expect(settlementInstructionNames.has("submit_resolution")).to.equal(true);
    expect(settlementInstructionNames.has("claim_payout")).to.equal(true);
  });

  it("derives deterministic PDAs used across market/vault/settlement", async () => {
    const marketProgramId = new PublicKey(marketIdl.address);
    const vaultProgramId = new PublicKey(vaultIdl.address);
    const settlementProgramId = new PublicKey(settlementIdl.address);

    const marketId = 42n;
    const [protocol] = protocolPda(marketProgramId);
    const [market] = marketPda(marketProgramId, marketId);
    const [vault] = vaultPda(vaultProgramId, marketId);
    const [vaultAuthority] = vaultAuthorityPda(settlementProgramId, marketId);
    const [claim] = claimPda(settlementProgramId, marketId, provider.wallet.publicKey);

    expect(protocol).to.be.instanceOf(PublicKey);
    expect(market).to.be.instanceOf(PublicKey);
    expect(vault).to.be.instanceOf(PublicKey);
    expect(vaultAuthority).to.be.instanceOf(PublicKey);
    expect(claim).to.be.instanceOf(PublicKey);
  });

  it("computes bounded linear payoff correctly", async () => {
    const payoff = (observed: number, lower: number, upper: number) => {
      if (observed <= lower) return 0;
      if (observed >= upper) return 1_000_000;
      return Math.floor(((observed - lower) * 1_000_000) / (upper - lower));
    };

    expect(payoff(50, 100, 200)).to.equal(0);
    expect(payoff(250, 100, 200)).to.equal(1_000_000);
    expect(payoff(150, 100, 200)).to.equal(500_000);
  });

  it("enforces canonical market state transition map in tests", async () => {
    const allowed: Record<string, string[]> = {
      Preliminary: ["Active", "Cancelled"],
      Active: ["Paused", "PendingResolution", "Cancelled"],
      Paused: ["Active", "PendingResolution", "Cancelled"],
      PendingResolution: ["Resolved", "Cancelled"],
      Resolved: ["Settled"],
      Cancelled: ["Settled"],
      Settled: [],
    };

    expect(allowed.Preliminary.includes("Active")).to.equal(true);
    expect(allowed.PendingResolution.includes("Resolved")).to.equal(true);
    expect(allowed.Settled.length).to.equal(0);
  });
});
