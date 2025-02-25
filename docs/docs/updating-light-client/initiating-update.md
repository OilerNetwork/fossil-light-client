---
id: initiating-update
title: Initiating the Update Process
---

The Client Updater (CU) monitors the Fossil Store (FS) contract for `LatestBlockhashFromL1Stored` events, which indicate new finalized block hashes have been relayed from Ethereum L1.

## Event Processing

1. **Event Detection**:
   - The CU polls for new events between its last processed block and the latest Starknet block
   - Events are filtered using the FS contract address and the `LatestBlockhashFromL1Stored` event selector

2. **State Retrieval**:
   - The CU queries two key pieces of information from the FS contract:
     - Latest relayed block from L1 using `get_latest_blockhash_from_l1()`
     - Latest MMR block using `get_latest_mmr_block()`
   - These values determine the range of blocks requiring an update

3. **Update Range Determination**:
   - Start: `latest_mmr_block + 1`
   - End: `latest_relayed_block`
   - The update only proceeds if `latest_mmr_block < latest_relayed_block`

## MMR State Management

The FS contract maintains the MMR state in batches, where each batch contains:

- Latest MMR block number and its hash
- Number of leaves in the batch
- Root hash of the MMR
- First block's parent hash
- IPFS hash for off-chain data

This batch structure allows for efficient state tracking and verification of continuous chain links between batches.

## Off-chain State Retrieval

For each batch, the complete MMR state is stored in a SQLite database file (`.db`) on IPFS:

1. **IPFS Hash Retrieval**:
   - The CU fetches the IPFS hash from the FS contract's MMR batch data
   - Basic validation ensures the hash starts with "Qm"

2. **Database Download**:
   - The `.db` file is downloaded from IPFS with size limits (default 50MB)
   - Files are downloaded atomically using temporary files
   - Parent directories are created if they don't exist

## Proof Generation and State Updates

The process of generating proofs and updating the MMR state follows the same pattern as the initial accumulation:

- Blocks are processed in configurable batch sizes
- Each batch generates a RISC0 zkVM proof
- The proof is converted to a Groth16 SNARK
- The proof and updated state are submitted to the Fossil Snark Verifier
- The FS contract state is updated with new MMR data and IPFS hashes

This ensures consistency between the initial accumulation and ongoing updates while maintaining the chain's integrity.
