---
id: onchain-storage
title: On-Chain Storage
---

The updated MMR state for each batch is stored on-chain, maintaining the same structure as the initial accumulation event. The following elements are included:

- **Batch Index**: Used as a mapping index for the MMR state.
- **Latest MMR Block**: Serves as a reference for subsequent updates.
- **Latest MMR Block Hash**: Validates the link between consecutive batches.
- **MMR Root Hash**: Enables block hash inclusion proofs.
- **Leaves Count**: Represents the number of hashes appended to the MMR.
- **IPFS Address**: The IPFS hash for the `.db` file containing the full off-chain MMR state, including all intermediate and leaf hashes.

By incorporating the IPFS storage and validation process into updates, the system ensures tamper-proof data integrity. Public availability of both the on-chain MMR state and IPFS addresses enables anyone to verify the integrity of Fossil's block data.
