# Fossil Light Client - Technical Documentation

## 📑 Index
- [Fossil Light Client - Technical Documentation](#fossil-light-client---technical-documentation)
  - [📑 Index](#-index)
  - [Prerequisites for All Users](#prerequisites-for-all-users)
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

3. Install IPFS:
   - Download and install [IPFS Desktop](https://github.com/ipfs/ipfs-desktop/releases)
   - Ensure IPFS daemon is running before proceeding

4. Platform-specific requirements:
   - **For macOS users:**
     ```bash
     # Install Python toolchain and gettext
     brew install python
     brew install gettext
     
     # Add to ~/.zshrc or ~/.bash_profile:
     export PATH="/usr/local/opt/python/libexec/bin:$PATH"
     ```
   - **For Linux users:** No additional requirements

## Docker-Based Deployment

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
   Wait for the `fossil-deploy` container to complete the deployment of all StarkNet contracts. The deployment is finished when you see a log message indicating environment variables have been updated. (it might take a few minutes)

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
   cargo run --bin build-mmr -- --batch-size 4 --num-batches 2 --env .env.local
   ```

5. Start the State Proof API:
   In a new terminal, start the state proof API service. This provides endpoints to query the MMR state and generate inclusion proofs.
   ```bash
   cargo run --bin state-proof-api -- --batch-size 4 --env .env.local
   ```
   Wait for the service to start up and begin listening for requests.

6. Test Fee Proof Fetching:
   In a new terminal, run the fee proof fetcher using a block range from the processed blocks. This example binary will:
   - Send a request to the API for fees within the specified block range
   - The API will fetch the corresponding block data from the Fossil database
   - Each block's integrity will be cryptographically verified
   - Block fees will be extracted and computed within a zkVM environment
   ```bash
   cargo run --bin fetch-fees-proof -- --from-block <start_block> --to-block <end_block>
   ```
   For example:
   ```bash
   cargo run --bin fetch-fees-proof -- --from-block 7494088 --to-block 7494095
   ```
   Note: The block range should match the blocks that were added to the MMR in step 4. You can find these numbers in the build_mmr output logs.

### Block Range Selection for Fee State Proofs
When requesting state proofs for fees, you can specify any block range within the available processed blocks. The system processes blocks in batches, but proofs can be requested for any valid range within those batches.

For example, if blocks 7494088-7494095 have been processed:
- You can request proofs for block range 7494090-7494093
- Or 7494088-7494095 (full range)
- Or any other valid subset within these bounds

Note: While the MMR internally processes batches from higher to lower block numbers (e.g., batch 1: 7494092-7494095, batch 2: 7494088-7494091), this is an implementation detail. Your proof requests can span across these internal batch boundaries.

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