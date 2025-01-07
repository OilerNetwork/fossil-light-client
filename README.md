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
   dojoup 
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
   katana --messaging $ANVIL_CONFIG --disable-fee --disable-validate
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

1. Execute client binary:
   ```bash
   cargo run --bin client --release
   ```

## Terminal 5: Block Hash Relayer Process

Initialize the L1->L2 block hash relay service:

1. Execute relayer process:
   ```bash
   ./scripts/run_relayer.sh
   ```

## Running with Docker

The application is split into two parts: core infrastructure and services. They need to be run in a specific sequence.

### 1. Start Core Infrastructure

First, start the core infrastructure services (Ethereum node, StarkNet node, and deployments):

```bash
# Start anvil, katana, and run deployments
docker-compose up -d

# Wait for all deployments to complete
# You can check logs with:
docker-compose logs -f
```

### 2. Run Services

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

### Troubleshooting

If you see warnings about orphaned containers, you can clean them up using:

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

# Network Issues

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
