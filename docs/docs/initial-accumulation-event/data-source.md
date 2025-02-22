---
id: data-source
title: "Data Source: Ethereum Block Headers"
---

Ethereum block headers from both mainnet and Sepolia testnet are stored in separate PostgreSQL databases hosted on AWS.

Using a relational database enables faster access to larger block ranges compared to querying directly from an Ethereum node.

This design choice ensures the system can efficiently fulfill requests for extensive datasets.

Maintaining separate databases for mainnet and Sepolia serves distinct purposes:

- **Mainnet**: Provides data for production use.

- **Sepolia**: Functions as a staging environment to test changes and improvements before deployment to production.

Additionally, direct access to Ethereum block hashes using Solidity opcode `0x40` (BLOCKHASH) ensures the hashes belong to the canonical chain. The latest block hash, which serves as the starting point for accumulation, is obtained directly from the Ethereum blockchain using the following smart contract:

```solidity
contract BlockHashFetcher {
        function getBlockHash() external view returns (uint256 blockNumber, bytes32 blockHash) {
            require(block.number > 100, "Block number must be greater than 100");
            blockNumber = block.number - 100;
            blockHash = blockhash(blockNumber);
            return (blockNumber, blockHash);
        }
    }
```

This contract fetches the hash of a block that is 100 blocks behind the current block, ensuring we're working with finalized blocks as a secure anchor point for starting the accumulation process.
