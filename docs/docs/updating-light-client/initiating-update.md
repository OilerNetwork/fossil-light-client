---
id: initiating-update
title: Initiating the Update Process
---

The CU listens for events emitted by the FS contract and triggers the update process upon detecting a new finalized block hash.

1. **Anchor Selection**:

    - The relayed block hash serves as an anchor for the update.

    - The CU queries the latest MMR block stored in the FS contract to determine the range of blocks requiring an update: `(latest_mmr_block, latest_relayed_block)`.

2. **Retrieving Block Headers**:

    - The CU fetches the required block headers for the computed range from the PostgreSQL database.

    - The last block header in the range is validated by checking that its block hash matches the latest relayed block hash from L1.
