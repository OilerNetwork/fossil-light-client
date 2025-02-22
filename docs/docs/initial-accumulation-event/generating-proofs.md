---
id: generating-proofs
title: Generating the Proofs
---
![Generating Proofs](/img/05.png)

Integrity verification and MMR append operations are executed inside a RISC0 zkVM, generating cryptographic guarantees for correctness without requiring trust in off-chain computations.

## Guest Program Input

The guest program receives a `CombinedInput` structure containing:
- `chain_id`: The Ethereum network identifier
- `batch_size`: Number of blocks to process in this batch
- `headers`: Vector of block headers with their representative timestamps
- `mmr_input`: Current state of the Merkle Mountain Range, including:
  - Initial peaks
  - Current elements count
  - Current leaves count
  - New elements to be added
- `skip_proof_verification`: Flag to control proof verification behavior

## Guest Program Execution

The program performs several key operations:

1. **Block Header Validation**: 
   - Flattens the input block headers into a sequential list
   - Verifies the validity of all block headers and their chain relationship
   - Ensures headers belong to the specified chain ID

2. **MMR Construction**:
   - Initializes the MMR with the previous state (peaks, elements count, leaves count)
   - Sequentially appends each block hash to the MMR
   - Calculates a new root hash after all blocks are processed

3. **State Verification**:
   - Validates batch continuity by checking block numbers
   - Ensures all blocks in the batch belong to the same batch index
   - Handles genesis case specially for the first batch

## Guest Program Output

The program produces a `GuestOutput` containing:
- `batch_index`: The sequential index of this batch
- `latest_mmr_block`: Most recent block number included in the MMR
- `latest_mmr_block_hash`: Hash of the most recent block
- `root_hash`: The new MMR root hash after processing
- `leaves_count`: Updated total number of leaves in the MMR
- `first_block_parent_hash`: Parent hash of the first block in the batch (or zeros for genesis)

For each batch, a Groth16 proof is produced. This proof includes the Groth16 verification key and the RISC0 receipt, serialized into a `felt252` array using the Garaga Rust SDK. The serialized proof is then sent to the Fossil Snark Verifier (FSV) on Starknet for validation.
