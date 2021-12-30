/**
 * Clash exchange client to execute/test program transactions
 */

import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
  SYSVAR_RENT_PUBKEY
} from '@solana/web3.js'

import { TOKEN_PROGRAM_ID } from '@solana/spl-token';

import fs from 'mz/fs';

import {
  getRpcUrl,
  getPayer,
  createKeypairFromFile,
  findAssociatedTokenAddress,
  SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID,
  SYSTEM_PROGRAM_ID
} from './utils';

/**
 * Expected location for the deployed program keypair file
*/
const PROGRAM_KEYPAIR_PATH = "./dist/program/clash_exchange_program-keypair.json"

const PROGRAM_BPF_PATH = "./dist/program/clash_exchange_program.so"

/**
 * Deploy command to print as help info
*/
const DEPLOY_PROGRAM_CMD = "`solana program deploy dist/program/clash_exchange_program.so`"

/**
 * CLASH NFT account
*/
let CLASH_TOKEN_ACCOUNT:PublicKey;

/**
 * Connection to the network
 */
let connection: Connection;

/**
 * Deployed program ID
 */
let programId: PublicKey;

/**
 * Keypair associated to the fees' payer
 */
let payer: Keypair;

/**
 * Establish a connection to the cluster
 */
export async function establishConnection(): Promise<void> {
  const rpcUrl = await getRpcUrl();
  connection = new Connection(rpcUrl, 'confirmed');
  const version = await connection.getVersion();
  console.log('Connection to cluster established:', rpcUrl, version);
}

/**
 * Establish an account to pay for everything
 */
export async function establishPayer(): Promise<void> {
  let fees = 0;
  if (!payer) {
    const {feeCalculator} = await connection.getRecentBlockhash();

    // Calculate the cost to fund the greeter account
    //fees += await connection.getMinimumBalanceForRentExemption(GREETING_SIZE);

    // Calculate the cost of sending transactions
    fees += feeCalculator.lamportsPerSignature * 100; // wag

    payer = await getPayer();
  }

  let lamports = await connection.getBalance(payer.publicKey);
  if (lamports < fees) {
    // If current balance is not enough to pay for fees, request an airdrop
    const sig = await connection.requestAirdrop(
      payer.publicKey,
      fees - lamports,
    );
    await connection.confirmTransaction(sig);
    lamports = await connection.getBalance(payer.publicKey);
  }

  console.log(
    'Using account',
    payer.publicKey.toBase58(),
    'containing',
    lamports / LAMPORTS_PER_SOL,
    'SOL to pay for fees',
  );
}

export async function getCurrentPayer() : Promise<Keypair> {
  return payer;
}

async function getProgramPDA() : Promise<[PublicKey, number]> {
    let [programPDA, seed] = await PublicKey.findProgramAddress([
      Buffer.from("genezys-fin", 'utf8'),
      Buffer.from("clash-ico", 'utf8')
    ], programId);

    return [programPDA, seed]
}

/**
 * Check if BPF program has been deployed
 */
export async function checkProgram(): Promise<void> {
  // Check for a valid keypar for deployed program
  try {
    const programKeyPar = await createKeypairFromFile(PROGRAM_KEYPAIR_PATH);
    programId = programKeyPar.publicKey;
  } catch (err) {
    const errMsg = (err as Error).message;
    throw new Error(
      `Failed to read program keypair at '${PROGRAM_KEYPAIR_PATH}' due to error:\n    ${errMsg}.\n\nProgram may need to be deployed with ${DEPLOY_PROGRAM_CMD}`,
    );
  }

  // Check if the program has been deployed to cluster
  const programInfo = await connection.getAccountInfo(programId);
  if (programInfo === null) {
    throw new Error(`Program needs to be built with 'npm run build:program-rust' and deploy with ${DEPLOY_PROGRAM_CMD}`);
  } else if (!programInfo.executable) {
    throw new Error('Program is not marked as executable');
  }

  if (!fs.existsSync(PROGRAM_BPF_PATH)) {
    console.log(`[Warning] Program is deployed but executable is missing. Run \`npm run build:program-rust\` to built it.`);
  }

  console.log(`Deployed program id is ${programId.toBase58()}.`);
}

export async function loadTokenId(): Promise<boolean> {
  const tokenFile = "token_id.json";

  if (!await fs.exists(tokenFile)) {
    return false;
  }

  let tokenFileContent = await fs.readFile(tokenFile, {encoding: 'utf8'});
  let tokenInfo = JSON.parse(tokenFileContent);

  CLASH_TOKEN_ACCOUNT = new PublicKey(tokenInfo.token_id);
  return CLASH_TOKEN_ACCOUNT != undefined;
}

export async function initializeICO(clashAuthorityInfo: CLASHAuthorityInfo, initializer:Keypair) : Promise<void> {
  console.log(`Initializing ICO program.`)

  let [programPDA, seed] = await getProgramPDA();

  clashAuthorityInfo.ATAWallet = await findAssociatedTokenAddress(programPDA, CLASH_TOKEN_ACCOUNT);

  console.log(CLASH_TOKEN_ACCOUNT.toString(), clashAuthorityInfo.ATAWallet.toString());

  let initializerATA = await findAssociatedTokenAddress(initializer.publicKey, CLASH_TOKEN_ACCOUNT);

  let programData = Buffer.alloc(1);
  programData.writeUInt8(0); // at 0: Instruction type

  const instruction = new TransactionInstruction({
    keys: [
      // Clash authority accounts
      {pubkey: initializer.publicKey, isSigner: true, isWritable: false},
      {pubkey: initializerATA, isSigner: false, isWritable: false},

      // Token account
      {pubkey: CLASH_TOKEN_ACCOUNT, isSigner: false, isWritable: true},

      // Program PDA to sign
      {pubkey: programPDA, isSigner: false, isWritable: true},

      // Program PDA associated token account
      {pubkey: clashAuthorityInfo.ATAWallet, isSigner: false, isWritable: true},

      // Native system and token programs accounts
      {pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false}
    ],
    programId,
    data: programData
  });

  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [initializer]
  );

  console.log("Successfuly initialized ICO! Send tokens to program associated token account to allow it to exchange them.");
  console.log("    Program ATA: ", clashAuthorityInfo.ATAWallet.toString());
}

export async function exchangeSOLByCLASH(clashAuthorityInfo: CLASHAuthorityInfo, exchangerInfo: ExchangerInfo, exchangeInfo: ExchangeSOLByCLASHInfo
): Promise<void> {

  console.log(`Preparing to exchange SOL's by Clash tokens.`)

  let programData = Buffer.alloc(9);
  programData.writeUInt8(1); // at 0: Instruction type
  programData.writeBigUInt64LE(BigInt(exchangeInfo.SOLAmount * LAMPORTS_PER_SOL), 1); // at 1: SOL amount

  let [programPDA, seed] = await getProgramPDA();

  clashAuthorityInfo.ATAWallet = await findAssociatedTokenAddress(programPDA, CLASH_TOKEN_ACCOUNT);

  const instruction = new TransactionInstruction({
    keys: [
      // User accounts
      {pubkey: exchangerInfo.SOLWallet.publicKey, isSigner: true, isWritable: true},
      {pubkey: exchangerInfo.ATAWallet, isSigner: false, isWritable: true},

      {pubkey: clashAuthorityInfo.SOLWallet, isSigner: false, isWritable: true},
      {pubkey: clashAuthorityInfo.ATAWallet, isSigner: false, isWritable: true},

      // Token account
      {pubkey: CLASH_TOKEN_ACCOUNT, isSigner: false, isWritable: false},

      // Program account and PDA to sign
      {pubkey: programId, isSigner: false, isWritable: false},
      {pubkey: programPDA, isSigner: false, isWritable: false},

      // Native system and token programs accounts
      {pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false},
    ],
    programId,
    data: programData
  });

  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [exchangerInfo.SOLWallet],
  );
}

export async function confirmCLASHPayment(clashAuthorityInfo: CLASHAuthorityInfo, exchangerInfo: ExchangerInfo, paymentInfo: CLASHPaymentInfo
): Promise<void> {
  console.log(`Deliverying buyed CLASH tokens.`)

  let [programPDA, seed] = await getProgramPDA();

  clashAuthorityInfo.ATAWallet = await findAssociatedTokenAddress(programPDA, CLASH_TOKEN_ACCOUNT);

  let programData = Buffer.alloc(9);
  programData.writeUInt8(2); // at 0: Instruction type
  programData.writeBigUInt64LE(BigInt(paymentInfo.CLASHAmount * LAMPORTS_PER_SOL), 1); // at 1: Payed amount

  let clashAuthority = payer;

  const instruction = new TransactionInstruction({
    keys: [
      // User accounts
      {pubkey: exchangerInfo.SOLWallet.publicKey, isSigner: false, isWritable: false},
      {pubkey: exchangerInfo.ATAWallet, isSigner: false, isWritable: true},

      // Token account
      {pubkey: CLASH_TOKEN_ACCOUNT, isSigner: false, isWritable: false},

      // Clash authority accounts
      {pubkey: clashAuthority.publicKey, isSigner: true, isWritable: false},
      {pubkey: clashAuthorityInfo.ATAWallet, isSigner: false, isWritable: true},

      // Program account and PDA to sign
      {pubkey: programId, isSigner: false, isWritable: false},
      {pubkey: programPDA, isSigner: false, isWritable: false},

      // Native system and token programs accounts
      {pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false},
    ],
    data: programData,
    programId
  });

  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [clashAuthority],
  );
}

type CLASHAuthorityInfo = {
  SOLWallet: PublicKey,
  ATAWallet: PublicKey
};

export async function terminateICO(clashAuthorityInfo: CLASHAuthorityInfo, initializer:Keypair) : Promise<void> {
  console.log(`Terminating the ICO program.`)

  let [programPDA, seed] = await getProgramPDA();

  clashAuthorityInfo.ATAWallet = await findAssociatedTokenAddress(programPDA, CLASH_TOKEN_ACCOUNT);

  let initializerATA = await findAssociatedTokenAddress(initializer.publicKey, CLASH_TOKEN_ACCOUNT);

  let programData = Buffer.alloc(1);
  programData.writeUInt8(3); // at 0: Instruction type

  const instruction = new TransactionInstruction({
    keys: [
      // Clash authority accounts
      {pubkey: initializer.publicKey, isSigner: true, isWritable: false},
      {pubkey: initializerATA, isSigner: false, isWritable: true},

      // Token account
      {pubkey: CLASH_TOKEN_ACCOUNT, isSigner: false, isWritable: true},

      // Program PDA to sign
      {pubkey: programPDA, isSigner: false, isWritable: true},

      // Program PDA associated token account
      {pubkey: clashAuthorityInfo.ATAWallet, isSigner: false, isWritable: true},

      // Native system and token programs accounts
      {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false}
    ],
    programId,
    data: programData
  });

  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [initializer]
  );
}

export async function getCLASHAuthorityInfo(): Promise<CLASHAuthorityInfo> {
  return {
    SOLWallet: new PublicKey("DMxXQTaLqGD82GUKPFT7j7zJYNecRwCWkegYHVcM1DTy"),
    ATAWallet: new PublicKey("GWWE2qvve6Xe1eodVFhf7mZ61hEKJkUFotQNZHe7HfHQ")
  }
}

type ExchangerInfo = {
  SOLWallet: Keypair,
  ATAWallet: PublicKey
};

export async function getExchangerInfo(exchagerKeypair: Keypair, exchangerATAPubkey: PublicKey) : Promise<ExchangerInfo> {
  return {
    SOLWallet: exchagerKeypair,
    ATAWallet: exchangerATAPubkey
  }
}

export async function loadExchangerInfoFromFile(walletFilePath: string) : Promise<ExchangerInfo> {
  let walletKeypair = await createKeypairFromFile(walletFilePath);
  let ATAWallet = await findAssociatedTokenAddress(walletKeypair.publicKey, CLASH_TOKEN_ACCOUNT);

  return await getExchangerInfo(walletKeypair, ATAWallet);
}

type ExchangeSOLByCLASHInfo = {
  SOLAmount: number,
};

type CLASHPaymentInfo = {
  CLASHAmount: number,
};
