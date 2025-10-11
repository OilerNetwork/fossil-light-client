# Initial Accumulation Event

The purpose of the initial accumulation event is to efficiently store all Ethereum block hashes from genesis on Starknet.

These hashes are used to ensure that the data Fossil processes for computations belongs to the canonical Ethereum chain.

Before appending each block hash, integrity and chain validity checks are performed.

To achieve efficient storage and fast append/proof generation, multiple MMRs, each with a maximum size of 1024 blocks, are constructed.

To complement on-chain storage, the full structure of each MMR—including the root hash, element count, and all leaf and intermediate hashes—is stored off-chain in a SQLite database file (`.db`).

Storing all intermediate hashes on-chain is impractical, as the total number of hashes in an MMR exceeds its leaf count.

Each MMR generates a separate `.db` file, which is uploaded to IPFS.

The resulting IPFS storage hash is stored on-chain and included in the MMR state, providing an additional layer of integrity and allowing public verification of the block data used by Fossil.
