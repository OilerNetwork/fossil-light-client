---
id: database-setup
title: Database Setup and Access
---

The Fossil Headers Database schema represents all fields of an Ethereum block header. The database is populated using Fossil's indexer, which operates independently of the FLC. Data is fetched via RPC calls to a Nethermind Ethereum client.

Stored fields provide the raw data Fossil requires for computations and the values necessary for integrity verification. The Keccak hash of each block is recomputed from the header fields during validation to ensure data consistency.

Refer to the **Entity Relationship Diagram (ERD)** here: [Fossil Headers DB ERD](https://www.notion.so/Fossil-Headers-DB-ERD-15a360fc38d080e1acf2c2035f25a987?pvs=21).
