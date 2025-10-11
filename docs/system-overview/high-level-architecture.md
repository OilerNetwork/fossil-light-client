# High-Level Architecture

The Fossil Light Client consists of two main components: the Merkle Mountain Range (MMR) Builder and the Light Client updater.

- **MMR Builder**: Responsible for storing all Ethereum block hashes, from a recently finalized block to genesis, in a compact and efficient manner. This data is subsequently stored on Starknet to serve as a trustless reference for block data integrity. The MMR Builder runs only once before the Light Client is started, providing the base of historical block hashes from Ethereum's canonical chain on which the Light Client will add new hashes.

- **Light Client**: Ensures Fossil stays synchronized with Ethereum by continuously adding new finalized block hashes to the MMRs as they are produced.

The system guarantees the integrity of appended blocks by verifying their hashes using direct access to Ethereum block hashes sourced at the smart contract level on L1.

These "anchor" hashes are used to validate that the appended hashes represent legitimate Ethereum blocks.

The verification and MMR append operations are executed inside a RISC0 zkVM, which produces a SNARK proof for each update.

After proof verification on Starknet, the updated MMR state is stored in the Fossil Store (FS) smart contract, serving as an integrity reference for downstream computations.
