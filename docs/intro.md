# Fossil Light Client - Documentation

## Introduction

The Fossil Light Client provides trustless guarantees of Ethereum block data integrity, serving as the foundation for Fossil's computational processes.

It achieves this by leveraging advanced cryptographic techniques, direct access to Ethereum block hashes via Solidity opcodes, and L1-to-L2 messaging.

All Ethereum block hashes are verified for integrity and stored in on-chain Merkle Mountain Range (MMR) states on Starknet. When a request for Ethereum data is made to Fossil, each block hash in the requested range can be validated as part of the on-chain MMR states.

This documentation provides a comprehensive technical overview of the system architecture, implementation details, and operational procedures for developers maintaining and extending the Fossil ecosystem.

## System Architecture Overview

![Fossil System Architecture](./images/fossil-architecture.png)

**Fossil System Architecture**: This diagram illustrates the complete Fossil data and computation pipeline. Finalized Ethereum block headers are indexed and validated through the Fossil Light Client and MMR Builder using the RISC0 zkVM. Validated fee data and Merkle Mountain Range (MMR) roots are stored on Starknet in the Fossil Store contract, with large state data persisted on IPFS. The Light Client continuously synchronizes new finalized blocks through an L1→L2 relayer, ensuring continuous updates. The Pitchlake Coprocessor—comprising the Fossil API, Proving Service, and Message Handler—consumes this fee data to perform verifiable computations (e.g., options pricing) in RISC0. Verified results are published to the Pitchlake Vault contract for protocol-level use.

## 📖 Documentation Structure

### Core Documentation

- **[Technical Specification](./technical-specification.md)** - Comprehensive technical overview of the entire Fossil system including architecture, data pipeline, security guarantees, and future considerations

### System Architecture

- **[High-Level Architecture](./system-overview/high-level-architecture.md)** - Overview of the two main components: MMR Builder and Light Client
- **[Components and Data Flow](./system-overview/components-and-data-flow.md)** - Detailed description of all system components and their interactions
- **[Key Technologies](./system-overview/key-technologies.md)** - Technologies used in the Fossil ecosystem

### Initial MMR Accumulation

- **[Initial Accumulation Overview](./initial-accumulation-event/intro.md)** - Purpose and process of the initial MMR accumulation
- **[Data Source](./initial-accumulation-event/data-source.md)** - Ethereum block header database and data retrieval
- **[Batch Selection](./initial-accumulation-event/batch-selection.md)** - How blocks are organized into batches
- **[Block Headers Validation](./initial-accumulation-event/block-headers-validation.md)** - Integrity and chain validity checks
- **[Constructing the MMR](./initial-accumulation-event/constructing-mmr.md)** - Merkle Mountain Range construction details
- **[Database Setup](./initial-accumulation-event/database-setup.md)** - PostgreSQL database schema and access
- **[Generating Proofs](./initial-accumulation-event/generating-proofs.md)** - RISC0 zkVM proof generation process
- **[On-Chain Submission](./initial-accumulation-event/onchain-submission.md)** - Proof verification and state storage on Starknet

### Light Client Updates

- **[Update Overview](./updating-light-client/intro.md)** - How the Light Client processes new blocks
- **[Relaying Block Hashes](./updating-light-client/relaying-block-hashes.md)** - L1 to L2 block hash relay mechanism
- **[Initiating Updates](./updating-light-client/initiating-update.md)** - Event monitoring and update trigger process

### Gas Fee Calculation

- **[Fee Calculation Overview](./calculating-avg-gas-fees/intro.md)** - System for calculating and storing average gas fees
- **[Block Header Grouping](./calculating-avg-gas-fees/grouping-headers.md)** - How blocks are grouped by hour
- **[ZK Proof of Fees](./calculating-avg-gas-fees/zkvm-calculation.md)** - Fee calculation inside RISC0 zkVM
- **[Fee Storage and Retrieval](./calculating-avg-gas-fees/storage-and-retrieval.md)** - On-chain storage structure and query methods

### Integration

- **[Pitchlake Integration](./pitchlake-integration/intro.md)** - How Pitchlake Coprocessor leverages Fossil's validated data

## Quick Start

For deployment instructions, configuration options, and troubleshooting, refer to the main [README](../README.md) in the repository root.
