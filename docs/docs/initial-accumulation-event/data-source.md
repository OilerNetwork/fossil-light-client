---
id: data-source
title: "Data Source: Ethereum Block Headers (Mainnet & Sepolia)"
---

Ethereum block headers from both mainnet and Sepolia testnet are stored in separate PostgreSQL databases hosted on AWS. Using a relational database enables faster access to larger block ranges compared to querying directly from an Ethereum node. This design choice ensures the system can efficiently fulfill requests for extensive datasets.

Maintaining separate databases for mainnet and Sepolia serves distinct purposes:

- **Mainnet**: Provides data for production use.
- **Sepolia**: Functions as a staging environment to test changes and improvements before deployment to production.

Additionally, direct access to Ethereum block hashes using Solidity opcode `0x40` (BLOCKHASH) ensures the hashes belong to the canonical chain. This opcode is only used once to retrieve the latest block hash, which acts as an anchor for the initial accumulation. (clarify)
