# Key Technologies

- **RISC0 zkVM**: A zero-knowledge virtual machine used to generate cryptographic proofs of computation integrity without exposing private inputs.

- **SNARK Proofs**: Succinct, non-interactive proofs that validate the correctness of computations and data integrity with minimal overhead.

- **Merkle Mountain Range (MMR)**: A tree-like data structure that allows efficient storage and proof generation for large datasets, used here for Ethereum block hashes.

- **Starknet**: A Layer-2 scaling solution for Ethereum that enables scalable and trust-minimized computation and storage.

- **Fossil Postures Database**: A PostgreSQL-based relational database populated by an indexer that tracks all finalized Ethereum blocks. It stores block header data that Fossil retrieves for processing and validation.

- **AWS SQS** (Pitchlake Integration): Used by the Pitchlake Coprocessor (separate repository) for asynchronous request queuing and processing.
