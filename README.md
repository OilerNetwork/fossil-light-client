<readme>

# Fossil Light Client Local Testing Setup

This README provides instructions for setting up and running the `fossil-light-client` for local testing. These steps will guide you through configuring a simulated Ethereum environment (Anvil), deploying necessary contracts, and initializing a local Starknet development network (Katana) for integrated testing. Each section corresponds to a separate terminal session to keep services organized and running simultaneously.

## Terminal 1: Start Anvil Ethereum Devnet

In this terminal, you'll set up an Ethereum development environment using Anvil, which will simulate an Ethereum network locally.

1. Load the environment variables:
   ```bash
   source .env
   ```

2. Start the Anvil Ethereum development network:
   ```bash
   anvil --fork-url $ETH_RPC_URL --block-time 12
   ```

> **Note:** `${ETH_RPC_URL}` should be configured in `anvil.env` to point to the desired RPC provider (e.g., Infura or Alchemy) for forking mainnet data.

## Terminal 2: Start Katana Starknet Devnet and Deploy Contracts

In this terminal, you'll initialize Katana, a local Starknet development environment. Katana will work in tandem with Anvil for cross-chain interactions in your testing setup.

1. Source the environment variables:
   ```bash
   source config/katana.env
   ```
2. Update the `anvil.messaging.json` file with the correct values for `from_block` taken from the Anvil logs.
   ```
   Fork
   ==================
   Endpoint:       http://xxx.x.x.x:x
   Block number:   21168847 <---
   Block hash:     0x67bc863205b5cd53f11d78bccb7a722db1b598bb24f4e11239598825bfb3e4d3
   Chain ID:       1
   ```

3. Start Katana with messaging integration for Anvil:
   ```bash
   katana --messaging config/anvil.messaging.json --disable-fee --disable-validate
   ```

> **Note:** `--messaging` enables communication between Anvil and Katana, and `--disable-fee` allows for testing without transaction fees.


## Terminal 3: Deploy L1MessageSender.sol

In this terminal, you will deploy the `L1MessageSender.sol` contract to the Anvil development network, which is essential for message relaying between Ethereum and Starknet in this testing setup.

1. Load the environment variables:
   ```bash
   source .env
   ```

2. Navigate to the Ethereum directory:
   ```bash
   cd contracts/ethereum
   ```

3. Deploy the contract:
   ```bash
   forge script script/LocalTesting.s.sol:LocalSetup --broadcast --rpc-url $ANVIL_URL
   ```

> **Note:** This deployment requires `forge` and should be configured to point to the `ANVIL_URL` as specified in `anvil.env`.

   Now deploy all necessary Starknet contracts to the Katana development network.

1. Navigate to the Starknet deployment script directory:
   ```bash
   cd ../../scripts/katana/
   ```

2. Run the deployment script:
   ```bash
   ./deploy.sh
   ```

> **Note:** Ensure the `deploy.sh` script is configured correctly to deploy the required contracts for testing on Katana.
>

## Terminal 4: Run The Light Client

1. Navigate to the Light Client directory:
   ```bash
   cd crates/client
   ```

2. Start the Light Client:
   ```bash
   cargo run
   ```

## Back to Terminal 3: Trigger the Relayer to Send Finalized Block Hash to L2

3. Send the finalized block hash from the Ethereum network to the Starknet network.

1. Navigate to the Ethereum directory:
   ```bash
   cd ../../crates/relayer
   ```

2. Start the Relayer and send the finalized block hash to the Starknet network:
   ```bash
   cargo run
   ```
</readme>