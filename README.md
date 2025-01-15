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

## Environment Configuration

Before proceeding, you'll need to set up the appropriate environment files:

1. For Docker-based setup (Quick Start):
   - `.env`: Contains only the database address for building Rust crates
   - `.env.docker`: Contains configuration for running the application
   Example files are provided in `config/`:
   ```bash
   cp config/.env.example .env
   cp config/.env.docker.example .env.docker
   ```

2. For Local Development (Minimal Setup):
   - `.env.local`: Additional configuration for local development
   ```bash
   cp config/.env.local.example .env.local
   ```

> **Note:** Example configurations can be found in the `config/` directory.

## Dependencies (Non-Docker Setup Only)

The following dependencies are only required if you're NOT using Docker (i.e., for Minimal Setup).
If you're using the Docker-based Quick Start, you can skip this section.

1. Rust toolchain:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Risc0 zkVM toolchain:
   ```bash
   curl -L https://risczero.com/install | bash
   rzup
   ```

## Quick Start with Docker

The easiest way to get started is using Docker. This method handles all dependencies and environment setup automatically.

### Prerequisites

- Docker
- Docker Compose
- Docker Buildx (for Linux users)
- IPFS Node:
  - Install [IPFS Desktop](https://github.com/ipfs/ipfs-desktop)
  - Ensure the IPFS daemon is running before proceeding

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

## Minimal Setup for State Proof Testing

This section describes how to run a minimal subset of the application focused on MMR building and state proof verification.

### Prerequisites

1. IPFS Node:
   - Install [IPFS Desktop](https://github.com/ipfs/ipfs-desktop)
   - Make sure the IPFS daemon is running before proceeding

2. Rust toolchain:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Risc0 zkVM toolchain:
   ```bash
   curl -L https://risczero.com/install | bash
   rzup
   ```

### 1. Build Network Images

First, build the essential network images:

```bash
# Make the build script executable
chmod +x scripts/build-network.sh

# Build network images (anvil, katana, deploy)
./scripts/build-network.sh
```

### 2. Start Network Services

Start the core infrastructure and wait for deployments to complete:

```bash
# Start network services
docker-compose up -d

# Monitor deployment progress
docker-compose logs -f
```

Wait until you see messages indicating that all deployments are complete and environment files are updated.

### 3. Build MMR (Small Test Set)

In a new terminal, build a small MMR with 2 batches of 4 blocks each:

```bash
cargo run --bin build_mmr -- -b 4 -n 2 -e .env.local
```

Monitor the output logs. You'll see information about the blocks being processed. Note the block range being processed - you'll need these numbers for step 5. The output will look similar to:

```
Starting MMR build... start_block=7494088 end_block=7494095
```

### 4. Start State Proof API

In a new terminal, start the state proof API service:

```bash
cargo run --bin state-proof-api -- -b 4 -e .env.local
```

Wait for the service to start up and begin listening for requests.

### 5. Test Fee Proof Fetching

In a new terminal, run the fee proof fetcher using the block range from step 3:

```bash
cargo run --bin fetch-fees-proof -- --from-block <start_block> --to-block <end_block>
```

For example, using the blocks from our example output:
```bash
cargo run --bin fetch-fees-proof -- --from-block 7494088 --to-block 7494095
```

> **Note:** The block range should match the blocks that were added to the MMR in step 3. You can find these numbers in the build_mmr output logs.

## Block Range Selection for Fee State Proofs

When requesting state proofs for fees, you can specify any block range within the available processed blocks. The system processes blocks in batches, but proofs can be requested for any valid range within those batches.

For example, if blocks 7494088-7494095 have been processed:
- You can request proofs for block range 7494090-7494093
- Or 7494088-7494095 (full range)
- Or any other valid subset within these bounds

Note: While the MMR internally processes batches from higher to lower block numbers (e.g., batch 1: 7494092-7494095, batch 2: 7494088-7494091), this is an implementation detail. Your proof requests can span across these internal batch boundaries.
