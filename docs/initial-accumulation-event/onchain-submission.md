--- 
id: onchain-submission
title: On-Chain Submission
---

The Fossil Snark Verifier (FSV) deserializes the proof and verifies its validity through pairing checks.

Upon successful verification, the FSV extracts the journal (proof output) directly on-chain, ensuring all relevant data is derived securely and trustlessly.

## Journal Structure

The journal is extracted, onchain, from the proof, this guarantees that the data is derived securely and trustlessly.

The journal is composed of the following elements:
```rust
pub struct Journal {
    pub batch_index: u64,
    pub latest_mmr_block: u64,
    pub latest_mmr_block_hash: u256,
    pub root_hash: u256,
    pub leaves_count: u64,
    pub first_block_parent_hash: u256,
}
```

## On-Chain Storage

```rust
    #[starknet::storage_node]
    pub(crate) struct MMRBatch {
        latest_mmr_block: u64,
        latest_mmr_block_hash: u256,
        leaves_count: u64,
        root_hash: u256,
        first_block_parent_hash: u256,
        ipfs_hash: ByteArray,
    }

    #[storage]
    struct Storage {
        ...,
        mmr_batches: Map<u64, MMRBatch>,
        ...
    }
```

The MMR state stored on-chain is composed of the following elements, all extracted from the proof journal:

- **Batch Index**: Used as the mapping index for the MMR state.

- **Latest MMR Block**: Used as a reference for FLC updates.

- **Latest MMR Block Hash**: Used to validate the link between consecutive batches.

- **MMR Root Hash**: Used to generate block hash inclusion proofs.

- **Leaves Count**: The number of block hashes appended to the MMR.

- **First Block Parent Hash**: Used to verify the batches link onchain.

- **IPFS Address**: The IPFS hash for the LiteSQL `.db` file containing the full off-chain MMR state, including all intermediate and leaf hashes.

By extracting and verifying these elements directly from the proof journal and including the IPFS address in the MMR state, the system ensures tamper-proof and trustless updates.

Public availability of the on-chain MMR state and IPFS address allows anyone to independently verify the integrity of Fossil's block data.
