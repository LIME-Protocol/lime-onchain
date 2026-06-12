import fs from "fs";
import path from "path";
import os from "os";
import * as anchor from "@coral-xyz/anchor";
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const marketIdl = JSON.parse(fs.readFileSync(path.join(root, "target/idl/lime_market.json"), "utf8"));
const vaultIdl = JSON.parse(fs.readFileSync(path.join(root, "target/idl/lime_vault.json"), "utf8"));
const settlementIdl = JSON.parse(fs.readFileSync(path.join(root, "target/idl/lime_settlement.json"), "utf8"));

const keypairPath = process.env.SOLANA_KEYPAIR ?? path.join(os.homedir(), ".config/solana/id.json");
const wallet = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(keypairPath, "utf8"))));

const rpcUrl = process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com";
const connection = new anchor.web3.Connection(rpcUrl, "confirmed");
const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(wallet), {
  commitment: "confirmed",
});
anchor.setProvider(provider);

const marketProgramId = new PublicKey(process.env.MARKET_PROGRAM_ID ?? marketIdl.address);
const vaultProgramId = new PublicKey(process.env.VAULT_PROGRAM_ID ?? vaultIdl.address);
const settlementProgramId = new PublicKey(process.env.SETTLEMENT_PROGRAM_ID ?? settlementIdl.address);
const protocolAdmin = new PublicKey(process.env.PROTOCOL_ADMIN ?? wallet.publicKey.toBase58());
const settlementResolver = new PublicKey(process.env.SETTLEMENT_RESOLVER ?? protocolAdmin.toBase58());
const canSignProtocolAdmin = protocolAdmin.equals(wallet.publicKey);

const marketProgram = new anchor.Program(
  { ...marketIdl, address: marketProgramId.toBase58() },
  provider,
);
const vaultProgram = new anchor.Program(
  { ...vaultIdl, address: vaultProgramId.toBase58() },
  provider,
);
const settlementProgram = new anchor.Program(
  { ...settlementIdl, address: settlementProgramId.toBase58() },
  provider,
);

const marketId = BigInt(process.env.LIME_SMOKE_MARKET_ID ?? "1");
const marketBytes = Buffer.alloc(8);
marketBytes.writeBigUInt64LE(marketId);

const [marketProtocol] = PublicKey.findProgramAddressSync([Buffer.from("protocol")], marketProgramId);
const [settlementProtocol] = PublicKey.findProgramAddressSync([Buffer.from("protocol")], settlementProgramId);
const [marketPda] = PublicKey.findProgramAddressSync([Buffer.from("market"), marketBytes], marketProgramId);
const [vaultPda] = PublicKey.findProgramAddressSync([Buffer.from("vault"), marketBytes], vaultProgramId);
const [vaultTokenAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault_authority"), marketBytes],
  vaultProgramId,
);
const [vaultAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault_authority"), marketBytes],
  settlementProgramId,
);

const usdcMint = new PublicKey(
  process.env.USDC_MINT ?? "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
);

const ensureIx = async (label, pda, fn) => {
  const info = await connection.getAccountInfo(pda);
  if (info) {
    console.log(`[skip] ${label} already initialized: ${pda.toBase58()}`);
    return null;
  }
  const sig = await fn();
  console.log(`[ok] ${label}: ${sig}`);
  return sig;
};

await ensureIx("market protocol", marketProtocol, async () =>
  (process.env.PROTOCOL_ADMIN
    ? marketProgram.methods.initializeProtocolFor(50, protocolAdmin)
    : marketProgram.methods.initializeProtocol(50))
    .accounts({
      admin: wallet.publicKey,
      protocolConfig: marketProtocol,
      systemProgram: SystemProgram.programId,
    })
    .rpc(),
);

await ensureIx("settlement protocol", settlementProtocol, async () =>
  (process.env.PROTOCOL_ADMIN
    ? settlementProgram.methods.initializeProtocolFor(settlementResolver, protocolAdmin)
    : settlementProgram.methods.initializeProtocol(settlementResolver))
    .accounts({
      admin: wallet.publicKey,
      protocolConfig: settlementProtocol,
      systemProgram: SystemProgram.programId,
    })
    .rpc(),
);

if (!canSignProtocolAdmin) {
  console.log(
    `[skip] smoke market requires protocol admin signature; deploy wallet ${wallet.publicKey.toBase58()} cannot sign for ${protocolAdmin.toBase58()}`,
  );
  printEnv();
  process.exit(0);
}

await ensureIx("sample market", marketPda, async () => {
  const resolutionTs = Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60;
  return marketProgram.methods
    .createMarket(
      new anchor.BN(marketId.toString()),
      new anchor.BN(0),
      new anchor.BN(1_000_000),
      new anchor.BN(resolutionTs),
      "devnet-smoke",
      { linear: {} },
      0,
    )
    .accounts({
      admin: wallet.publicKey,
      protocolConfig: marketProtocol,
      market: marketPda,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
});

const marketData = await marketProgram.account.market.fetch(marketPda);
if (Object.keys(marketData.status)[0] === "preliminary") {
  const sig = await marketProgram.methods
    .activateMarket()
    .accounts({
      admin: wallet.publicKey,
      protocolConfig: marketProtocol,
      market: marketPda,
    })
    .rpc();
  console.log(`[ok] market activation: ${sig}`);
}

let vaultTokenAccount = process.env.VAULT_TOKEN_ACCOUNT
  ? new PublicKey(process.env.VAULT_TOKEN_ACCOUNT)
  : null;

const vaultInfo = await connection.getAccountInfo(vaultPda);
if (!vaultInfo) {
  if (!vaultTokenAccount) {
    const ata = await getOrCreateAssociatedTokenAccount(
      connection,
      wallet,
      usdcMint,
      vaultTokenAuthority,
      true,
    );
    vaultTokenAccount = ata.address;
    console.log(`[ok] created vault token account: ${vaultTokenAccount.toBase58()}`);
  }

  const sig = await vaultProgram.methods
    .initMarketVault(new anchor.BN(marketId.toString()), vaultAuthority)
    .accounts({
      payer: wallet.publicKey,
      usdcMint,
      market: marketPda,
      vaultAuthority: vaultTokenAuthority,
      marketVault: vaultPda,
      vaultTokenAccount,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log(`[ok] market vault init: ${sig}`);
}

await ensureIx("market settlement", vaultAuthority, async () =>
  settlementProgram.methods
    .initMarketSettlement(new anchor.BN(marketId.toString()))
    .accounts({
      admin: wallet.publicKey,
      protocolConfig: settlementProtocol,
      market: marketPda,
      marketVault: vaultPda,
      vaultAuthority,
      systemProgram: SystemProgram.programId,
    })
    .rpc(),
);

printEnv();

function printEnv() {
  console.log("\nDevnet bootstrap complete. Use these frontend env values:");
  console.log(`VITE_SOLANA_RPC_URL=${rpcUrl}`);
  console.log(`VITE_SOLANA_USDC_MINT=${usdcMint.toBase58()}`);
  console.log(`VITE_LIME_MARKET_PROGRAM_ID=${marketProgramId.toBase58()}`);
  console.log(`VITE_LIME_VAULT_PROGRAM_ID=${vaultProgramId.toBase58()}`);
  console.log(`VITE_LIME_SETTLEMENT_PROGRAM_ID=${settlementProgramId.toBase58()}`);
  console.log(`VITE_LIME_DEFAULT_MARKET_ID=${marketId.toString()}`);
  console.log(`PROTOCOL_ADMIN=${protocolAdmin.toBase58()}`);
  console.log(`SETTLEMENT_RESOLVER=${settlementResolver.toBase58()}`);
}
