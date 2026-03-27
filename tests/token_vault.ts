import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { TokenExample } from "../target/types/token_example";

const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey(
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);
const TOKEN_2022_PROGRAM_ID = new PublicKey(
  "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
);

const findPda = (seeds: Buffer[], programId: PublicKey): PublicKey =>
  PublicKey.findProgramAddressSync(seeds, programId)[0];

const findAta = (owner: PublicKey, mint: PublicKey): PublicKey =>
  PublicKey.findProgramAddressSync(
    [owner.toBuffer(), TOKEN_2022_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    ASSOCIATED_TOKEN_PROGRAM_ID
  )[0];

describe("token_vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.tokenExample as Program<TokenExample>;
  const owner = provider.wallet;

  const configPda = findPda([Buffer.from("config")], program.programId);
  const vaultAuthorityPda = findPda(
    [Buffer.from("authority")],
    program.programId
  );
  const mintPda = findPda([Buffer.from("adsayan_mint")], program.programId);
  const vaultAta = findAta(vaultAuthorityPda, mintPda);

  const subscriptionPrice = new anchor.BN(1_000_000);
  const duration = new anchor.BN(60);
  const updatedPrice = new anchor.BN(2_500_000);
  let stranger: Keypair;

  before("airdrop non-admin signer", async () => {
    stranger = Keypair.generate();
    const sig = await provider.connection.requestAirdrop(
      stranger.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig, "confirmed");
  });

  it("initializes subscription config", async () => {
    await program.methods
      .initializeTokenSubscription(subscriptionPrice, duration)
      .accountsPartial({
        owner: owner.publicKey,
        config: configPda,
        vaultAuthority: vaultAuthorityPda,
        mint: mintPda,
        vaultAta,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    const config = await program.account.configOwner.fetch(configPda);
    expect(config.admin.toBase58()).to.eq(owner.publicKey.toBase58());
    expect(config.price.toString()).to.eq(subscriptionPrice.toString());
    expect(config.duration.toString()).to.eq(duration.toString());
    expect(config.isPaused).to.eq(false);
  });

  it("allows admin to update price", async () => {
    await program.methods
      .setPrice(updatedPrice)
      .accountsPartial({
        admin: owner.publicKey,
        vaultAuthority: vaultAuthorityPda,
        mint: mintPda,
        config: configPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const config = await program.account.configOwner.fetch(configPda);
    expect(config.price.toString()).to.eq(updatedPrice.toString());
  });

  it("rejects non-admin price update", async () => {
    try {
      await program.methods
        .setPrice(new anchor.BN(9_999_999))
        .accountsPartial({
          admin: stranger.publicKey,
          vaultAuthority: vaultAuthorityPda,
          mint: mintPda,
          config: configPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([stranger])
        .rpc();
      expect.fail("setPrice should fail for non-admin signer");
    } catch (err: any) {
      const msg = String(err);
      const hasExpectedError =
        msg.includes("has one") ||
        msg.includes("ConstraintHasOne") ||
        msg.includes("A has one constraint was violated");
      expect(hasExpectedError).to.eq(true);
    }
  });

  it("fails to subscribe without minted user funds", async () => {
    const userAta = findAta(owner.publicKey, mintPda);
    const subscriptionPda = findPda(
      [Buffer.from("subscription"), owner.publicKey.toBuffer()],
      program.programId
    );

    try {
      await program.methods
        .subscribeToVault()
        .accountsPartial({
          owner: owner.publicKey,
          mint: mintPda,
          userAta,
          vaultAuthority: vaultAuthorityPda,
          vaultAta,
          config: configPda,
          subcription: subscriptionPda,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        })
        .rpc();
      expect.fail("subscribeToVault should fail when user ATA has no tokens");
    } catch (err: any) {
      expect(String(err).toLowerCase()).to.include("insufficient");
    }
  });
});
