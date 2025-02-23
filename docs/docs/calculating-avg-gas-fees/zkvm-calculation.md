---
id: zkvm-calculation
title: ZK Proof of Fee Calculation
---

The average gas fees are calculated inside a RISC Zero zkVM guest program, ensuring the integrity and correctness of the calculations.

## Guest Output Structure

The guest program outputs both MMR-related data and fee calculations:

```rust
pub struct GuestOutput {
    batch_index: u64,
    latest_mmr_block: u64,
    latest_mmr_block_hash: String,
    root_hash: String,
    leaves_count: usize,
    first_block_parent_hash: String,
    avg_fees: Vec<(usize, usize, u64)>, // (timestamp, data_points, avg_fee)
}
```

The `avg_fees` vector contains tuples of:
- `timestamp`: Hour-aligned timestamp (multiple of 3600)
- `data_points`: Number of blocks in this hour
- `avg_fee`: Calculated average gas fee for the hour

## Verification Process

1. **Input Processing**:
   - Block headers are grouped by hour
   - Each group's timestamp is normalized to hour boundaries
   - Gas fees are extracted from each block header

2. **Fee Calculation**:
   - For each hour group:
     - Sum all gas fees in the group
     - Count number of blocks (data points)
     - Calculate average by dividing sum by count
     - Store result with normalized timestamp

3. **State Update**:
   The Fossil Store contract receives the proof journal containing:
   - MMR state updates
   - Average fee data for each hour
   - IPFS hash for off-chain data

## Contract Integration

The store contract's `update_store_state` function processes the fee data:

```cairo
fn update_store_state(
    ref self: ContractState,
    journal: verifier::Journal,
    avg_fees: Span<verifier::AvgFees>,
    ipfs_hash: ByteArray,
)
```

For each hour's fee data:
- If no existing data: store directly
- If existing data: calculate weighted average based on data points
- Update both average fee and data point count

This ensures that all fee calculations are verified through the ZK proof system before being stored on Starknet. 