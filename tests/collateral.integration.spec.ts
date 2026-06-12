import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import {
  Ed25519Program,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  createMint,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import fs from "fs";
import path from "path";
import {
  encodeSignedOrder,
  type SignedOrderInput,
} from "../sdk/src/signed-order";

function marketIdBuffer(marketId: bigint): Buffer {
  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64LE(marketId);
  return buffer;
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

function vaultAuthorityPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
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

function settlementProtocolPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("protocol")], programId);
}

type PositionSide = "long" | "short";

function positionSideSeed(side: PositionSide): Buffer {
  return Buffer.from(side === "short" ? "short" : "long");
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

function resolutionPda(programId: PublicKey, marketId: bigint): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("resolution"), marketIdBuffer(marketId)],
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

function refundPda(
  programId: PublicKey,
  marketId: bigint,
  owner: PublicKey,
  side: PositionSide,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("refund"), marketIdBuffer(marketId), owner.toBuffer(), positionSideSeed(side)],
    programId,
  );
}

function isLocalProviderUrl(url: string | undefined): boolean {
  return !!url && (url.includes("127.0.0.1") || url.includes("localhost"));
}

const SYSVAR_INSTRUCTIONS_PUBKEY = new PublicKey("Sysvar1nstructions1111111111111111111111111");

function anchorNetwork(network: SignedOrderInput["network"]): Record<string, Record<string, never>> {
  if (network === "mainnet-beta") return { mainnetBeta: {} };
  if (network === "devnet") return { devnet: {} };
  return { localnet: {} };
}

function anchorSide(side: SignedOrderInput["side"]): Record<string, Record<string, never>> {
  return side === "buy" ? { buy: {} } : { sell: {} };
}

function anchorSignedOrder(order: SignedOrderInput): Record<string, unknown> {
  return {
    version: order.version,
    network: anchorNetwork(order.network),
    marketProgramId: order.marketProgramId,
    vaultProgramId: order.vaultProgramId,
    marketId: new anchor.BN(order.marketId.toString()),
    owner: order.owner,
    side: anchorSide(order.side),
    priceScaled: new anchor.BN(order.priceScaled.toString()),
    quantity: new anchor.BN(order.quantity.toString()),
    expirationTs: new anchor.BN(order.expirationTs.toString()),
    nonce: new anchor.BN(order.nonce.toString()),
  };
}

function signedOrder(input: {
  marketProgramId: PublicKey;
  vaultProgramId: PublicKey;
  marketId: bigint;
  owner: PublicKey;
  side: SignedOrderInput["side"];
  priceScaled: bigint;
  quantity: bigint;
  expirationTs?: bigint;
  nonce: bigint;
}): SignedOrderInput {
  return {
    version: 1,
    network: "localnet",
    marketProgramId: input.marketProgramId,
    vaultProgramId: input.vaultProgramId,
    marketId: input.marketId,
    owner: input.owner,
    side: input.side,
    priceScaled: input.priceScaled,
    quantity: input.quantity,
    expirationTs: input.expirationTs ?? BigInt(Math.floor(Date.now() / 1000) + 3600),
    nonce: input.nonce,
  };
}

async function settleSignedTrade(input: {
  provider: anchor.AnchorProvider;
  vaultProgram: anchor.Program;
  marketProgramId: PublicKey;
  vaultProgramId: PublicKey;
  market: PublicKey;
  marketVault: PublicKey;
  buyer: Keypair;
  seller: Keypair;
  buyerCollateral: PublicKey;
  sellerCollateral: PublicKey;
  marketId: bigint;
  quantity: bigint;
  buyerPriceScaled?: bigint;
  sellerPriceScaled: bigint;
  buyerNonce: bigint;
  sellerNonce: bigint;
  buyerOrderOverride?: Partial<SignedOrderInput>;
  sellerOrderOverride?: Partial<SignedOrderInput>;
  omitBuyerSignature?: boolean;
  omitSellerSignature?: boolean;
  signBuyerOrderWithSellerKey?: boolean;
  signSellerOrderWithBuyerKey?: boolean;
  swapSignatureMessages?: boolean;
  signWrongSellerMessage?: boolean;
}): Promise<{
  buyerOrder: SignedOrderInput;
  sellerOrder: SignedOrderInput;
  buyerPosition: PublicKey;
  sellerPosition: PublicKey;
  buyerFill: PublicKey;
  sellerFill: PublicKey;
  signature: string;
}> {
  const buyerOrder = {
    ...signedOrder({
      marketProgramId: input.marketProgramId,
      vaultProgramId: input.vaultProgramId,
      marketId: input.marketId,
      owner: input.buyer.publicKey,
      side: "buy",
      priceScaled: input.buyerPriceScaled ?? input.sellerPriceScaled,
      quantity: input.quantity,
      nonce: input.buyerNonce,
    }),
    ...input.buyerOrderOverride,
  };
  const sellerOrder = {
    ...signedOrder({
      marketProgramId: input.marketProgramId,
      vaultProgramId: input.vaultProgramId,
      marketId: input.marketId,
      owner: input.seller.publicKey,
      side: "sell",
      priceScaled: input.sellerPriceScaled,
      quantity: input.quantity,
      nonce: input.sellerNonce,
    }),
    ...input.sellerOrderOverride,
  };
  const [buyerFill] = fillPda(input.vaultProgramId, buyerOrder.marketId, buyerOrder.owner, buyerOrder.nonce);
  const [sellerFill] = fillPda(input.vaultProgramId, sellerOrder.marketId, sellerOrder.owner, sellerOrder.nonce);
  const [buyerPosition] = positionPda(input.vaultProgramId, buyerOrder.marketId, buyerOrder.owner, "long");
  const [sellerPosition] = positionPda(input.vaultProgramId, sellerOrder.marketId, sellerOrder.owner, "short");
  const transaction = new Transaction();

  if (!input.omitBuyerSignature) {
    transaction.add(
      Ed25519Program.createInstructionWithPrivateKey({
        privateKey: input.signBuyerOrderWithSellerKey
          ? input.seller.secretKey
          : input.buyer.secretKey,
        message: input.swapSignatureMessages
          ? encodeSignedOrder(sellerOrder)
          : encodeSignedOrder(buyerOrder),
      }),
    );
  }
  if (!input.omitSellerSignature) {
    transaction.add(
      Ed25519Program.createInstructionWithPrivateKey({
        privateKey: input.signSellerOrderWithBuyerKey
          ? input.buyer.secretKey
          : input.seller.secretKey,
        message: input.swapSignatureMessages
          ? encodeSignedOrder(buyerOrder)
          : input.signWrongSellerMessage
            ? encodeSignedOrder({ ...sellerOrder, priceScaled: sellerOrder.priceScaled + 1n })
            : encodeSignedOrder(sellerOrder),
      }),
    );
  }
  transaction.add(
    await input.vaultProgram.methods
      .settleTrade(
        anchorSignedOrder(buyerOrder),
        anchorSignedOrder(sellerOrder),
        new anchor.BN(input.quantity.toString()),
      )
      .accounts({
        submitter: input.provider.wallet.publicKey,
        market: input.market,
        marketVault: input.marketVault,
        buyerCollateral: input.buyerCollateral,
        sellerCollateral: input.sellerCollateral,
        buyerFill,
        sellerFill,
        buyerPosition,
        sellerPosition,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: SystemProgram.programId,
      })
      .instruction(),
  );

  const signature = await input.provider.sendAndConfirm(transaction, []);
  return { buyerOrder, sellerOrder, buyerPosition, sellerPosition, buyerFill, sellerFill, signature };
}

describe("collateral integration", () => {
  const providerUrl = process.env.ANCHOR_PROVIDER_URL;
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

  let provider: anchor.AnchorProvider;
  let marketProgram: anchor.Program;
  let vaultProgram: anchor.Program;
  let settlementProgram: anchor.Program;
  let payer: Keypair;

  before(function () {
    if (!isLocalProviderUrl(providerUrl)) {
      this.skip();
    }

    provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    marketProgram = new anchor.Program(marketIdl, provider);
    vaultProgram = new anchor.Program(vaultIdl, provider);
    settlementProgram = new anchor.Program(settlementIdl, provider);
    payer = (provider.wallet as any).payer as Keypair;
  });

  it("lets a user deposit and withdraw only available market collateral", async () => {
    expect(payer).to.not.equal(undefined);

    const marketProgramId = new PublicKey(marketIdl.address);
    const vaultProgramId = new PublicKey(vaultIdl.address);
    const user = Keypair.generate();
    const marketId = BigInt(Date.now());
    const depositAmount = 5_000_000n;
    const withdrawAmount = 2_000_000n;

    const payerAirdropSignature = await provider.connection.requestAirdrop(
      payer.publicKey,
      5 * LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(payerAirdropSignature, "confirmed");

    const airdropSignature = await provider.connection.requestAirdrop(
      user.publicKey,
      LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(airdropSignature, "confirmed");

    const usdcMint = await createMint(
      provider.connection,
      payer,
      payer.publicKey,
      null,
      6,
    );
    const userAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      user.publicKey,
    );

    await mintTo(
      provider.connection,
      payer,
      usdcMint,
      userAta.address,
      payer,
      depositAmount,
    );

    const [protocolConfig] = protocolPda(marketProgramId);
    const existingProtocol = await (marketProgram.account as any).protocolConfig.fetchNullable(
      protocolConfig,
    );
    if (!existingProtocol) {
      await marketProgram.methods
        .initializeProtocol(50)
        .accounts({
          admin: payer.publicKey,
          protocolConfig,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    const [market] = marketPda(marketProgramId, marketId);
    await marketProgram.methods
      .createMarket(
        new anchor.BN(marketId.toString()),
        new anchor.BN(100),
        new anchor.BN(200),
        new anchor.BN(Math.floor(Date.now() / 1000) + 3600),
        "integration-test",
        { linear: {} },
        0,
      )
      .accounts({
        admin: payer.publicKey,
        protocolConfig,
        market,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const [marketVault] = vaultPda(vaultProgramId, marketId);
    const [vaultAuthority] = vaultAuthorityPda(vaultProgramId, marketId);
    const [settlementAuthority] = vaultAuthorityPda(new PublicKey("3YMsnQEW4koSRwLJw1gUeyf6S53GxNFQWSGjRr3NMjeo"), marketId);
    const vaultTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      vaultAuthority,
      true,
    );

    await vaultProgram.methods
      .initMarketVault(new anchor.BN(marketId.toString()), settlementAuthority)
      .accounts({
        payer: payer.publicKey,
        usdcMint,
        market,
        vaultAuthority,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const [userCollateral] = collateralPda(vaultProgramId, marketId, user.publicKey);
    await vaultProgram.methods
      .depositCollateral(
        new anchor.BN(marketId.toString()),
        new anchor.BN(depositAmount.toString()),
      )
      .accounts({
        user: user.publicKey,
        usdcMint,
        market,
        userAta: userAta.address,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        userCollateral,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    await vaultProgram.methods
      .withdrawAvailableCollateral(
        new anchor.BN(marketId.toString()),
        new anchor.BN(withdrawAmount.toString()),
      )
      .accounts({
        user: user.publicKey,
        market,
        usdcMint,
        marketVault,
        vaultAuthority,
        vaultTokenAccount: vaultTokenAccount.address,
        userAta: userAta.address,
        userCollateral,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    const collateral = await (vaultProgram.account as any).userCollateral.fetch(userCollateral);
    const vault = await (vaultProgram.account as any).marketVault.fetch(marketVault);
    const userTokenAccount = await getAccount(provider.connection, userAta.address);
    const vaultTokenAccountAfter = await getAccount(provider.connection, vaultTokenAccount.address);

    expect(BigInt(collateral.availableCollateral.toString())).to.equal(
      depositAmount - withdrawAmount,
    );
    expect(BigInt(collateral.totalDeposited.toString())).to.equal(depositAmount);
    expect(BigInt(vault.totalCollateral.toString())).to.equal(depositAmount - withdrawAmount);
    expect(userTokenAccount.amount).to.equal(withdrawAmount);
    expect(vaultTokenAccountAfter.amount).to.equal(depositAmount - withdrawAmount);
  });

  it("settles a signed-order trade into long and short position accounts", async () => {
    expect(payer).to.not.equal(undefined);

    const marketProgramId = new PublicKey(marketIdl.address);
    const vaultProgramId = new PublicKey(vaultIdl.address);
    const buyer = Keypair.generate();
    const seller = Keypair.generate();
    const marketId = BigInt(Date.now() + 1);
    const buyerDeposit = 8_000_000n;
    const sellerDeposit = 8_000_000n;
    const quantity = 4_000_000n;
    const priceScaled = 650_000n;
    const buyerCost = (quantity * priceScaled) / 1_000_000n;
    const sellerCost = quantity - buyerCost;

    const payerAirdropSignature = await provider.connection.requestAirdrop(
      payer.publicKey,
      5 * LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(payerAirdropSignature, "confirmed");

    for (const trader of [buyer, seller]) {
      const signature = await provider.connection.requestAirdrop(
        trader.publicKey,
        LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(signature, "confirmed");
    }

    const usdcMint = await createMint(
      provider.connection,
      payer,
      payer.publicKey,
      null,
      6,
    );
    const buyerAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      buyer.publicKey,
    );
    const sellerAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      seller.publicKey,
    );

    await mintTo(
      provider.connection,
      payer,
      usdcMint,
      buyerAta.address,
      payer,
      buyerDeposit,
    );
    await mintTo(
      provider.connection,
      payer,
      usdcMint,
      sellerAta.address,
      payer,
      sellerDeposit,
    );

    const [protocolConfig] = protocolPda(marketProgramId);
    const existingProtocol = await (marketProgram.account as any).protocolConfig.fetchNullable(
      protocolConfig,
    );
    if (!existingProtocol) {
      await marketProgram.methods
        .initializeProtocol(50)
        .accounts({
          admin: payer.publicKey,
          protocolConfig,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    const [market] = marketPda(marketProgramId, marketId);
    await marketProgram.methods
      .createMarket(
        new anchor.BN(marketId.toString()),
        new anchor.BN(100),
        new anchor.BN(200),
        new anchor.BN(Math.floor(Date.now() / 1000) + 3600),
        "integration-test",
        { linear: {} },
        0,
      )
      .accounts({
        admin: payer.publicKey,
        protocolConfig,
        market,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await marketProgram.methods
      .forceActivateMarket()
      .accounts({
        admin: payer.publicKey,
        protocolConfig,
        market,
      })
      .rpc();

    const [marketVault] = vaultPda(vaultProgramId, marketId);
    const [vaultAuthority] = vaultAuthorityPda(vaultProgramId, marketId);
    const [settlementAuthority] = vaultAuthorityPda(
      new PublicKey("3YMsnQEW4koSRwLJw1gUeyf6S53GxNFQWSGjRr3NMjeo"),
      marketId,
    );
    const vaultTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      vaultAuthority,
      true,
    );

    await vaultProgram.methods
      .initMarketVault(new anchor.BN(marketId.toString()), settlementAuthority)
      .accounts({
        payer: payer.publicKey,
        usdcMint,
        market,
        vaultAuthority,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const [buyerCollateral] = collateralPda(vaultProgramId, marketId, buyer.publicKey);
    const [sellerCollateral] = collateralPda(vaultProgramId, marketId, seller.publicKey);

    await vaultProgram.methods
      .depositCollateral(
        new anchor.BN(marketId.toString()),
        new anchor.BN(buyerDeposit.toString()),
      )
      .accounts({
        user: buyer.publicKey,
        usdcMint,
        market,
        userAta: buyerAta.address,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        userCollateral: buyerCollateral,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    await vaultProgram.methods
      .depositCollateral(
        new anchor.BN(marketId.toString()),
        new anchor.BN(sellerDeposit.toString()),
      )
      .accounts({
        user: seller.publicKey,
        usdcMint,
        market,
        userAta: sellerAta.address,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        userCollateral: sellerCollateral,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([seller])
      .rpc();

    try {
      await settleSignedTrade({
        provider,
        vaultProgram,
        marketProgramId,
        vaultProgramId,
        market,
        marketVault,
        buyer,
        seller,
        buyerCollateral,
        sellerCollateral,
        marketId,
        quantity,
        sellerPriceScaled: priceScaled,
        buyerNonce: 1n,
        sellerNonce: 2n,
        omitBuyerSignature: true,
      });
      throw new Error("Expected missing buyer signature to be rejected");
    } catch (error) {
      expect(String(error)).to.include("SignedOrderSignatureMissing");
    }

    try {
      await settleSignedTrade({
        provider,
        vaultProgram,
        marketProgramId,
        vaultProgramId,
        market,
        marketVault,
        buyer,
        seller,
        buyerCollateral,
        sellerCollateral,
        marketId,
        quantity,
        sellerPriceScaled: priceScaled,
        buyerNonce: 16n,
        sellerNonce: 17n,
        omitSellerSignature: true,
      });
      throw new Error("Expected missing seller signature to be rejected");
    } catch (error) {
      expect(String(error)).to.include("SignedOrderSignatureMissing");
    }

    try {
      await settleSignedTrade({
        provider,
        vaultProgram,
        marketProgramId,
        vaultProgramId,
        market,
        marketVault,
        buyer,
        seller,
        buyerCollateral,
        sellerCollateral,
        marketId,
        quantity,
        sellerPriceScaled: priceScaled,
        buyerNonce: 18n,
        sellerNonce: 19n,
        signBuyerOrderWithSellerKey: true,
      });
      throw new Error("Expected buyer order signed by seller to be rejected");
    } catch (error) {
      expect(String(error)).to.include("SignedOrderSignatureMissing");
    }

    try {
      await settleSignedTrade({
        provider,
        vaultProgram,
        marketProgramId,
        vaultProgramId,
        market,
        marketVault,
        buyer,
        seller,
        buyerCollateral,
        sellerCollateral,
        marketId,
        quantity,
        sellerPriceScaled: priceScaled,
        buyerNonce: 10n,
        sellerNonce: 11n,
        signWrongSellerMessage: true,
      });
      throw new Error("Expected mismatched seller signature to be rejected");
    } catch (error) {
      expect(String(error)).to.include("SignedOrderSignatureMismatch");
    }

    try {
      await settleSignedTrade({
        provider,
        vaultProgram,
        marketProgramId,
        vaultProgramId,
        market,
        marketVault,
        buyer,
        seller,
        buyerCollateral,
        sellerCollateral,
        marketId,
        quantity,
        sellerPriceScaled: priceScaled,
        buyerNonce: 20n,
        sellerNonce: 21n,
        swapSignatureMessages: true,
      });
      throw new Error("Expected swapped signature messages to be rejected");
    } catch (error) {
      expect(String(error)).to.include("SignedOrderSignatureMismatch");
    }

    try {
      await settleSignedTrade({
        provider,
        vaultProgram,
        marketProgramId,
        vaultProgramId,
        market,
        marketVault,
        buyer,
        seller,
        buyerCollateral,
        sellerCollateral,
        marketId,
        quantity,
        sellerPriceScaled: priceScaled,
        buyerNonce: 12n,
        sellerNonce: 13n,
        buyerOrderOverride: {
          expirationTs: BigInt(Math.floor(Date.now() / 1000) - 1),
        },
      });
      throw new Error("Expected expired buyer order to be rejected");
    } catch (error) {
      expect(String(error)).to.include("SignedOrderExpired");
    }

    try {
      await settleSignedTrade({
        provider,
        vaultProgram,
        marketProgramId,
        vaultProgramId,
        market,
        marketVault,
        buyer,
        seller,
        buyerCollateral,
        sellerCollateral,
        marketId,
        quantity,
        buyerPriceScaled: priceScaled - 1n,
        sellerPriceScaled: priceScaled,
        buyerNonce: 14n,
        sellerNonce: 15n,
      });
      throw new Error("Expected uncrossed order pair to be rejected");
    } catch (error) {
      expect(String(error)).to.include("SignedOrderNotCrossed");
    }

    const { buyerPosition, sellerPosition } = await settleSignedTrade({
      provider,
      vaultProgram,
      marketProgramId,
      vaultProgramId,
      market,
      marketVault,
      buyer,
      seller,
      buyerCollateral,
      sellerCollateral,
      marketId,
      quantity,
      sellerPriceScaled: priceScaled,
      buyerNonce: 1n,
      sellerNonce: 2n,
    });

    try {
      await settleSignedTrade({
        provider,
        vaultProgram,
        marketProgramId,
        vaultProgramId,
        market,
        marketVault,
        buyer,
        seller,
        buyerCollateral,
        sellerCollateral,
        marketId,
        quantity,
        sellerPriceScaled: priceScaled,
        buyerNonce: 1n,
        sellerNonce: 2n,
      });
      throw new Error("Expected overfill to be rejected");
    } catch (error) {
      expect(String(error)).to.include("SignedOrderOverfilled");
    }

    try {
      await vaultProgram.methods
        .withdrawAvailableCollateral(
          new anchor.BN(marketId.toString()),
          new anchor.BN(buyerDeposit.toString()),
        )
        .accounts({
          user: buyer.publicKey,
          market,
          usdcMint,
          marketVault,
          vaultAuthority,
          vaultTokenAccount: vaultTokenAccount.address,
          userAta: buyerAta.address,
          userCollateral: buyerCollateral,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([buyer])
        .rpc();
      throw new Error("Expected reserved collateral withdrawal to be rejected");
    } catch (error) {
      expect(String(error)).to.include("InsufficientAvailableCollateral");
    }

    const buyerCollateralAccount = await (vaultProgram.account as any).userCollateral.fetch(
      buyerCollateral,
    );
    const sellerCollateralAccount = await (vaultProgram.account as any).userCollateral.fetch(
      sellerCollateral,
    );
    const buyerPositionAccount = await (vaultProgram.account as any).userPosition.fetch(
      buyerPosition,
    );
    const sellerPositionAccount = await (vaultProgram.account as any).userPosition.fetch(
      sellerPosition,
    );
    const vault = await (vaultProgram.account as any).marketVault.fetch(marketVault);
    const vaultTokenAccountAfter = await getAccount(provider.connection, vaultTokenAccount.address);

    expect(BigInt(buyerCollateralAccount.availableCollateral.toString())).to.equal(
      buyerDeposit - buyerCost,
    );
    expect(BigInt(sellerCollateralAccount.availableCollateral.toString())).to.equal(
      sellerDeposit - sellerCost,
    );
    expect(BigInt(buyerPositionAccount.quantity.toString())).to.equal(quantity);
    expect(BigInt(buyerPositionAccount.costBasis.toString())).to.equal(buyerCost);
    expect(buyerPositionAccount.side).to.deep.equal({ long: {} });
    expect(BigInt(sellerPositionAccount.quantity.toString())).to.equal(quantity);
    expect(BigInt(sellerPositionAccount.costBasis.toString())).to.equal(sellerCost);
    expect(sellerPositionAccount.side).to.deep.equal({ short: {} });
    expect(BigInt(vault.totalCollateral.toString())).to.equal(buyerDeposit + sellerDeposit);
    expect(vaultTokenAccountAfter.amount).to.equal(buyerDeposit + sellerDeposit);
  });

  it("claims resolved long payout through the settlement program and vault CPI", async () => {
    expect(payer).to.not.equal(undefined);

    const marketProgramId = new PublicKey(marketIdl.address);
    const vaultProgramId = new PublicKey(vaultIdl.address);
    const settlementProgramId = new PublicKey(settlementIdl.address);
    const buyer = Keypair.generate();
    const seller = Keypair.generate();
    const marketId = BigInt(Date.now() + 2);
    const buyerDeposit = 8_000_000n;
    const sellerDeposit = 8_000_000n;
    const quantity = 4_000_000n;
    const priceScaled = 650_000n;
    const observedValue = 170;
    const expectedPayout = 2_800_000n;

    const payerAirdropSignature = await provider.connection.requestAirdrop(
      payer.publicKey,
      5 * LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(payerAirdropSignature, "confirmed");

    for (const trader of [buyer, seller]) {
      const signature = await provider.connection.requestAirdrop(
        trader.publicKey,
        LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(signature, "confirmed");
    }

    const usdcMint = await createMint(
      provider.connection,
      payer,
      payer.publicKey,
      null,
      6,
    );
    const buyerAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      buyer.publicKey,
    );
    const sellerAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      seller.publicKey,
    );

    await mintTo(
      provider.connection,
      payer,
      usdcMint,
      buyerAta.address,
      payer,
      buyerDeposit,
    );
    await mintTo(
      provider.connection,
      payer,
      usdcMint,
      sellerAta.address,
      payer,
      sellerDeposit,
    );

    const [marketProtocolConfig] = protocolPda(marketProgramId);
    const existingMarketProtocol = await (marketProgram.account as any).protocolConfig.fetchNullable(
      marketProtocolConfig,
    );
    if (!existingMarketProtocol) {
      await marketProgram.methods
        .initializeProtocol(50)
        .accounts({
          admin: payer.publicKey,
          protocolConfig: marketProtocolConfig,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    const [settlementProtocolConfig] = settlementProtocolPda(settlementProgramId);
    const existingSettlementProtocol = await (settlementProgram.account as any).protocolConfig.fetchNullable(
      settlementProtocolConfig,
    );
    if (!existingSettlementProtocol) {
      await settlementProgram.methods
        .initializeProtocol(payer.publicKey)
        .accounts({
          admin: payer.publicKey,
          protocolConfig: settlementProtocolConfig,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    const [market] = marketPda(marketProgramId, marketId);
    await marketProgram.methods
      .createMarket(
        new anchor.BN(marketId.toString()),
        new anchor.BN(100),
        new anchor.BN(200),
        new anchor.BN(Math.floor(Date.now() / 1000) + 3600),
        "integration-test",
        { linear: {} },
        0,
      )
      .accounts({
        admin: payer.publicKey,
        protocolConfig: marketProtocolConfig,
        market,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await marketProgram.methods
      .forceActivateMarket()
      .accounts({
        admin: payer.publicKey,
        protocolConfig: marketProtocolConfig,
        market,
      })
      .rpc();

    const [marketVault] = vaultPda(vaultProgramId, marketId);
    const [vaultTokenAuthority] = vaultAuthorityPda(vaultProgramId, marketId);
    const [settlementAuthority] = vaultAuthorityPda(settlementProgramId, marketId);
    const vaultTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      vaultTokenAuthority,
      true,
    );

    await vaultProgram.methods
      .initMarketVault(new anchor.BN(marketId.toString()), settlementAuthority)
      .accounts({
        payer: payer.publicKey,
        usdcMint,
        market,
        vaultAuthority: vaultTokenAuthority,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const [buyerCollateral] = collateralPda(vaultProgramId, marketId, buyer.publicKey);
    const [sellerCollateral] = collateralPda(vaultProgramId, marketId, seller.publicKey);

    await vaultProgram.methods
      .depositCollateral(
        new anchor.BN(marketId.toString()),
        new anchor.BN(buyerDeposit.toString()),
      )
      .accounts({
        user: buyer.publicKey,
        usdcMint,
        market,
        userAta: buyerAta.address,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        userCollateral: buyerCollateral,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    await vaultProgram.methods
      .depositCollateral(
        new anchor.BN(marketId.toString()),
        new anchor.BN(sellerDeposit.toString()),
      )
      .accounts({
        user: seller.publicKey,
        usdcMint,
        market,
        userAta: sellerAta.address,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        userCollateral: sellerCollateral,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([seller])
      .rpc();

    const { buyerPosition, sellerPosition } = await settleSignedTrade({
      provider,
      vaultProgram,
      marketProgramId,
      vaultProgramId,
      market,
      marketVault,
      buyer,
      seller,
      buyerCollateral,
      sellerCollateral,
      marketId,
      quantity,
      sellerPriceScaled: priceScaled,
      buyerNonce: 3n,
      sellerNonce: 4n,
    });

    await marketProgram.methods
      .closeMarket()
      .accounts({
        admin: payer.publicKey,
        protocolConfig: marketProtocolConfig,
        market,
      })
      .rpc();

    await settlementProgram.methods
      .initMarketSettlement(new anchor.BN(marketId.toString()))
      .accounts({
        admin: payer.publicKey,
        protocolConfig: settlementProtocolConfig,
        market,
        marketVault,
        vaultAuthority: settlementAuthority,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const [resolution] = resolutionPda(settlementProgramId, marketId);
    await settlementProgram.methods
      .submitResolution(
        new anchor.BN(marketId.toString()),
        new anchor.BN(observedValue),
      )
      .accounts({
        resolver: payer.publicKey,
        protocolConfig: settlementProtocolConfig,
        market,
        vaultAuthority: settlementAuthority,
        resolution,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const [claimReceipt] = claimPda(settlementProgramId, marketId, buyer.publicKey, "long");
    await settlementProgram.methods
      .claimPayout(new anchor.BN(marketId.toString()))
      .accounts({
        user: buyer.publicKey,
        usdcMint,
        userAta: buyerAta.address,
        resolution,
        userPosition: buyerPosition,
        claimReceipt,
        marketVault,
        vaultAuthority: settlementAuthority,
        vaultTokenAuthority,
        vaultTokenAccount: vaultTokenAccount.address,
        vaultProgram: vaultProgramId,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    const receipt = await (settlementProgram.account as any).claimReceipt.fetch(claimReceipt);
    const vault = await (vaultProgram.account as any).marketVault.fetch(marketVault);
    const buyerTokenAccount = await getAccount(provider.connection, buyerAta.address);
    const vaultTokenAccountAfter = await getAccount(provider.connection, vaultTokenAccount.address);

    expect(receipt.claimed).to.equal(true);
    expect(BigInt(receipt.amount.toString())).to.equal(expectedPayout);
    expect(buyerTokenAccount.amount).to.equal(expectedPayout);
    expect(vaultTokenAccountAfter.amount).to.equal(
      buyerDeposit + sellerDeposit - expectedPayout,
    );
    expect(BigInt(vault.totalCollateral.toString())).to.equal(
      buyerDeposit + sellerDeposit - expectedPayout,
    );
  });

  it("refunds cancelled long cost basis through the settlement program and vault CPI", async () => {
    expect(payer).to.not.equal(undefined);

    const marketProgramId = new PublicKey(marketIdl.address);
    const vaultProgramId = new PublicKey(vaultIdl.address);
    const settlementProgramId = new PublicKey(settlementIdl.address);
    const buyer = Keypair.generate();
    const seller = Keypair.generate();
    const marketId = BigInt(Date.now() + 3);
    const buyerDeposit = 8_000_000n;
    const sellerDeposit = 8_000_000n;
    const quantity = 4_000_000n;
    const priceScaled = 650_000n;
    const expectedRefund = 2_600_000n;

    const payerAirdropSignature = await provider.connection.requestAirdrop(
      payer.publicKey,
      5 * LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(payerAirdropSignature, "confirmed");

    for (const trader of [buyer, seller]) {
      const signature = await provider.connection.requestAirdrop(
        trader.publicKey,
        LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(signature, "confirmed");
    }

    const usdcMint = await createMint(
      provider.connection,
      payer,
      payer.publicKey,
      null,
      6,
    );
    const buyerAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      buyer.publicKey,
    );
    const sellerAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      seller.publicKey,
    );

    await mintTo(
      provider.connection,
      payer,
      usdcMint,
      buyerAta.address,
      payer,
      buyerDeposit,
    );
    await mintTo(
      provider.connection,
      payer,
      usdcMint,
      sellerAta.address,
      payer,
      sellerDeposit,
    );

    const [marketProtocolConfig] = protocolPda(marketProgramId);
    const existingMarketProtocol = await (marketProgram.account as any).protocolConfig.fetchNullable(
      marketProtocolConfig,
    );
    if (!existingMarketProtocol) {
      await marketProgram.methods
        .initializeProtocol(50)
        .accounts({
          admin: payer.publicKey,
          protocolConfig: marketProtocolConfig,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    const [settlementProtocolConfig] = settlementProtocolPda(settlementProgramId);
    const existingSettlementProtocol = await (settlementProgram.account as any).protocolConfig.fetchNullable(
      settlementProtocolConfig,
    );
    if (!existingSettlementProtocol) {
      await settlementProgram.methods
        .initializeProtocol(payer.publicKey)
        .accounts({
          admin: payer.publicKey,
          protocolConfig: settlementProtocolConfig,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    const [market] = marketPda(marketProgramId, marketId);
    await marketProgram.methods
      .createMarket(
        new anchor.BN(marketId.toString()),
        new anchor.BN(100),
        new anchor.BN(200),
        new anchor.BN(Math.floor(Date.now() / 1000) + 3600),
        "integration-test",
        { linear: {} },
        0,
      )
      .accounts({
        admin: payer.publicKey,
        protocolConfig: marketProtocolConfig,
        market,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await marketProgram.methods
      .forceActivateMarket()
      .accounts({
        admin: payer.publicKey,
        protocolConfig: marketProtocolConfig,
        market,
      })
      .rpc();

    const [marketVault] = vaultPda(vaultProgramId, marketId);
    const [vaultTokenAuthority] = vaultAuthorityPda(vaultProgramId, marketId);
    const [settlementAuthority] = vaultAuthorityPda(settlementProgramId, marketId);
    const vaultTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      usdcMint,
      vaultTokenAuthority,
      true,
    );

    await vaultProgram.methods
      .initMarketVault(new anchor.BN(marketId.toString()), settlementAuthority)
      .accounts({
        payer: payer.publicKey,
        usdcMint,
        market,
        vaultAuthority: vaultTokenAuthority,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const [buyerCollateral] = collateralPda(vaultProgramId, marketId, buyer.publicKey);
    const [sellerCollateral] = collateralPda(vaultProgramId, marketId, seller.publicKey);

    await vaultProgram.methods
      .depositCollateral(
        new anchor.BN(marketId.toString()),
        new anchor.BN(buyerDeposit.toString()),
      )
      .accounts({
        user: buyer.publicKey,
        usdcMint,
        market,
        userAta: buyerAta.address,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        userCollateral: buyerCollateral,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    await vaultProgram.methods
      .depositCollateral(
        new anchor.BN(marketId.toString()),
        new anchor.BN(sellerDeposit.toString()),
      )
      .accounts({
        user: seller.publicKey,
        usdcMint,
        market,
        userAta: sellerAta.address,
        marketVault,
        vaultTokenAccount: vaultTokenAccount.address,
        userCollateral: sellerCollateral,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([seller])
      .rpc();

    const { buyerPosition, sellerPosition } = await settleSignedTrade({
      provider,
      vaultProgram,
      marketProgramId,
      vaultProgramId,
      market,
      marketVault,
      buyer,
      seller,
      buyerCollateral,
      sellerCollateral,
      marketId,
      quantity,
      sellerPriceScaled: priceScaled,
      buyerNonce: 5n,
      sellerNonce: 6n,
    });

    await marketProgram.methods
      .cancelMarket()
      .accounts({
        admin: payer.publicKey,
        protocolConfig: marketProtocolConfig,
        market,
      })
      .rpc();

    await settlementProgram.methods
      .initMarketSettlement(new anchor.BN(marketId.toString()))
      .accounts({
        admin: payer.publicKey,
        protocolConfig: settlementProtocolConfig,
        market,
        marketVault,
        vaultAuthority: settlementAuthority,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const [refundReceipt] = refundPda(settlementProgramId, marketId, buyer.publicKey, "long");
    await settlementProgram.methods
      .refund(new anchor.BN(marketId.toString()))
      .accounts({
        admin: payer.publicKey,
        protocolConfig: settlementProtocolConfig,
        market,
        usdcMint,
        userAta: buyerAta.address,
        userPosition: buyerPosition,
        refundReceipt,
        marketVault,
        vaultAuthority: settlementAuthority,
        vaultTokenAuthority,
        vaultTokenAccount: vaultTokenAccount.address,
        vaultProgram: vaultProgramId,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const receipt = await (settlementProgram.account as any).refundReceipt.fetch(refundReceipt);
    const vault = await (vaultProgram.account as any).marketVault.fetch(marketVault);
    const buyerTokenAccount = await getAccount(provider.connection, buyerAta.address);
    const vaultTokenAccountAfter = await getAccount(provider.connection, vaultTokenAccount.address);

    expect(receipt.refunded).to.equal(true);
    expect(BigInt(receipt.amount.toString())).to.equal(expectedRefund);
    expect(buyerTokenAccount.amount).to.equal(expectedRefund);
    expect(vaultTokenAccountAfter.amount).to.equal(
      buyerDeposit + sellerDeposit - expectedRefund,
    );
    expect(BigInt(vault.totalCollateral.toString())).to.equal(
      buyerDeposit + sellerDeposit - expectedRefund,
    );
  });
});
