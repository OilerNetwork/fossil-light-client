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
  - [Troubleshooting](#troubleshooting)
    - [Docker Issues](#docker-issues)
    - [Common Issues](#common-issues)
  - [Technical Notes](#technical-notes)

This documentation outlines two deployment approaches for the Fossil Light Client:
1. 🐋 **Docker-Based Deployment**: Recommended for most users, handles all dependencies automatically
2. 🔧 **Manual Compilation**: For development and debugging, runs light client binaries from source

## Prerequisites for All Users

1. Initialize repository:
   ```bash
   git submodule update --init --recursive
   ```

2. Install IPFS:
   - Download and install [IPFS Desktop](https://github.com/ipfs/ipfs-desktop/releases)
   - Ensure IPFS daemon is running before proceeding

3. Platform-specific requirements:
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
   docker-compose up -d
   ```

3. Run light client components:
   ```bash
   # Build MMR (processes 8 blocks: 2 batches * 4 blocks)
   cargo run --bin build-mmr -- --batch-size 4 --num-batches 2 --env .env.local

   # Start API
   cargo run --bin state-proof-api -- --batch-size 4 --env .env.local
   ```

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