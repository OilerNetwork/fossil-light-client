---
id: intro
title: Updating the Light Client
---

As new blocks are produced by Ethereum, their hashes must be added to the Fossil Light Client (FLC). The client maintains two key block numbers:

- The latest processed events block
- The latest processed MMR block

The update process follows these steps:

1. **Event Monitoring**: The client periodically polls Starknet for new `LatestBlockhashFromL1Stored` events from the Fossil Store contract.

2. **Block Range Processing**: When new events are found, the client:
   - Starts from the last processed block + 1
   - Processes blocks up to either:
     - The latest available block, or
     - A configured maximum number of blocks per run

3. **MMR Updates**: For each batch of new blocks:
   - Fetches the latest relayed block hash from L1
   - Updates the MMR structure with new block hashes
   - Generates and verifies proofs on-chain
   - Updates the MMR state in the Starknet store

This forward-moving process ensures the FLC remains synchronized with Ethereum's canonical chain, processing new blocks as they become available and are finalized on L1.

The client runs continuously, with a configurable polling interval, to maintain up-to-date state between Ethereum and Starknet.
