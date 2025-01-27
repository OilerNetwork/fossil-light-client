---
id: generating-proofs
title: Generating the Proofs (RISC0 zkVM and SNARK)
---

Integrity verification and MMR append operations are executed inside a RISC0 zkVM, generating cryptographic guarantees for correctness without requiring trust in off-chain computations.

For each batch, a Groth16 proof is produced. This proof includes the Groth16 verification key and the RISC0 receipt, serialized into a `felt252` array using the Garaga Rust SDK. The serialized proof is then sent to the Fossil Snark Verifier (FSV) on Starknet for validation.
