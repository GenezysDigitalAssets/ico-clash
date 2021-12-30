# Genezys Clash ICO Solana SC

This repository holds an implementation of a smart contract to perform ICO for Clash coin through Solana blockchain.



## Dependencies

After cloning the first thing to do is to install the dependencies necessary to run the client code. You can do this running the NPM install comand from the repository root folder where the `package.json` file is located.

```shell
$: npm install
```

If everything went well you should see something like
```shell
..
added 92 packages, and audited 93 packages in 6s
..
```



## Configure

After cloning the first thing to do is to generate a `config.json` file that will be consumed to generate a source file containing hard-coded constant values to be used on the SC.

```shell
$: npm run config
```

This command will create a `config.json` file where you should configure the SC with the coin ID and the wallet that will receive the Solanas exchanged by the Clash coins.

After configuring the file just run it again and it will show on the output that the SC was configured and also the current SOL/USD quotation used.



## Build

Before building you should generate a `config.rs` file through the configuration process, so make sure to follow steps bellow. If already done you can proceed with:

```shell
$: npm run build
```

If everything went well it will outputs a program ID which will be your deployed program ID, store that for later.



## Deploy

Before deploying you should have built the SC with steps above, if so then proceed by selecting where you want to deploy the program:

`target_net`: (_mainnet-beta_ | _testnet_ | _devnet_ | _localhost_)

```shell
$: solana config set --url <target_net>
```

Then run the deploy command that will do the rest and return the deployed program ID:

```shell
$: npm run deploy
```



## Test

For testing one can use `npm run start -- test` or `npm run start -- exchange 0.5`.  For exchanging make sure to airdrop some Solana native tokens at `dist/static_wallet.json` that will be used.