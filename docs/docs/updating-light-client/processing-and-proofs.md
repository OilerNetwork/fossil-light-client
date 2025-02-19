---
id: processing-and-proofs
title: Processing and Proof Generation
---

The update process within the RISC0 zkVM follows the same steps as the initial accumulation event:

- Each block hash is recomputed using the Keccak hash function to validate its integrity.

- Parent-child links between blocks are checked to ensure chain validity.

For each batch, the RISC0 zkVM generates a Groth16 proof.

This proof guarantees the correctness of the update operations without relying on off-chain trust.
