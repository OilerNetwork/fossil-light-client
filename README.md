# Fossil Light Client Local Testing Setup

This README outlines the technical configuration for deploying and testing the `fossil-light-client` in a local development environment. The architecture comprises:

1. Anvil-based Ethereum devnet operating in mainnet fork mode
2. Katana-based Starknet devnet with configured L1<>L2 messaging bridge
3. Contract deployment pipeline for both L1 (Ethereum) and L2 (Starknet) networks
4. Light Client binary implementation:
   - Event listener for Fossil Store contract emissions
   - State synchronization logic for light client updates
5. Relayer binary implementation for L1->L2 finalized block hash propagation via messaging contract

The system requires multiple concurrent processes, each isolated in separate terminal instances.

## Dependencies

Required toolchain components:

1. Rust toolchain:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Risc0 zkVM toolchain:
   ```bash
   curl -L https://risczero.com/install | bash
   rzup
   ```

3. Dojo framework:
   ```bash
   curl -L https://install.dojoengine.org | bash
   dojoup -v 1.0.0-alpha.16
   ```

3. Foundry development framework:
   ```bash
   curl -L https://foundry.paradigm.xyz | bash
   foundryup
   ```

## Terminal 1: Anvil Ethereum Devnet Configuration

Initialize the Ethereum development environment with Anvil:

1. Load environment configuration:
   ```bash
   source .env
   ```

2. Initialize Anvil instance with mainnet fork:
   ```bash
   anvil --fork-url $ETH_RPC_URL --block-time 12
   ```

> **Technical Note:** Configure `${ETH_RPC_URL}` in `anvil.env` with an RPC endpoint (Infura/Alchemy) for mainnet state replication.

## Terminal 2: Katana Starknet Devnet Initialization

Configure the Starknet development environment with L1 messaging capabilities:

1. Source environment variables:
   ```bash
   source .env
   ```

2. Configure `anvil.messaging.json` with fork block parameters from Anvil initialization output:
   ```
   Fork Configuration
   ==================
   Endpoint:       http://xxx.x.x.x:x
   Block number:   21168847 <--- Required for messaging configuration
   Block hash:     0x67bc863205b5cd53f11d78bccb7a722db1b598bb24f4e11239598825bfb3e4d3
   Chain ID:       1
   ```

3. Initialize Katana with L1 messaging bridge:
   ```bash
   katana --messaging config/anvil.messaging.json --disable-fee --disable-validate
   ```

> **Technical Note:** The `--messaging` flag enables L1<>L2 message passing. `--disable-fee` and `--disable-validate` flags optimize for development environment.

## Terminal 3: Contract Deployment Pipeline

Deploy the messaging infrastructure contracts:

1. Initialize environment:
   ```bash
   source .env
   ```

2. Execute deployment pipeline:
   ```bash
   ./scripts/deploy.sh
   ```

> **Technical Note:** Verify `deploy.sh` configuration for correct contract deployment parameters on Katana network.

## Terminal 4: Light Client Process

Initialize the Light Client service:

1. Navigate to client implementation:
   ```bash
   cd crates/client
   ```

2. Execute client binary:
   ```bash
   cargo run --release
   ```

## Terminal 5: Block Hash Relayer Process

Initialize the L1->L2 block hash relay service:

1. Execute relayer process:
   ```bash
   ./scripts/run_relayer.sh
   ```
