---
id: subdividing-in-batches
title: Subdividing into Batches
---

To ensure efficient updates, the block range is subdivided into batches that adhere to the following rules:

- The first block number of each batch is a multiple of 1024 (`start_block % 1024 = 0`).

- The last block number in the batch is `start_block + 1023`.

For incomplete batches (i.e., those with fewer than 1024 leaves):

- The MMR state for that batch is fetched from the FS contract on-chain.

- Using the on-chain IPFS address for the batch, the `.db` file containing the full off-chain MMR state is retrieved from IPFS.

- The MMR root from the IPFS file is recomputed and compared with the on-chain root to ensure the integrity of the off-chain data.

- If the validation succeeds, the incomplete batch is updated in the zkVM, and the MMR is completed by adding the remaining leaves.

- After the update, a new `.db` file representing the updated MMR state is saved to IPFS, and its new IPFS address is added to the on-chain state for the batch.

If the update spans multiple batches, multiple SNARK proofs will be generated and verified.
