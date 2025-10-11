# Pitchlake Coprocessor Integration

> **Note:** The Pitchlake Coprocessor is maintained in a separate repository. This section provides a brief overview of how it integrates with Fossil.

## Overview

The Pitchlake Coprocessor leverages Fossil's validated fee data for on-chain computations in the Pitchlake options protocol. It consumes the trustless Ethereum gas fee data stored by Fossil and performs verifiable computations using RISC0 zero-knowledge proofs.

## Architecture Components

The Pitchlake Coprocessor consists of three main components:

| Component | Description |
|-----------|-------------|
| **fossil-api** | Receives computation requests from the Pitchlake backend and forwards them to the proving service |
| **fossil-proving-service** | Queues requests in AWS SQS for asynchronous processing |
| **message-handler** | Consumes queued requests, executes computations inside a RISC0 VM, and generates ZK proofs |

## Data Flow

1. **Request Submission**: The Pitchlake backend submits computation requests to the fossil-api
2. **Queue Management**: fossil-proving-service enqueues requests in AWS SQS for asynchronous processing
3. **Computation**: message-handler consumes queued requests and executes the required computations within a RISC0 VM
4. **Proof Generation**: The computation results are written to a proof journal and a ZK proof is generated
5. **On-chain Verification**: The proof is submitted to Starknet for verification through the Fossil Store contract
6. **Data Extraction**: Once verified, the journal data is extracted and transmitted to the Pitchlake Vault contract, where it becomes available for the Pitchlake options protocol

## Integration Benefits

- **Trustless Computation**: All gas fee data is cryptographically verified before use
- **Scalability**: Asynchronous processing via SQS enables handling multiple requests efficiently
- **Verifiability**: Zero-knowledge proofs ensure computation correctness without revealing private inputs
- **On-chain Availability**: Verified results are directly accessible to Pitchlake smart contracts on Starknet

## Related Documentation

For detailed implementation details, API specifications, and deployment instructions, please refer to the Pitchlake Coprocessor repository.
