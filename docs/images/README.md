# Documentation Images

This directory contains images and diagrams used in the Fossil Light Client documentation.

## Required Images

### fossil-architecture.png

**Location**: `docs/images/fossil-architecture.png`

**Description**: Complete Fossil system architecture diagram showing:
- Fossil Light Client (Relayer and Client components)
- Headers DB Indexer
- IPFS storage
- MMR Builder
- Ethereum L1 (L1Message Sender)
- Starknet L2 (L1Message Proxy, Fossil Verifier, Store)
- Fossil API and Proving Service
- Fossil Message Handler
- SQS Queue
- Pitchlake Backend integration
- Pitchlake Verifier and Vault

**Usage**: Referenced in:
- `docs/intro.md` - Main documentation index
- `docs/technical-specification.md` - Technical specification overview
- `docs/system-overview/components-and-data-flow.md` - Components and data flow documentation

**Instructions**: Place the architecture diagram image file in this directory as `fossil-architecture.png`.
