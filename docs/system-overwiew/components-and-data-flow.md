---
id: components-and-data-flow
title: Components and Data Flow
---

![Fossil Components and Data Flow](/img/02.png)

- **L1 Message Sender (L1MS)**: An Ethereum L1 smart contract that sends finalized block hashes to Starknet via L1-to-L2 messaging.

- **L1 Message Proxy (L1MP)**: A Starknet smart contract that receives messages from the L1MS and routes them to other contracts in the Fossil ecosystem.

- **Fossil Store (FS)**: A Starknet smart contract that stores the latest finalized block hash and the state of each MMR. It emits events that the Client Updater listens to for triggering updates.

- **Fossil Relayer (FR)**: A Rust binary responsible for calling the `sendFinalizedBlockHashToL2` function in L1MS to relay finalized block hashes to Starknet at predetermined intervals (TBD).

- **Client Updater (CU)**: A Rust binary that listens for events from FS and appends new finalized Ethereum block hashes to the MMRs.

  - **MMR Builder**: A Rust binary that performs the initial accumulation of Ethereum block hashes, starting from a known finalized block (using Solidity opcode for direct access) and traversing backward to genesis. The solidity 0x40 opcode (BLOCK_HASH) is used only once to retrieve the latest finalaside block at the beginning of  the process.

- **Fossil Snark Verifier (FSV)**: A Starknet smart contract that trustlessly verifies Groth16 proofs. It extracts the journal from the serialized proof and sends the verified MMR state to the Fossil Store (FS).
