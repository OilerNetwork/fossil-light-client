---
id: technical-specification
title: Fossil Technical Specification
---

## 1. Overview

Fossil is a trustless data infrastructure designed to store Ethereum Layer 1 (L1) base gas fee data on Starknet, enabling verified off-chain computation for the Pitchlake options market. It leverages the RISC0 zkVM to perform deterministic data extraction, validation, and aggregation, producing zero-knowledge proofs verifiable on-chain.

## 2. Architecture Components

### 2.1 Core Infrastructure

| Component | Description | Technology |
|-----------|-------------|------------|
| **RISC0 zkVM** | Executes data validation and computation off-chain in a verifiable environment | Zero-knowledge virtual machine |
| **Fossil Postures Database** | Stores finalized Ethereum block headers indexed by an off-chain indexer | PostgreSQL |
| **Fossil Store Contract** | On-chain Starknet contract maintaining commitments, proof metadata, and IPFS references | Cairo/Starknet |
| **L1MessageProxy Contract** | Handles L1 → L2 message forwarding from Ethereum to Starknet | Cairo/Starknet |
| **Off-chain Relayer** | Periodically transmits finalized L1 block data to Starknet (every 6 hours, configurable) | Rust |
| **IPFS Storage** | Stores large MMR state snapshots referenced by content identifiers (CIDs) | IPFS |

### 2.2 Smart Contracts

**Ethereum L1:**
- **L1 Message Sender (L1MS)**: Sends finalized block hashes to Starknet via L1-to-L2 messaging

**Starknet L2:**
- **L1 Message Proxy (L1MP)**: Receives messages from L1MS and routes them to Fossil ecosystem contracts
- **Fossil Store (FS)**: Stores the latest finalized block hash and MMR states, emits update events
- **Fossil Snark Verifier (FSV)**: Trustlessly verifies Groth16 proofs and extracts journal data
- **Fossil Verifier**: Additional verification layer for proof validation

## 3. Data Pipeline

### 3.1 Header Ingestion

1. Finalized Ethereum block headers are indexed and stored in the Fossil Postures Database
2. The MMR Builder retrieves headers in batches of **1024 blocks** for processing inside the RISC0 VM

### 3.2 Header Validation

Within the zkVM, the following validation steps occur:

1. **Hash Verification**: Each header is rehashed and compared with its stated block hash
2. **Chain Integrity**: Parent-child relationships are validated for sequence integrity
3. **Fee Extraction**: The base fee per gas is extracted from verified headers
4. **Fee Aggregation**: Hourly average base fees are computed across the batch

### 3.3 MMR Construction and Proof Generation

1. A Merkle Mountain Range (MMR) is constructed using the block hashes as leaves
2. The prover commits the following items to the **proof journal**:
   - Hourly average base fees
   - MMR root hash
   - Total number of MMR leaves

3. The complete MMR state is:
   - Serialized to a SQLite database
   - Uploaded to IPFS and referenced by its CID
   - Stored on-chain (proof journal and CID only) in the Fossil Store contract for verification

## 4. System Processes

### 4.1 MMR Builder (Backward Reconstruction)

**Purpose**: Build historical Ethereum block hash commitments from a finalized block to genesis

**Process**:
1. Initializes from a known finalized block hash and number, obtained via an on-chain call
2. Processes batches backward in time (1024 blocks per batch)
3. Reconstructs MMRs iteratively until the genesis block
4. Ensures full historical coverage and trustless validation of Ethereum L1 gas fee data

**Execution**: Runs once before the Light Client starts to establish the base of historical block hashes

### 4.2 Light Client (Forward Synchronization)

**Purpose**: Continuously track new finalized Ethereum blocks and update MMRs

**Process**:
1. Works in tandem with the off-chain relayer, which:
   - Sends finalized block numbers and hashes from Ethereum L1 to Starknet L2 every **6 hours (configurable)**
   - Uses L1 → L2 messaging contracts for transmission

2. On Starknet:
   - The L1MessageProxy receives and forwards block data to the Fossil Store
   - The Fossil Store emits an update event

3. The Fossil Light Client:
   - Listens to these events
   - Retrieves the latest MMR batch metadata
   - Updates the MMR from the previous finalized block to the new one
   - If a batch is incomplete, reconstructs its state using on-chain MMR data and the IPFS snapshot before resuming aggregation

**Execution**: Runs continuously to maintain synchronization with Ethereum's canonical chain

## 5. Fee State Proof System

### 5.1 Data Aggregation

- Block headers are grouped by hour
- Each group contains headers with timestamps within the same hour
- A representative timestamp is calculated for each group (hour × 3600 seconds)
- Weighted average fees are computed based on the number of blocks in each hour

### 5.2 Storage Structure

Fees are stored in the Fossil Store contract with each hourly entry containing:
- Number of data points (blocks in that hour)
- Average fee for that hour
- Timestamps must be multiples of 3600 (seconds in an hour)

### 5.3 Query Requirements

**Validation Rules**:
- All timestamps must be hour-aligned (multiples of 3600 seconds)
- For range queries, start timestamp must be ≤ end timestamp
- Queries return weighted average fees based on number of blocks in each hour

**Example**: If blocks from timestamp 1704067200 (Jan 1, 2024 00:00:00 UTC) to 1704153600 (Jan 2, 2024 00:00:00 UTC) have been processed:
- Query a single hour: 1704070800 (Jan 1, 2024 01:00:00 UTC)
- Query a range: 1704067200 to 1704153600 (full 24 hours)
- Query any subset of hours within these bounds

## 6. Integration with Pitchlake

> **Note**: The Pitchlake Coprocessor is maintained in a separate repository.

### 6.1 Objective

The Pitchlake Coprocessor consumes validated Fossil fee data to perform off-chain computations used in the Pitchlake options market. These computations are verifiable through RISC0-generated zero-knowledge proofs.

### 6.2 Components

| Component | Description |
|-----------|-------------|
| **fossil-api** | Receives computation requests from the Pitchlake backend and forwards them to the proving service |
| **fossil-proving-service** | Queues requests in an AWS SQS queue for asynchronous execution |
| **message-handler** | Consumes queued requests, executes computations within a RISC0 VM, and generates the proof |

### 6.3 Proof Lifecycle

1. **Computation**: The message-handler executes the requested operation in the RISC0 zkVM
2. **Proof Generation**: Results are written to the proof journal
3. **On-chain Verification**: The proof is submitted to Starknet for verification through the Fossil Store contract
4. **Data Extraction**: Verified journal data is forwarded to the Pitchlake Vault contract, enabling on-chain consumption by the Pitchlake protocol

## 7. Security and Trustlessness

### 7.1 Trustless Guarantees

1. **Header Validation**: Every block header is cryptographically verified by rehashing within the zkVM
2. **Chain Integrity**: Parent-child relationships ensure sequential block validity
3. **Proof Verification**: All computations are verified on-chain through SNARK proofs
4. **Data Availability**: MMR states stored on IPFS provide public verifiability

### 7.2 Verification Flow

```
Ethereum Block → Fossil Postures DB → RISC0 VM (validate + compute)
→ Generate Proof → Submit to Starknet → Verify Proof → Store Commitment
```

## 8. Performance Characteristics

- **Batch Size**: 1024 blocks per MMR batch
- **Update Frequency**: Every 6 hours (configurable)
- **Fee Aggregation**: Hourly intervals (3600 seconds)
- **Storage**: On-chain (commitments only), Off-chain (full MMR state in IPFS)

## 9. Current Limitations and Future Considerations

> **⚠️ IMPORTANT FOR FUTURE DEVELOPERS**: The following limitations require attention and planning for long-term system sustainability.

### 9.1 Bonsai Prover Deprecation

Fossil currently relies on the **Bonsai remote prover**, a managed proving service operated by the RISC0 team. However, the RISC0 team has announced plans to **deprecate Bonsai**, and they recommend migrating to **Boundless**, a decentralized and trustless proving marketplace.

**Impact**: This transition is non-trivial, as it introduces architectural and operational implications:

- **Integration Requirements**: Boundless integration requires modifications to Fossil's proof submission and verification workflows
- **Performance Changes**: Proof batching, verification latency, and cost structures will change compared to the managed Bonsai environment
- **Security Considerations**: Security guarantees remain equivalent but require additional protocol-level coordination

### 9.2 Evaluation of Alternative Proving Systems

Given the computational complexity of Pitchlake's pricing models and the size of Fossil's aggregated datasets, **RISC0 may not be the most efficient long-term proving system**.

**Recommendations for future development teams**:

1. **Migrate to Boundless** if maintaining zkVM compatibility with RISC0 is a priority
   - Ensures continuity with current RISC0-based implementation
   - Provides decentralized proving infrastructure

2. **Evaluate SP1 (Succinct)** or similar high-performance zkVMs as potential replacements for RISC0
   - **SP1 Advantages**:
     - Lower proof generation latency
     - Improved scalability for large, data-heavy computations
     - Potential to significantly reduce compute costs
   - **Migration Considerations**:
     - Requires adapting Fossil's proof format
     - Requires updating verification contracts on Starknet
     - Development effort vs. performance gains trade-off analysis needed

### 9.3 Action Items for Future Contributors

Future contributors should:

1. **Monitor RISC0 Deprecation Timeline**: Track Bonsai deprecation schedule and plan migration accordingly
2. **Benchmark Alternative Provers**: Conduct performance analysis comparing:
   - Boundless (RISC0-compatible)
   - SP1 (Succinct)
   - Other emerging zkVM solutions
3. **Cost Analysis**: Evaluate proof generation costs across different proving backends
4. **Migration Planning**: Develop a phased migration strategy that minimizes system downtime
5. **Verification Contract Updates**: Plan for smart contract upgrades to support new proof formats if migrating away from RISC0

**Decision Framework**: Carefully assess the trade-offs between maintaining RISC0 compatibility versus migrating to a more performant proving backend based on:
- Computational efficiency requirements
- Cost constraints
- Development resources available
- Timeline for Bonsai deprecation

## 10. Summary

Fossil establishes a verifiable, decentralized data bridge between Ethereum and Starknet, transforming raw L1 gas fee data into trustless, on-chain accessible state. Its modular architecture—comprising the MMR Builder, Light Client, and integration with the Pitchlake Coprocessor—provides a scalable foundation for zk-powered computation markets.

**Note**: Future teams should prioritize addressing the proving system migration outlined in Section 9 to ensure long-term system sustainability and optimal performance.
