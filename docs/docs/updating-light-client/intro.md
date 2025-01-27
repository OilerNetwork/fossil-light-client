---
id: intro
title: Updating the Light Client
---

As new blocks are produced by Ethereum, their hashes must be added to the Fossil Light Client (FLC). This process is twofold: finalized block hashes are periodically relayed from Ethereum L1 to Starknet, and the Client Updater (CU) uses these relayed hashes to update the MMRs. This ensures the FLC remains synchronized with Ethereum's canonical chain.
