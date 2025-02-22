---
id: grouping-headers
title: Block Header Grouping
---

Block headers are grouped by hour to calculate average gas fees efficiently. This process ensures that fee data is organized in consistent time intervals.

## Grouping Process

1. **Timestamp Extraction**:
   ```rust
   let timestamp = header
       .timestamp
       .as_ref()
       .and_then(|ts| i64::from_str_radix(ts.trim_start_matches("0x"), 16).ok())
       .unwrap_or_default();
   ```

2. **Hour Calculation**:
   ```rust
   let hour = timestamp / 3600;
   ```

3. **Group Management**:
   - Headers are collected into groups based on their hour
   - Each group maintains its original block headers
   - A representative timestamp is assigned (hour * 3600)

## Group Structure

Each group contains:
- Representative timestamp (aligned to hour boundary)
- Vector of block headers within that hour
- Metadata including:
  - Block number range
  - Timestamp range
  - Number of headers in group

## Logging and Verification

The system logs detailed information about each group:
- Group size
- Block number range
- Timestamp range
- Representative timestamp

This information helps verify correct grouping and diagnose any issues in the fee calculation process. 