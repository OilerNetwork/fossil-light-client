---
id: batch-selection
title: Batch Selection
---

To optimize MMR append and proof operations, the size of each MMR is limited to 1024 blocks.

While increasing batch size could reduce on-chain storage costs, it would also slow append and proof generation operations.

Each batch begins at a block number that is a multiple of 1024 (`start_block % 1024 = 0`) and contains at most 1024 blocks.

If a batch cannot be completed due to insufficient blocks, it will be "topped up" during subsequent updates.

Excess blocks from a new update that exceed the current MMR's capacity will start a new MMR.

This setup simplifies mapping block numbers to batch indices using the formula `block_number / 1024`.
