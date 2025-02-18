---
id: relaying-block-hashes
title: Relaying Finalized Block Hashes
---

Every several hours, a finalized block hash is sent from L1 to L2 on Starknet via the Fossil Relayer. Finalized blocks are considered "safe" as they are not subject to chain reorganizations.

- The finalized block hash is sourced directly on-chain, guaranteeing its validity and inclusion in Ethereum’s canonical chain.
- Once the block hash and its block number are stored in the Fossil Store (FS) contract, an event is emitted to signal that a new finalized block has been relayed.
