# Fossil Light Client Local Testing Setup

Below is a modified README that includes a brief Quick-Run Checklist, an overview of the two main setups (Docker vs. minimal), and a bit more clarity on running order. Everything else is unchanged, but reorganized slightly for quick reference.

---

## Quick-Run Checklist (TL;DR)

1. **Install Docker** (and Docker Compose, Buildx on Linux).  
2. **Set up environment files** (copy `.env` and `.env.docker` from `config/`).  
3. **Build Docker images** by running `./scripts/build-images.sh`.  
4. **Start the core infrastructure** with `docker-compose up -d`.  
5. **Run additional services** in sequence (MMR Builder → Relayer → Client).  
6. **Verify logs** (`docker-compose logs -f` or `docker-compose -f docker-compose.services.yml logs -f`).  
7. **Stop everything** with `docker-compose down` and `docker-compose -f docker-compose.services.yml down`.  

If you’re using the Minimal (non-Docker) setup, skip to [Minimal Setup for State Proof Testing](#minimal-setup-for-state-proof-testing).  

---

## About the Two Approaches
### 1. Docker-Based Quick Start
This is the fastest and easiest way to see everything running. It bundles all dependencies in containers, so you don’t need a local Rust toolchain or Risc0 installed—just Docker.

### 2. Minimal Setup (Non-Docker)
For deeper debugging or if you’re unable to run Docker, you can set up Rust, Risc0, and the IPFS node manually. This setup gives you more control but requires installing more tools.

---

## Prerequisites: Installing Docker

Before getting started, you'll need Docker and Docker Compose installed on your system.

### Installing Docker
- **Windows & Mac**: Download and install [Docker Desktop](https://www.docker.com/products/docker-desktop/)  
- **Linux**: Follow the [official installation instructions](https://docs.docker.com/engine/install/) for your distribution  
  - After installation on Linux, remember to follow the [post-installation steps](https://docs.docker.com/engine/install/linux-postinstall/) to run Docker without sudo  

### Installing Docker Buildx (Linux only)
```bash
mkdir -p ~/.docker/cli-plugins/
curl -L https://github.com/docker/buildx/releases/download/v0.12.1/buildx-v0.12.1.linux-amd64 -o ~/.docker/cli-plugins/docker-buildx
chmod +x ~/.docker/cli-plugins/docker-buildx
```

### Verifying Installation
```bash
docker --version
docker compose version
docker buildx version
```
You should see version numbers for all. If you get errors, consult the [Docker troubleshooting guide](https://docs.docker.com/troubleshoot/).

---

## Environment Configuration

Before proceeding, you'll need to set up the appropriate environment files:

1. For Docker-based setup (Quick Start):  
   - `.env`: Contains only the database address for building Rust crates  
   - `.env.docker`: Contains configuration for running the application  
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

---

## Dependencies (Non-Docker Setup Only)

If you’re doing the Docker-based Quick Start, skip this section. Otherwise, you’ll need:

1. Rust toolchain:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. Risc0 zkVM toolchain:
   ```bash
   curl -L https://risczero.com/install | bash
   rzup
   ```

---

## Quick Start with Docker

This approach handles all dependencies and environment setup automatically.

### Prerequisites
- Docker  
- Docker Compose  
- Docker Buildx (for Linux)  
- IPFS Node:
  - Install [IPFS Desktop](https://github.com/ipfs/ipfs-desktop)
  - Ensure the IPFS daemon is running before proceeding

### 1. Building the Images
```bash
chmod +x scripts/build-images.sh
./scripts/build-images.sh        # Normal build
./scripts/build-images.sh -v     # Verbose output for debugging
```
This builds:
- anvil (Ethereum dev node)
- katana (StarkNet dev node)
- deploy (contracts deployment)
- build-mmr (MMR builder service)
- relayer (block hash relayer)
- client (Fossil light client)

### 2. Running the Stack

#### 2.1 Start Core Infrastructure
```bash
docker-compose up -d
docker-compose logs -f   # Monitor logs until deployments finish
```

#### 2.2 Run Additional Services
Once the above is running and deployments are done:
```bash
docker-compose -f docker-compose.services.yml run --rm mmr-builder
docker-compose -f docker-compose.services.yml up -d relayer
docker-compose -f docker-compose.services.yml up -d client
```

### Monitoring
```bash
docker ps
docker-compose logs -f
docker-compose -f docker-compose.services.yml logs -f
docker logs -f <container-name>
```

### Cleanup
```bash
docker-compose down
docker-compose -f docker-compose.services.yml down
docker network rm fossil-network
```

### Troubleshooting
- If you see orphaned container warnings:
  ```bash
  docker-compose -f docker-compose.services.yml up -d --remove-orphans
  ```
- To reset everything:
  ```bash
  docker-compose down
  docker-compose -f docker-compose.services.yml down
  docker rm $(docker ps -a -q --filter name=fossil-light-client)
  # Start again from the build step
  ```
- Network issues:  
  ```bash
  docker network ls
  docker network rm fossil-network
  ```

---

## Minimal Setup for State Proof Testing

Use this to run a lightweight version of the application if you don’t want a full Docker-based approach.

### Prerequisites
1. IPFS Node (daemon running)  
2. Rust toolchain  
3. Risc0 zkVM toolchain  

### 1. Build Network Images
```bash
chmod +x scripts/build-network.sh
./scripts/build-network.sh
```

### 2. Start Network Services
```bash
docker-compose up -d
docker-compose logs -f
```

Wait for a “deployments complete” message.

### 3. Build MMR (Small Test Set)
```bash
cargo run --bin build-mmr -- -b 4 -n 2 -e .env.local
```
Output shows the processed block range.

### 4. Start State Proof API
```bash
cargo run --bin state-proof-api -- -b 4 -e .env.local
```

### 5. Test Fee Proof Fetching
```bash
cargo run --bin fetch-fees-proof -- --from-block <start_block> --to-block <end_block>
```
Use the block range from step 3.

---

## Block Range Selection for Fee State Proofs

You can request proofs for any range of blocks that have already been processed by the MMR. For example, if blocks 7494088-7494095 have been processed, you can request:
- 7494090-7494093, or
- 7494088-7494095, etc.

The MMR processes blocks in batches, but your proof requests can span any valid subset within the processed block range.

---
