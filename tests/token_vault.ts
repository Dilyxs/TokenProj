import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TokenExample } from "../target/types/token_example";
import {
  getAssociatedTokenAddress,
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { Keypair, PublicKey } from "@solana/web3.js";
import { expect } from "chai";

describe("token_vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const oneDaySeconds = 60 * 60 * 24;
  const program = anchor.workspace.TokenExample as Program<TokenExample>;
  const owner = provider.wallet;
  const stranger = Keypair.generate();

  const tokenProgram = new PublicKey(TOKEN_2022_PROGRAM_ID);
  const associatedTokenProgram = new PublicKey(ASSOCIATED_TOKEN_PROGRAM_ID);
  const systemProgram: PublicKey = anchor.web3.SystemProgram.programId;

  const [configPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  );
  const [vaultAuthorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("authority")],
    program.programId
  );
  const [mintPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("adsayan_mint")],
    program.programId
  );
  const [depositInfoPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("deposit_info"), owner.publicKey.toBuffer()],
    program.programId
  );

  let vaultAta: PublicKey;
  let userAta: PublicKey;
  before(async () => {
    vaultAta = await getAssociatedTokenAddress(
      mintPda,
      vaultAuthorityPda,
      true,
      TOKEN_2022_PROGRAM_ID
    );
    const sig = await provider.connection.requestAirdrop(
      stranger.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    const latestBlock = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      blockhash: latestBlock.blockhash,
      lastValidBlockHeight: latestBlock.lastValidBlockHeight,
      signature: sig,
    });

    userAta = PublicKey.findProgramAddressSync(
      [
        owner.publicKey.toBuffer(),
        TOKEN_2022_PROGRAM_ID.toBuffer(),
        mintPda.toBuffer(),
      ],
      ASSOCIATED_TOKEN_PROGRAM_ID
    )[0];
  });

  it("initialize mint, vault, config", async () => {
    await program.methods
      .initializeTokenSubscription(
        new anchor.BN(1000000),
        new anchor.BN(oneDaySeconds * 30)
      )
      .accounts({
        owner: owner.publicKey,
        config: configPda,
        vaultAuthority: vaultAuthorityPda,
        mint: mintPda,
        vaultAta: vaultAta,
        systemProgram: systemProgram,
        tokenProgram: tokenProgram,
        associatedTokenProgram: associatedTokenProgram,
      })
      .rpc();
    const config = await program.account.configOwner.fetch(configPda);
    expect(config.admin.toBase58()).to.equal(owner.publicKey.toBase58());
    expect(config.price.toNumber()).to.equal(1000000);
    expect(config.duration.toNumber()).to.equal(oneDaySeconds * 30);
  });
  //now check if we can get somes Tokens
  it("get some adsayan tokens", async () => {
    await program.methods
      .mintToUser(new anchor.BN(1000000))
      .accounts({
        owner: owner.publicKey,
        vaultAuthority: vaultAuthorityPda,
        mint: mintPda,
        userTokenAccount: userAta,
        systemProgram: systemProgram,
        tokenProgram: tokenProgram,
        associatedTokenProgram: associatedTokenProgram,
      })
      .rpc();
    //check that we did get the tokens
    const balance = await provider.connection.getTokenAccountBalance(userAta);
    expect(Number(balance.value.amount)).to.equal(1000000);
  });
  //we should have exactly enough to buy a subscription
  it("enough for one subscription", async () => {
    const [subscriptionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("subscription"), owner.publicKey.toBuffer()],
      program.programId
    );
    //event listener
    const listener = program.addEventListener(
      "succesfullSubscription",
      (event) => {
        if ("message" in event) {
          expect(event.message).to.equals("success");
        } else {
          expect(false).to.equals(true);
        }
      }
    );
    await program.methods
      .subscribeToVault()
      .accounts({
        owner: owner.publicKey,
        mint: mintPda,
        userAta: userAta,
        vaultAuthority: vaultAuthorityPda,
        vaultAta: vaultAta,
        config: configPda,
        subscription: subscriptionPda,
        systemProgram: systemProgram,
        tokenProgram: tokenProgram,
        associatedTokenProgram: associatedTokenProgram,
      })
      .rpc();

    await new Promise((resolve) => setTimeout(resolve, 1000));
    await program.removeEventListener(listener);
  });

  it("check if subscription_valid", async () => {
    const [subscriptionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("subscription"), owner.publicKey.toBuffer()],
      program.programId
    );

    const listener = program.addEventListener(
      "isValidSubscription",
      (event) => {
        if ("isValid" in event) {
          expect(event.isValid).to.equals(true);
        } else {
          expect(false).to.equals(true);
        }
      }
    );

    await program.methods
      .isUserSubcribed()
      .accounts({
        owner: owner.publicKey,
        mint: mintPda,
        userAcc: subscriptionPda,
      })
      .rpc();

    await new Promise((resolve) => setTimeout(resolve, 1000));
    await program.removeEventListener(listener);
  });

  it("deposit tokens into vault", async () => {
    await program.methods
      .mintToUser(new anchor.BN(2000000))
      .accounts({
        owner: owner.publicKey,
        vaultAuthority: vaultAuthorityPda,
        mint: mintPda,
        userTokenAccount: userAta,
        systemProgram: systemProgram,
        tokenProgram: tokenProgram,
        associatedTokenProgram: associatedTokenProgram,
      })
      .rpc();

    await program.methods
      .deposit(new anchor.BN(1000000))
      .accounts({
        owner: owner.publicKey,
        vaultAuthority: vaultAuthorityPda,
        mint: mintPda,
        userTokenAcc: userAta,
        vaultAcc: vaultAta,
        data: depositInfoPda,
        systemProgram: systemProgram,
        tokenProgram: tokenProgram,
        associatedTokenProgram: associatedTokenProgram,
      })
      .rpc();

    const depositInfo = await program.account.depositeToken.fetch(
      depositInfoPda
    );
    expect(Number(depositInfo.quantity)).to.equal(1000000);
  });

  it("withdraw tokens from vault (success)", async () => {
    await program.methods
      .withdraw(new anchor.BN(500000))
      .accounts({
        owner: owner.publicKey,
        vaultAuthority: vaultAuthorityPda,
        mint: mintPda,
        ownerAta: userAta,
        vaultAta: vaultAta,
        bookeepingAcc: depositInfoPda,
        systemProgram: systemProgram,
        tokenProgram: tokenProgram,
        associatedTokenProgram: associatedTokenProgram,
      })
      .rpc();

    const depositInfo = await program.account.depositeToken.fetch(
      depositInfoPda
    );
    expect(Number(depositInfo.quantity)).to.equal(500000);
  });

  it("withdraw tokens from vault (should fail)", async () => {
    try {
      await program.methods
        .withdraw(new anchor.BN(700000))
        .accounts({
          owner: owner.publicKey,
          vaultAuthority: vaultAuthorityPda,
          mint: mintPda,
          ownerAta: userAta,
          vaultAta: vaultAta,
          bookeepingAcc: depositInfoPda,
          systemProgram: systemProgram,
          tokenProgram: tokenProgram,
          associatedTokenProgram: associatedTokenProgram,
        })
        .rpc();

      expect.fail("expected withdraw to fail with not enough funds");
    } catch (err) {
      const msg = (err as Error).message;
      expect(msg).to.include("not enough founds");
    }
  });
});
