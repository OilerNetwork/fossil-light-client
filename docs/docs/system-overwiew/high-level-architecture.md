---
id: high-level-architecture
title: High-Level Architecture
---

The Fossil Light Client (FLC) consists of two main components: the Merkle Mountain Range (MMR) Builder and the Client Updater (CU).

- **MMR Builder**: The MMR Builder is responsible for storing all Ethereum block hashes, from genesis to a recently finalized block, in a compact and efficient manner. This data is subsequently stored on Starknet to serve as a trustless reference for block data integrity.

The MMR Builder runs only once before the Light Client is started, it will provide the base of historical blocks hashes of Ethereum canonical chain on which the FLC will add new hashes.

- **Client Updater (CU)**: The Client Updater ensures the FLC stays synchronized with Ethereum by adding new finalized block hashes to the MMRs as they are produced.

The FLC guarantees the integrity of appended blocks by verifying their hashes using direct access to Ethereum block hashes sourced at the smart contract level.

These "anchor" hashes are used to validate that the appended hashes represent legitimate Ethereum blocks.

The verification and MMR append operations are executed inside a RISC0 zkVM, which produces a SNARK proof for each update.

After proof verification on Starknet, the updated MMR state is stored in the Fossil Store (FS) smart contract, serving as an integrity reference for downstream computations.
