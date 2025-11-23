import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AmmAnchor } from "../target/types/amm_anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  getAssociatedTokenAddress,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";

describe("amm-anchor", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.env();
  const program = anchor.workspace.AmmAnchor as Program<AmmAnchor>;
  const user = provider.wallet.publicKey;

  let mintX: anchor.web3.PublicKey;
  let mintY: anchor.web3.PublicKey;
  let vaultX: anchor.web3.PublicKey;
  let vaultY: anchor.web3.PublicKey;
  let userxATA: anchor.web3.PublicKey;
  let useryATA: anchor.web3.PublicKey;
  let mintLp: anchor.web3.PublicKey;
  let userLpATA: anchor.web3.PublicKey;
  let state: anchor.web3.PublicKey;

  const fee = 30;
  const initialAmount = 1_000_000_000;
  const seed = new anchor.BN(2002);
  const decimals = BigInt(1_000_000);

  before("setting up state", async () => {
    await provider.connection.requestAirdrop(user, 10 * anchor.web3.LAMPORTS_PER_SOL);

    mintX = await createMint(provider.connection, provider.wallet.payer, user, null, 6);
    mintY = await createMint(provider.connection, provider.wallet.payer, user, null, 6);

    const userXATA = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      mintX,
      user
    );
    userxATA = userXATA.address;

    const userYATA = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      mintY,
      user
    );
    useryATA = userYATA.address;

    await mintTo(
      provider.connection,
      provider.wallet.payer,
      mintX,
      userxATA,
      provider.wallet.payer,
      initialAmount
    );

    await mintTo(
      provider.connection,
      provider.wallet.payer,
      mintY,
      useryATA,
      provider.wallet.payer,
      initialAmount
    );

    const seedBuf = seed.toArrayLike(Buffer, "le", 8);

    [state] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("amm-state"), mintX.toBuffer(), mintY.toBuffer(), seedBuf],
      program.programId
    );

    [mintLp] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("amm-lp-mint"), state.toBuffer()],
      program.programId
    );

    vaultX = await getAssociatedTokenAddress(mintX, state, true);
    vaultY = await getAssociatedTokenAddress(mintY, state, true);

    console.log("Mint X:", mintX.toBase58());
    console.log("Mint Y:", mintY.toBase58());
    console.log("User X ATA:", userxATA.toBase58());
    console.log("User Y ATA:", useryATA.toBase58());
    console.log("Vault X:", vaultX.toBase58());
    console.log("Vault Y:", vaultY.toBase58());
    console.log("LP Mint:", mintLp.toBase58());
    console.log("State PDA:", state.toBase58());

  });

  it("Initializing the Amm pool", async () => {
    const tx = await program.methods
      .initialize(seed, fee)
      .accountsStrict({
        user: user,
        state: state,
        mintX: mintX,
        mintY: mintY,
        mintLp: mintLp,
        vaultX: vaultX,
        vaultY: vaultY,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();


    const stateAccount = await program.account.ammState.fetch(state);
    assert.equal(stateAccount.seed.toString(), seed.toString());
    assert.equal(stateAccount.isLocked, false);
    assert.equal(stateAccount.fee, fee);
    assert.equal(stateAccount.mintX.toBase58(), mintX.toBase58());
    assert.equal(stateAccount.mintY.toBase58(), mintY.toBase58());
    assert.equal(stateAccount.mintLp.toBase58(), mintLp.toBase58());

  });

  it("Initial Deposit into the pool for liquidity", async () => {
    userLpATA = (await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      mintLp,
      user
    )).address;

    const depositMaxAmountX = 500_000_000;
    const depositMaxAmountY = 500_000_000;

    const tx = await program.methods
      .deposit(new anchor.BN(1_000_000), new anchor.BN(depositMaxAmountX), new anchor.BN(depositMaxAmountY))
      .accountsStrict({
        user: user,
        mintX: mintX,
        mintY: mintY,
        state: state,
        vaultX: vaultX,
        vaultY: vaultY,
        userAtaX: userxATA,
        userAtaY: useryATA,
        mintLp: mintLp,
        userAtaLp: userLpATA,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const vaultXAccount = await getAccount(provider.connection, vaultX);
    const vaultYAccount = await getAccount(provider.connection, vaultY);
    const userLpAccount = await getAccount(provider.connection, userLpATA);

    assert.equal(vaultXAccount.amount, BigInt(500_000_000));
    assert.equal(vaultYAccount.amount, BigInt(500_000_000));
    assert.equal(userLpAccount.amount, BigInt(1_000_000));

  });

  it("swap X for Y", async () => {
    const swapAmountX = 100_000_000;
    const tx = await program.methods
      .swap(true, new anchor.BN(swapAmountX), new anchor.BN(0))
      .accountsStrict({
        user: user,
        state: state,
        mintX: mintX,
        mintY: mintY,
        vaultX: vaultX,
        vaultY: vaultY,
        userAtaX: userxATA,
        userAtaY: useryATA,
        mintLp: mintLp,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultXAccount = await getAccount(provider.connection, vaultX);
    const vaultYAccount = await getAccount(provider.connection, vaultY);
    const userXAccount = await getAccount(provider.connection, userxATA);
    const userYAccount = await getAccount(provider.connection, useryATA);

  });

  it("withdraw liquidity from the pool", async () => {
    const withdrawLpAmount = 500_000;

    const tx = await program.methods
      .withdraw(new anchor.BN(withdrawLpAmount), new anchor.BN(0), new anchor.BN(0))
      .accountsStrict({
        user: user,
        state: state,
        mintX: mintX,
        mintY: mintY,
        vaultX: vaultX,
        vaultY: vaultY,
        userAtaX: userxATA,
        userAtaY: useryATA,
        mintLp: mintLp,
        userAtaLp: userLpATA,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const vaultXAccount = await getAccount(provider.connection, vaultX);
    const vaultYAccount = await getAccount(provider.connection, vaultY);
    const userXAccount = await getAccount(provider.connection, userxATA);
    const userYAccount = await getAccount(provider.connection, useryATA);
    const userLpAccount = await getAccount(provider.connection, userLpATA);

  });
});
