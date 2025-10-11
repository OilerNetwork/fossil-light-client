---
id: intro
title: Gas Fee Calculation Overview
---

The Fossil Light Client includes a system for calculating and storing average gas fees from Ethereum blocks. This data is processed in hourly intervals and stored on Starknet for efficient retrieval.

## Calculation Timing

Gas fee calculations are integrated into two main operations:
1. Initial MMR accumulation
2. Light client updates

This integration is intentional and optimizes for both cost and performance:
- Block header validation, which is the most computationally intensive operation, is already performed during these operations
- Calculating fees separately would require re-validating the same headers
- By combining fee calculation with existing validation, we significantly reduce:
  - ZK proving costs
  - Computation time

## Key Components

1. **Block Header Processing**:
   - Block headers are grouped by hour
   - Each group contains headers with timestamps within the same hour
   - A representative timestamp is calculated for each group (hour * 3600 seconds)

2. **Fee Storage Structure**:
   - Fees are stored in the Fossil Store contract
   - Each hourly entry contains:
     - Number of data points
     - Average fee for that hour
   - Timestamps must be multiples of 3600 (seconds in an hour)

3. **Data Flow**:
   - Block headers → Hourly groups → ZK proof generation → Starknet storage
   - Updates are atomic and verified through the proof system
   - Historical data is preserved and can be queried by timestamp range 