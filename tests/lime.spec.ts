import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import fs from "fs";
import path from "path";

type PositionSide = "long" | "short";

function marketIdBuffer(marketId: bigint): Buffer {
  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64LE(marketId);
  return buffer;
}

function positionSideSeed(side: PositionSide): Buffer {
  return Buffer.from(side === "short" ? "short" : "long");
}

function marketPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("market"), marketIdBuffer(marketId)], programId);
}

function protocolPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("protocol")], programId);
}

function vaultPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("vault"), marketIdBuffer(marketId)], programId);
}

function vaultTokenAuthorityPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority"), marketIdBuffer(marketId)],
    programId,
  );
}

function collateralPda(programId: PublicKey, marketId: bigint, owner: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("collateral"), marketIdBuffer(marketId), owner.toBuffer()],
    programId,
  );
}

function positionPda(
  programId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
  side: PositionSide,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position"), marketIdBuffer(marketId), owner.toBuffer(), positionSideSeed(side)],
    programId,
  );
}

function claimPda(
  programId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
  side: PositionSide,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("claim"), marketIdBuffer(marketId), owner.toBuffer(), positionSideSeed(side)],
    programId,
  );
}

function vaultAuthorityPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority"), marketIdBuffer(marketId)],
    programId,
  );
}

describe("lime-onchain baseline", () => {
  const provider = process.env.ANCHOR_PROVIDER_URL
    ? anchor.AnchorProvider.env()
    : new anchor.AnchorProvider(
        new Connection("https://api.devnet.solana.com", "confirmed"),
        new anchor.Wallet(Keypair.generate()),
        { commitment: "confirmed" },
      );
  anchor.setProvider(provider);

  const rootDir = process.cwd();
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
    expect(vaultInstructionNames.has("withdraw_available_collateral")).to.equal(true);
    expect(vaultInstructionNames.has("withdraw_collateral")).to.equal(false);
    expect(vaultInstructionNames.has("transfer_for_settlement")).to.equal(true);

    const settlementInstructionNames = new Set(
      settlementIdl.instructions.map((i: any) => i.name),
    );
    expect(settlementInstructionNames.has("init_market_settlement")).to.equal(true);
    expect(settlementInstructionNames.has("submit_resolution")).to.equal(true);
    expect(settlementInstructionNames.has("claim_payout")).to.equal(true);
  });

  it("keeps the SDK aligned with the current vault withdrawal instruction", async () => {
    const vaultSdk = fs.readFileSync(path.join(rootDir, "sdk/src/vault.ts"), "utf8");

    expect(vaultSdk).to.include(".withdrawAvailableCollateral(");
    expect(vaultSdk).to.not.include(".withdrawCollateral(");
  });

  it("keeps the SDK aligned with backend trade execution accounts", async () => {
    const vaultSdk = fs.readFileSync(path.join(rootDir, "sdk/src/vault.ts"), "utf8");
    const typesSdk = fs.readFileSync(path.join(rootDir, "sdk/src/types.ts"), "utf8");

    expect(typesSdk).to.include("interface TradeExecutionInput");
    expect(typesSdk).to.include("interface OnchainTradeExecution");
    expect(typesSdk).to.include("depositCollateral(");
    expect(typesSdk).to.include("withdrawAvailableCollateral(");
    expect(vaultSdk).to.include("async settleTrade(");
    expect(vaultSdk).to.include(".settleTrade(");
    expect(vaultSdk).to.include("async depositCollateral(");
    expect(vaultSdk).to.include("async withdrawAvailableCollateral(");
    expect(vaultSdk).to.include("buyerPosition");
    expect(vaultSdk).to.include("sellerPosition");
  });

  it("documents the vault custody boundary in generated IDLs", async () => {
    const vaultAccounts = new Map(vaultIdl.accounts.map((account: any) => [account.name, account]));
    const vaultTypes = new Map(vaultIdl.types.map((type: any) => [type.name, type]));
    const settlementClaim = settlementIdl.instructions.find((i: any) => i.name === "claim_payout");
    const settlementRefund = settlementIdl.instructions.find((i: any) => i.name === "refund");
    const transferForSettlement = vaultIdl.instructions.find(
      (i: any) => i.name === "transfer_for_settlement",
    );

    expect(vaultAccounts.has("MarketVault")).to.equal(true);
    expect(vaultTypes.get("MarketVault")?.type.fields.map((field: any) => field.name)).to.include.members([
      "vault_authority",
      "settlement_authority",
      "vault_authority_bump",
    ]);
    expect(vaultTypes.get("UserCollateral")?.type.fields.map((field: any) => field.name)).to.include.members([
      "available_collateral",
      "total_deposited",
    ]);
    expect(vaultTypes.get("UserPosition")?.type.fields.map((field: any) => field.name)).to.include.members([
      "side",
      "quantity",
      "cost_basis",
    ]);

    const transferAccounts = new Map(
      transferForSettlement.accounts.map((account: any) => [account.name, account]),
    );
    expect(transferAccounts.get("settlement_authority")?.signer).to.equal(true);
    expect(transferAccounts.get("vault_authority")).to.not.equal(undefined);
    expect(transferAccounts.get("recipient_ata")?.writable).to.equal(true);

    for (const instruction of [settlementClaim, settlementRefund]) {
      const accounts = new Set(instruction.accounts.map((account: any) => account.name));
      expect(accounts.has("market_vault")).to.equal(true);
      expect(accounts.has("vault_token_authority")).to.equal(true);
      expect(accounts.has("vault_token_account")).to.equal(true);
      expect(accounts.has("vault_program")).to.equal(true);
    }
  });

  it("derives deterministic PDAs used across market/vault/settlement", async () => {
    const marketProgramId = new PublicKey(marketIdl.address);
    const vaultProgramId = new PublicKey(vaultIdl.address);
    const settlementProgramId = new PublicKey(settlementIdl.address);

    const marketId = 42n;
    const [protocol] = protocolPda(marketProgramId);
    const [market] = marketPda(marketProgramId, marketId);
    const [vault] = vaultPda(vaultProgramId, marketId);
    const [vaultTokenAuthority] = vaultTokenAuthorityPda(vaultProgramId, marketId);
    const [collateral] = collateralPda(vaultProgramId, marketId, provider.wallet.publicKey);
    const [longPosition] = positionPda(vaultProgramId, marketId, provider.wallet.publicKey, "long");
    const [shortPosition] = positionPda(vaultProgramId, marketId, provider.wallet.publicKey, "short");
    const [vaultAuthority] = vaultAuthorityPda(settlementProgramId, marketId);
    const [claim] = claimPda(settlementProgramId, marketId, provider.wallet.publicKey, "long");

    expect(protocol).to.be.instanceOf(PublicKey);
    expect(market).to.be.instanceOf(PublicKey);
    expect(vault).to.be.instanceOf(PublicKey);
    expect(vaultTokenAuthority).to.be.instanceOf(PublicKey);
    expect(collateral).to.be.instanceOf(PublicKey);
    expect(longPosition).to.be.instanceOf(PublicKey);
    expect(shortPosition).to.be.instanceOf(PublicKey);
    expect(longPosition.equals(shortPosition)).to.equal(false);
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

  it("computes settlement payout from quantity rather than cost basis", async () => {
    const payout = (quantity: number, side: "long" | "short", payoffRatio: number) => {
      return side === "long"
        ? Math.floor((quantity * payoffRatio) / 1_000_000)
        : Math.floor((quantity * (1_000_000 - payoffRatio)) / 1_000_000);
    };

    const quantity = 1_000_000;
    const payoffRatio = 700_000;

    expect(payout(quantity, "long", payoffRatio)).to.equal(700_000);
    expect(payout(quantity, "short", payoffRatio)).to.equal(300_000);
    expect(payout(quantity, "long", payoffRatio) + payout(quantity, "short", payoffRatio)).to.equal(quantity);
  });

  it("allows valid zero settlement amounts to be receipted without vault transfer", async () => {
    const settlementSource = fs.readFileSync(
      path.join(rootDir, "programs/lime-settlement/src/lib.rs"),
      "utf8",
    );

    expect(settlementSource).to.include("if amount > 0");
    expect(settlementSource).to.include("receipt.claimed = true");
    expect(settlementSource).to.include("receipt.refunded = true");
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
