/**
 * Clash exchange client to execute/test program transactions
 */

import {
  establishConnection,
  establishPayer,
  checkProgram,
  exchangeSOLByCLASH,
  getCLASHAuthorityInfo,
  loadExchangerInfoFromFile,
  initializeICO,
  getCurrentPayer,
  terminateICO,
  confirmCLASHPayment,
  loadTokenId
} from './program';

import fs from 'mz/fs';

async function main(argv:any) {
  let command = argv._[0];

  if (command === "config") {
    let configFile = (argv._[1] != undefined) ? argv._[1] : "config.json";
    await updateProgramConfig(configFile);
    return;
  }

  // Connects to the cluster
  await establishConnection();

  // Determines the account for paying transactions fees
  await establishPayer();

  // Check for a valid deployed program
  await checkProgram();

  // Loads configured token ID
  if (!await loadTokenId() && command !== "config") {
    console.log("Run the command `npm run config` and configure a valid token ID first.");
    return;
  }

  // Load data to use
  let clashAuthorityInfo = await getCLASHAuthorityInfo();
  let exchangerInfo = await loadExchangerInfoFromFile("./dist/static_wallet.json");

  if (command == undefined) {
    throw "Undefined program command";
  }

  if (command ==="init") {
    await initializeICO(clashAuthorityInfo, await getCurrentPayer());
  }
  else if (command === "terminate") {
    await terminateICO(clashAuthorityInfo, await getCurrentPayer());
  }
  else if (command === "exchange") {
    let amount = (argv._[1] != undefined) ? parseFloat(argv._[1]) : 0.35;
    await exchangeSOLByCLASH(clashAuthorityInfo, exchangerInfo, {SOLAmount: amount});
  }
  else if (command === "confirm") {
    let amount = (argv._[1] != undefined) ? parseFloat(argv._[1]) : 0.35;
    await confirmCLASHPayment(clashAuthorityInfo, exchangerInfo, {CLASHAmount: amount});
  } else {
    throw("Invalid command `" + command + "`.");
  }
}

async function updateProgramConfig(configPath:string): Promise<void> {
  console.log("Configuring program with values from file: " + configPath);

  let configSample = `{
  "target_file": "program-rust/src/config.rs.dist",
  "output_file": "program-rust/src/config.rs",
  "clash_team_sol_wallet": "<clash authority wallet address to receive SOL>",
  "clash_token_id": "<clash token address on the Solana blockchain>",
  "clash_payment_authority": "<authority key to sign a payment realized by coinpayment>",
  "ico_freeze_duration_days": 30,
  "clash_usd_price": 0.035,
  "min_usd_price": "1.0",
  "max_usd_price": "10000.0"
}`

  if (!await fs.exists(configPath)) {
    await fs.writeFile(configPath, configSample, {encoding: 'utf8'});

    console.log("Config file '" + configPath + "' not found! A sample was created, edit this file and run the command again.");
    return;
  }

  const fileString = await fs.readFile(configPath, {encoding: 'utf8'});
  const config = JSON.parse(fileString);

  const targetContent = await fs.readFile(config.target_file, {encoding: 'utf8'});

  // Replace by pattern matching
  let modifiedContent = targetContent.replace("#CLASH_SOL_WALLET", config.clash_team_sol_wallet);
  modifiedContent = modifiedContent.replace("#CLASH_TOKEN_ID", config.clash_token_id);
  modifiedContent = modifiedContent.replace("#CLASH_PAYMENT_AUTHORITY", config.clash_payment_authority);
  modifiedContent = modifiedContent.replace("#CLASH_USD", config.clash_usd_price);
  modifiedContent = modifiedContent.replace("#MIN_USD", config.min_usd_price);
  modifiedContent = modifiedContent.replace("#MAX_USD", config.max_usd_price);

  const axios = require('axios');
  const SOLtoUSDQuotationURI = "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd";

  await axios.get(SOLtoUSDQuotationURI)
  .then((response:any) => {
    console.log("Current SOL/USD price: ", response.data.solana.usd);
    modifiedContent = modifiedContent.replace("#SOL_USD", response.data.solana.usd);
  })
  .catch((error:any) => {
    console.log(error);
  });

  if (config.clash_token_id != undefined) {
    let token_id_content = `{
  "token_id": "TOKEN_ID"
}`.replace("TOKEN_ID", config.clash_token_id);

    await fs.writeFile("token_id.json", token_id_content);
  };

  await fs.writeFile(config.output_file, modifiedContent, {encoding: 'utf8'});
  console.log("Generated updated configuration file: " + config.output_file);
}

var argv = require('minimist')(process.argv.slice(2));

main(argv).then(
  () => process.exit(),
  err => {
    console.error(err);
    process.exit(-1);
  }
);