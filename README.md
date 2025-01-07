# Fossil Light Client Local Testing Setup

This README outlines the technical configuration for deploying and testing the `fossil-light-client` in a local development environment.

## Prerequisites: Installing Docker

Before getting started, you'll need Docker and Docker Compose installed on your system.

### Installing Docker
- **Windows & Mac**: Download and install [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- **Linux**: Follow the [official installation instructions](https://docs.docker.com/engine/install/) for your distribution
  - After installation on Linux, remember to follow the [post-installation steps](https://docs.docker.com/engine/install/linux-postinstall/) to run Docker without sudo

### Installing Docker Buildx (Linux only)

If you're on Linux, you'll need to install Docker Buildx:

```bash
# Create docker cli plugins directory
mkdir -p ~/.docker/cli-plugins/

# Download buildx binary
curl -L https://github.com/docker/buildx/releases/download/v0.12.1/buildx-v0.12.1.linux-amd64 -o ~/.docker/cli-plugins/docker-buildx

# Make it executable
chmod +x ~/.docker/cli-plugins/docker-buildx
```

### Verifying Installation
After installation, verify that Docker is properly installed:
```bash
docker --version
docker compose version
docker buildx version  # Should work after installing buildx
```

You should see version numbers for both commands. If you get any errors, consult the [Docker troubleshooting guide](https://docs.docker.com/troubleshoot/).

## Quick Start with Docker

The easiest way to get started is using Docker. This method handles all dependencies and environment setup automatically.

### Prerequisites

- Docker
- Docker Compose
- Docker Buildx (for Linux users)

### Building the Images

First, build all required Docker images:

```bash
# Make the build script executable
chmod +x scripts/build-images.sh

# Build all images
# For normal build:
./scripts/build-images.sh

# For verbose build output:
./scripts/build-images.sh --verbose  # or -v
```

This will build the following images:
- anvil: Ethereum development node
- katana: StarkNet development node
- deploy: Deployment container for contracts
- build-mmr: MMR builder service
- relayer: Block hash relayer service
- client: Fossil light client

> **Note:** The `--verbose` flag shows detailed build progress and is useful for debugging build issues.

### Running the Stack

The application is split into two parts: core infrastructure and services. They need to be run in a specific sequence.

#### 1. Start Core Infrastructure

First, start the core infrastructure services (Ethereum node, StarkNet node, and deployments):

```bash
# Start anvil, katana, and run deployments
docker-compose up -d

# Wait for all deployments to complete
# You can check logs with:
docker-compose logs -f
```

#### 2. Run Services

After the core infrastructure is running and deployments are complete, run the additional services in sequence:

```bash
# 1. Run MMR Builder
docker-compose -f docker-compose.services.yml run --rm mmr-builder

# 2. Start the Relayer
docker-compose -f docker-compose.services.yml up -d relayer

# 3. Start the Client
docker-compose -f docker-compose.services.yml up -d client
```

### Monitoring

You can monitor the services using:

```bash
# Check all running containers
docker ps

# View logs for specific services
docker-compose logs -f               # For core infrastructure
docker-compose -f docker-compose.services.yml logs -f  # For services

# View logs for specific container
docker logs -f <container-name>
```

### Cleanup

To stop and remove all containers:

```bash
# Stop core infrastructure
docker-compose down

# Stop services
docker-compose -f docker-compose.services.yml down

# Remove the docker network
docker network rm fossil-network
```

### Troubleshooting Docker Setup

If you see warnings about orphaned containers:
```bash
docker-compose -f docker-compose.services.yml up -d --remove-orphans
```

To reset everything and start fresh:
```bash
# Stop and remove all containers
docker-compose down
docker-compose -f docker-compose.services.yml down

# Remove all related containers (optional)
docker rm $(docker ps -a -q --filter name=fossil-light-client)

# Start again from step 1
```

#### Network Issues

To check existing networks:
```bash
docker network ls
```

To clean up and recreate the network:
```bash
# Remove existing network (if any)
docker network rm fossil-network

# Network will be automatically created when running docker-compose up
```

## Advanced: Manual Setup with Local Tools

If you need to run the components locally without Docker, follow these instructions.

### Dependencies

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
   dojoup 
   ```

4. Foundry development framework:
   ```bash
   curl -L https://foundry.paradigm.xyz | bash
   foundryup
   ```

### Manual Setup Instructions

#### Terminal 1: Anvil Ethereum Devnet Configuration

1. Load environment configuration:
   ```bash
   source .env
   ```

2. Initialize Anvil instance with mainnet fork:
   ```bash
   anvil --fork-url $ETH_RPC_URL --block-time 12
   ```

> **Technical Note:** Configure `${ETH_RPC_URL}` in `anvil.env` with an RPC endpoint (Infura/Alchemy) for mainnet state replication.

#### Terminal 2: Katana Starknet Devnet Initialization

1. Source environment variables:
   ```bash
   source .env
   ```

2. Configure `anvil.messaging.json` with fork block parameters from Anvil initialization output.

3. Initialize Katana with L1 messaging bridge:
   ```bash
   katana --messaging $ANVIL_CONFIG --disable-fee --disable-validate
   ```

#### Terminal 3: Contract Deployment Pipeline

1. Initialize environment:
   ```bash
   source .env
   ```

2. Execute deployment pipeline:
   ```bash
   ./scripts/deploy.sh
   ```

#### Terminal 4: MMR Builder Options

The MMR builder supports several options for controlling how the MMR is built:

```bash
# Build MMR with default settings (from latest finalized block)
cargo run --bin build_mmr --release

# Build from a specific start block
cargo run --bin build_mmr --release -- --start-block <BLOCK_NUMBER>

# Build from the latest onchain MMR block
cargo run --bin build_mmr --release -- --from-latest

# Control batch size
cargo run --bin build_mmr --release -- --batch-size <SIZE>

# Process specific number of batches
cargo run --bin build_mmr --release -- --num-batches <COUNT>

# Skip proof verification
cargo run --bin build_mmr --release -- --skip-proof

# Combine options (examples)
cargo run --bin build_mmr --release -- --from-latest --num-batches 10
cargo run --bin build_mmr --release -- --start-block 1000 --batch-size 512
```

Available options:
- `--start-block, -s`: Start building from this block number
- `--from-latest, -l`: Start building from the latest onchain MMR block
- `--batch-size`: Number of blocks per batch (default: 1024)
- `--num-batches, -n`: Number of batches to process
- `--skip-proof, -p`: Skip proof verification
- `--env-file, -e`: Path to environment file (default: .env)

Note: `--from-latest` and `--start-block` cannot be used together.

#### Terminal 4: Light Client Process

Execute client binary:
```bash
cargo run --bin client --release
```

The client supports the following options:

```bash
# Run with default settings (5 second polling interval)
cargo run --bin client --release

# Run with custom polling interval (in seconds)
cargo run --bin client --release -- --polling-interval 10

# Use a specific environment file
cargo run --bin client --release -- --env-file .env.local
```

Available options:
- `--polling-interval`: Time between polls in seconds (default: 5)
- `--env-file, -e`: Path to environment file (default: .env)

#### Terminal 5: Block Hash Relayer Process

Execute relayer process:
```bash
./scripts/run_relayer.sh
```
