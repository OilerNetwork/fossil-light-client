<div align="center">

<h1>🦴 Fossil Light Client</h1>

<!-- CI Status Badges -->

[![Cairo Workflow](https://github.com/OilerNetwork/fossil-light-client/actions/workflows/cairo.yml/badge.svg?branch=sepolia-deployment)](https://github.com/OilerNetwork/fossil-light-client/actions/workflows/cairo.yml)
[![Rust Workflow](https://github.com/OilerNetwork/fossil-light-client/actions/workflows/rust.yml/badge.svg?branch=sepolia-deployment)](https://github.com/OilerNetwork/fossil-light-client/actions/workflows/rust.yml)

<!-- Project Information Badges -->

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](http://www.apache.org/licenses/LICENSE-2.0)
[![Ethereum](https://img.shields.io/badge/Ethereum-3C3C3D?style=flat&logo=ethereum&logoColor=white)](https://ethereum.org/)
[![Starknet](https://img.shields.io/badge/Starknet-Powered-purple?style=flat)](https://starknet.io/)
[![RISC Zero](https://img.shields.io/badge/RISC_Zero-ZK_Proofs-orange?style=flat)](https://www.risczero.com/)

**A lightweight Ethereum client for Starknet with ZK-powered fee state proofs**

</div>

<hr />

## 📋 Table of Contents

- [📋 Table of Contents](#-table-of-contents)
- [✨ Key Features](#-key-features)
- [📚 Detailed Documentation](#-detailed-documentation)
  - [Deployment Options](#deployment-options)
  - [Documentation Setup](#documentation-setup)
- [🐋 Docker-Based Deployment](#-docker-based-deployment)
  - [Docker Prerequisites](#docker-prerequisites)
  - [Docker Deployment Options](#docker-deployment-options)
    - [Option 1: Using Local Development Network](#option-1-using-local-development-network)
    - [Option 2: Using Production Networks](#option-2-using-production-networks)
  - [Management Commands](#management-commands)
- [🔧 Manual Compilation and Execution](#-manual-compilation-and-execution)
  - [Manual Prerequisites](#manual-prerequisites)
  - [Setup and Execution](#setup-and-execution)
  - [Block Range Selection for Fee State Proofs](#block-range-selection-for-fee-state-proofs)
- [🛠️ Advanced Topics](#️-advanced-topics)
  - [Building Docker Containers Locally](#building-docker-containers-locally)
  - [Manual Contract Deployment](#manual-contract-deployment)
    - [Manual Deployment Prerequisites](#manual-deployment-prerequisites)
    - [Deploying the Ethereum Smart Contract](#deploying-the-ethereum-smart-contract)
    - [Deploying the Starknet Smart Contract](#deploying-the-starknet-smart-contract)
  - [Deploying to Sepolia Network](#deploying-to-sepolia-network)
- [❓ Troubleshooting](#-troubleshooting)
  - [Docker Issues](#docker-issues)
  - [Common Issues](#common-issues)

<hr />

## ✨ Key Features

- 🔗 **Cross-Chain Communication**: Securely relay Ethereum block hashes to Starknet
- 🔒 **ZK-Powered Verification**: Use zero-knowledge proofs for efficient verification
- 📊 **Fee State Proofs**: Generate and verify cryptographic proofs of Ethereum gas fees
- 🚀 **Scalable Architecture**: Process Ethereum blocks in batches with MMR accumulation
- 🛠️ **Flexible Deployment**: Run via Docker or compile manually for development


## 📚 Detailed Documentation

### Deployment Options

This documentation outlines two deployment approaches:

1. 🐋 **Docker-Based Deployment**: Recommended for most users, handles all dependencies automatically
2. 🔧 **Manual Compilation**: For development and debugging, runs light client binaries from source

### Documentation Setup

To run the documentation locally:

1. Install Yarn:
   - **For macOS:**

     ```bash
     # Using Homebrew
     brew install yarn
     # Using npm
     npm install --global yarn
     ```

   - **For Linux:**

     ```bash
     # Using npm
     npm install --global yarn
     # Using Debian/Ubuntu
     curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | sudo apt-key add -
     echo "deb https://dl.yarnpkg.com/debian/ stable main" | sudo tee /etc/apt/sources.list.d/yarn.list
     sudo apt update
     sudo apt install yarn
     ```

   - **For Windows:**

     ```bash
     # Using npm
     npm install --global yarn
     # Using Chocolatey
     choco install yarn
     # Using Scoop
     scoop install yarn
     ```

2. Start the documentation server:

   ```bash
   cd docs/
   yarn
   yarn start
   ```

This will start a local server and open the documentation in your default browser. The documentation will automatically reload when you make changes to the source files.

## 🐋 Docker-Based Deployment

### Docker Prerequisites

1. Install [Docker Desktop](https://www.docker.com/products/docker-desktop/) (includes Docker Engine and Compose)
2. For Linux only: Install Docker Buildx

   ```bash
   mkdir -p ~/.docker/cli-plugins/
   curl -L https://github.com/docker/buildx/releases/download/v0.12.1/buildx-v0.12.1.linux-amd64 -o ~/.docker/cli-plugins/docker-buildx
   chmod +x ~/.docker/cli-plugins/docker-buildx
   ```

### Docker Deployment Options

#### Option 1: Using Local Development Network

This approach builds a local Ethereum and Starknet network for development and testing:

1. Set up configuration:

   ```bash
   cp config/.env.example .env
   cp config/.env.docker.example .env.docker
   ```

2. Start core network infrastructure:

   ```bash
   docker-compose up -d
   docker-compose logs -f  # Monitor until initialization complete
   ```

3. Run MMR accumulation:

   ```bash
   # Run with default settings (build all batches until block #0)
   ENV_FILE=.env.docker docker-compose -f docker-compose.accumulation.yml up

   # Or specify number of batches (build with 4 batches only)
   ENV_FILE=.env.docker NUM_BATCHES=4 docker-compose -f docker-compose.accumulation.yml up
   ```

4. Run The Light Client:

   ```bash
   # Run with default settings
   ENV_FILE=.env.docker docker-compose -f docker-compose.client.yml up
   ```

5. Run The Relayer:

   ```bash
   # Run with default settings (default: relays block hashes every 60 minutes)
   ENV_FILE=.env.docker docker-compose -f docker-compose.relayer.yml up

   # Or specify the relay time (relays block hashes every 5 minutes)
   ENV_FILE=.env.docker RELAY_TIME_MINUTES=5 docker-compose -f docker-compose.relayer.yml up
   ```

#### Option 2: Using Production Networks

This approach connects to existing Ethereum and Starknet networks:

1. Set up configuration:

   ```bash
   # For testnet (Sepolia)
   cp config/.env.local.example .env.sepolia

   # For mainnet
   cp config/.env.local.example .env.mainnet
   ```

2. Update the configuration file with:
   - Ethereum RPC endpoint
   - Starknet RPC endpoint
   - Contract addresses for deployed contracts
   - API keys and other required credentials

3. Run MMR accumulation:

   ```bash
   # For testnet (Sepolia)
   ENV_FILE=.env.sepolia docker-compose -f docker-compose.accumulation.yml up

   # For mainnet
   ENV_FILE=.env.mainnet docker-compose -f docker-compose.accumulation.yml up

   # Optionally specify number of batches
   ENV_FILE=.env.sepolia NUM_BATCHES=4 docker-compose -f docker-compose.accumulation.yml up
   ```

4. Run The Light Client:

   ```bash
   # Run with default settings
   ENV_FILE=.env.sepolia docker-compose -f docker-compose.client.yml up
   ```

5. Run The Relayer:

   ```bash
   # Run with default settings (default: relays block hashes every 60 minutes)
   ENV_FILE=.env.sepolia docker-compose -f docker-compose.relayer.yml up
   ```

The Docker image will be automatically pulled from DockerHub, so no local build is required.

### Management Commands

```bash
# View containers
docker ps

# View logs
docker-compose logs -f
docker-compose -f docker-compose.accumulation.yml logs -f

# Stop everything
docker-compose down
docker-compose -f docker-compose.accumulation.yml down
```

## 🔧 Manual Compilation and Execution

This setup uses Docker only for networks (Ethereum & StarkNet) and contract deployments, while running light client components directly with Cargo.

### Manual Prerequisites

You can install all prerequisites automatically with:

```bash
make setup
```

This will install all necessary dependencies:
- Rust (with nightly toolchain)
- Foundry for Ethereum development
- RISC Zero tools
- Starknet development toolchain (scarb, starknet-foundry, starkli)
- Platform-specific requirements

For individual component installation:

```bash
# Install only specific components
make setup-rust         # Install Rust and nightly toolchain
make setup-foundry      # Install Foundry
make setup-risc0        # Install RISC Zero
make setup-starknet     # Install Starknet tools
make setup-platform     # Install platform-specific dependencies
make init-repo          # Initialize git submodules
```

See the [Makefile](./Makefile) for all available commands.

### Setup and Execution

1. Configure environment:

   ```bash
   cp config/.env.local.example .env.local
   ```

2. Start networks and deploy contracts:

   ```bash
   chmod +x scripts/build-network.sh
   ./scripts/build-network.sh
   ```

   **Option 1: Standard deployment (with container build)**

   ```bash
   docker-compose up
   ```

   **Option 2: Faster deployment (build locally first)**
   To save time during network container boot-up, you can build the Starknet contracts locally before running docker-compose:

   ```bash
   # Build Starknet contracts locally
   scarb build
   cd ../..

   # Run docker-compose with NO_BUILD=1 to skip the build step in the container
   NO_BUILD=1 docker-compose up
   ```

   Wait for the `deploy-starknet` container to complete the deployment of all StarkNet contracts. The deployment is finished when you see a log message indicating environment variables have been updated. (it might take a few minutes)

3. Build the project:

   ```bash
   cargo build
   ```

4. Build MMR and generate proofs:

   ```bash
   cargo run --bin build-mmr -- --num-batches 2 --env-file .env.local
   ```

5. Start the client:

   ```bash
   cargo run --bin client -- --env-file .env.local
   ```

6. Start the relayer:

   ```bash
   chmod +x scripts/run_relayer_local.sh
   ./scripts/run_relayer_local.sh
   ```

7. Test Fee Proof Fetching:

   ```bash
   starkli call <fossil_store_contract_address> get_avg_fees_in_range <start_timestamp> <end_timestamp> --rpc http://localhost:5050
   ```

### Block Range Selection for Fee State Proofs

When requesting state proofs for fees, you can query any hour-aligned timestamp or range within the processed blocks. The system aggregates fees hourly and requires timestamps to be multiples of 3600 seconds (1 hour).

For example, if blocks from timestamp 1704067200 (Jan 1, 2024 00:00:00 UTC) to 1704153600 (Jan 2, 2024 00:00:00 UTC) have been processed:

- You can query a single hour: 1704070800 (Jan 1, 2024 01:00:00 UTC)
- Or a range: 1704067200 to 1704153600 (full 24 hours)
- Or any subset of hours within these bounds

Key validation rules:

- All timestamps must be hour-aligned (multiples of 3600 seconds)
- For range queries, start timestamp must be ≤ end timestamp
- Queries return weighted average fees based on number of blocks in each hour

Note: While blocks are processed in batches internally, fee queries operate on hour boundaries regardless of batch structure.

## 🛠️ Advanced Topics

### Building Docker Containers Locally

To build the docker containers locally (currently only supported on Linux machines):

```bash
chmod +x scripts/build-all.sh
./scripts/build-all.sh
```

This process might take a while depending on your machine. Note that building images locally is currently only supported on Linux due to platform-specific binary compatibility requirements. macOS and Windows users should use the pre-built images from DockerHub instead.

### Manual Contract Deployment

There are multiple contracts that need to be deployed on Ethereum and Starknet:

1. L1 Message Sender (Ethereum)
2. Fossil Store (Starknet)
3. L1 Message Proxy (Starknet)
4. Groth16 Verifier (Starknet)
5. Fossil Verifier (Starknet)

#### Manual Deployment Prerequisites

Make sure you have the following installed:

1. Foundry (<https://book.getfoundry.sh/getting-started/installation>)
2. Starkli (<https://book.starkli.rs/installation>)

You'll need both Ethereum and Starknet wallets for deployment:

1. Create a new `.env.sepolia` file:

   ```bash
   ETH_RPC_URL=
   STARKNET_RPC_URL=
   ACCOUNT_PRIVATE_KEY=
   STARKNET_ACCOUNT=
   STARKNET_ACCOUNT_ADDRESS=
   ```

2. Set up RPC providers:

   ```bash
   ETH_RPC_URL=https://eth-sepolia.g.alchemy.com/v2/xxxxxx # replace with your API key
   STARKNET_RPC_URL=https://starknet-sepolia.g.alchemy.com/starknet/version/rpc/v0_7/xxxxxx # replace with your API key
   ```

3. Set up Ethereum wallet (Metamask):
   - Download Metamask from <https://metamask.io/download>
   - Create a wallet and save the seed phrase
   - Get your private key and add it to `.env.sepolia`:

     ```bash
     ACCOUNT_PRIVATE_KEY=0x01234456 # replace with your actual private key
     ```

4. Set up Starknet wallet (Starkli):

   ```bash
   starkli account oz init deploy_wallet
   starkli account deploy deploy_wallet
   ```

   Add to `.env.sepolia`:

   ```bash
   STARKNET_ACCOUNT=deploy_wallet # path to your account file
   STARKNET_ACCOUNT_ADDRESS=0x00ecac1256b0f48686dd90819299537d3bd2a8fc192402b926d3eff516307f87 # your address
   ```

5. Add Starknet messaging contract:

   ```bash
   SN_MESSAGING=0xE2Bb56ee936fd6433DC0F6e7e3b8365C906AA057
   ```

#### Deploying the Ethereum Smart Contract

```bash
chmod +x ./scripts/deploy-ethereum.sh
./scripts/deploy-ethereum.sh sepolia
```

This will update your `.env.sepolia` with the L1_MESSAGE_SENDER address.

#### Deploying the Starknet Smart Contract

```bash
chmod +x ./scripts/deploy-starknet.sh
./scripts/deploy-starknet.sh sepolia
```

This will update your `.env.sepolia` with addresses for:

- L2_MSG_PROXY
- FOSSIL_STORE
- STARKNET_VERIFIER
- FOSSIL_VERIFIER

### Deploying to Sepolia Network

To deploy to Sepolia testnet:

1. Configure Environment:

   ```bash
   cp config/.env.local.example .env.sepolia
   nano .env.sepolia
   ```

   Ensure these variables are properly set:
   - `ETH_RPC_URL`: Your Sepolia Ethereum RPC endpoint
   - `ACCOUNT_PRIVATE_KEY`: Private key for deployment
   - `SN_MESSAGING`: Starknet core messaging contract address

## ❓ Troubleshooting

### Docker Issues

- Reset deployment:

  ```bash
  docker-compose down
  docker-compose -f docker-compose.accumulation.yml down
  docker network rm fossil-network
  ```

- Remove orphaned containers:

  ```bash
  docker-compose up -d --remove-orphans
  ```

### Common Issues

- Ensure IPFS daemon is running
- Verify Docker network connectivity
- Check logs: `docker-compose logs -f`
