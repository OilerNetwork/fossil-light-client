# Fossil Light Client - Technical Documentation

## 📑 Index

- [Fossil Light Client - Technical Documentation](#fossil-light-client---technical-documentation)
  - [📑 Index](#-index)
  - [Prerequisites for All Users](#prerequisites-for-all-users)
  - [Documentation Setup](#documentation-setup)
  - [Docker-Based Deployment](#docker-based-deployment)
    - [Docker Prerequisites](#docker-prerequisites)
    - [Deployment Steps](#deployment-steps)
    - [Management Commands](#management-commands)
  - [Manual Compilation and Execution](#manual-compilation-and-execution)
    - [Manual Prerequisites](#manual-prerequisites)
    - [Setup and Execution](#setup-and-execution)
    - [Block Range Selection for Fee State Proofs](#block-range-selection-for-fee-state-proofs)
  - [Troubleshooting](#troubleshooting)
    - [Docker Issues](#docker-issues)
    - [Common Issues](#common-issues)
  - [Technical Notes](#technical-notes)

This documentation outlines two deployment approaches for the Fossil Light Client:

1. 🐋 **Docker-Based Deployment**: Recommended for most users, handles all dependencies automatically
2. 🔧 **Manual Compilation**: For development and debugging, runs light client binaries from source

## Prerequisites for All Users

1. Clone the repository:

   ```bash
   git clone https://github.com/OilerNetwork/fossil-light-client.git
   cd fossil-light-client
   ```

2. Initialize repository:

   ```bash
   git submodule update --init --recursive
   ```

3. Install Yarn:
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

4. Install IPFS:
   - Download and install [IPFS Desktop](https://github.com/ipfs/ipfs-desktop/releases)
   - Ensure IPFS daemon is running before proceeding

5. Platform-specific requirements:
   - **For macOS users:**

     ```bash
     # Install Python toolchain and gettext
     brew install python
     brew install gettext
     
     # Add to ~/.zshrc or ~/.bash_profile:
     export PATH="/usr/local/opt/python/libexec/bin:$PATH"
     ```

   - **For Linux users:** No additional requirements

## Documentation Setup

To run the documentation locally:

```bash
cd docs/
yarn
yarn start
```

This will start a local server and open the documentation in your default browser. The documentation will automatically reload when you make changes to the source files.

## Docker-Based Deployment

> ⚠️ **Note**: The Docker-based deployment is currently under development and not functional. Please use the [Manual Compilation and Execution](#manual-compilation-and-execution) method instead.

### Docker Prerequisites

1. Install [Docker Desktop](https://www.docker.com/products/docker-desktop/) (includes Docker Engine and Compose)
2. For Linux only: Install Docker Buildx

   ```bash
   mkdir -p ~/.docker/cli-plugins/
   curl -L https://github.com/docker/buildx/releases/download/v0.12.1/buildx-v0.12.1.linux-amd64 -o ~/.docker/cli-plugins/docker-buildx
   chmod +x ~/.docker/cli-plugins/docker-buildx
   ```

### Deployment Steps

1. Set up configuration:

   ```bash
   cp config/.env.example .env
   cp config/.env.docker.example .env.docker
   ```

2. Build images:

   ```bash
   chmod +x scripts/build-images.sh
   ./scripts/build-images.sh
   ```

3. Start core infrastructure:

   ```bash
   docker-compose up -d
   docker-compose logs -f  # Monitor until initialization complete
   ```

4. Deploy services:

   ```bash
   # Initialize MMR builder
   docker-compose -f docker-compose.services.yml run --rm mmr-builder
   
   # Deploy relayer and client
   docker-compose -f docker-compose.services.yml up -d relayer
   docker-compose -f docker-compose.services.yml up -d client
   ```

### Management Commands

```bash
# View containers
docker ps

# View logs
docker-compose logs -f
docker-compose -f docker-compose.services.yml logs -f

# Stop everything
docker-compose down
docker-compose -f docker-compose.services.yml down
```

## Manual Compilation and Execution

This setup uses Docker only for networks (Ethereum & StarkNet) and contract deployments, while running light client components directly with Cargo.

### Manual Prerequisites

1. Install Rust:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Install Risc0:

   ```bash
   curl -L https://risczero.com/install | bash && rzup
   ```

### Setup and Execution

1. Configure environment:

   ```bash
   cp config/.env.local.example .env.local
   ```

2. Start networks and deploy contracts:

   ```bash
   chmod +x scripts/build-network.sh
   ./scripts/build-network.sh
   docker-compose up
   ```

   Wait for the `deploy-starknet` container to complete the deployment of all StarkNet contracts. The deployment is finished when you see a log message indicating environment variables have been updated. (it might take a few minutes)

3. Build the project:

   ```bash
   cargo build
   ```

4. Build MMR and generate proofs:
   This step will:
   - Start from the latest Ethereum finalized block and process 8 blocks backwards (2 batches * 4 blocks)
   - Generate a ZK proof of computation for each batch
   - Create and store .db files for each MMR batch and upload them to IPFS
   - Generate and verify Groth16 proofs on StarkNet for batch correctness
   - Extract batch state from proof journal and store it in the Fossil Store contract

   ```bash
   cargo run --bin build-mmr -- --batch-size 4 --num-batches 2 --env-file .env.local
   ```

5. Start the relayer:
   This step will:
   - Monitor the latest finalized block on Ethereum
   - Call the L1 contract to relay the finalized block hash to Starknet
   - Automatically retry on failures and continue monitoring
   - Run as a background service with configurable intervals (default: 3 minutes for local testing)

   ```bash
   chmod +x scripts/run_relayer_local.sh
   ./scripts/run_relayer_local.sh
   ```

6. Start the client:
   This step will:
   - Monitor the Fossil Store contract on Starknet for new block hash events
   - Upon receiving a new block hash:
     - Fetch block headers from the latest MMR root up to the new block hash
     - Update the local light client state with the new block headers
     - Verify the cryptographic proofs for each block header
   - Maintain a recent block buffer to handle potential chain reorganizations

   ```bash
   cargo run --bin client -- --batch-size 4 --env-file .env.local
   ```

7. Test Fee Proof Fetching:
   In a new terminal, fetch the fees for a block range from the Fossil Store contract:

   ```bash
   starkli call <fossil_store_contract_address> get_avg_fees_in_range <start_timestamp> <end_timestamp> --rpc http://localhost:5050
   ```

   Note: The block range should match the blocks that were added to the MMR in step 4. You can find these numbers in the build_mmr output logs.

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

## Troubleshooting

### Docker Issues

- Reset deployment:

  ```bash
  docker-compose down
  docker-compose -f docker-compose.services.yml down
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

## Technical Notes

```bash
# View containers
docker ps

# View logs
docker-compose logs -f
docker-compose -f docker-compose.services.yml logs -f

# Stop everything
docker-compose down
docker-compose -f docker-compose.services.yml down
```
