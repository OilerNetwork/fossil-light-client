---
id: storage-and-retrieval
title: Fee Storage and Retrieval
---

The Fossil Store contract maintains a mapping of hourly gas fee data that can be efficiently queried.

## Storage Structure

```cairo
#[starknet::storage_node]
pub struct AvgFees {
    data_points: u64,
    avg_fee: u64,
}

// Storage mapping
avg_fees: Map<u64, AvgFees>
```

## Fee Updates

When new fee data is received:

1. **For New Hours**:
   - Data points and average fee are stored directly

2. **For Existing Hours**:
   - New average is calculated weighted by data points:
   ```cairo
   let new_data_points = existing_points + new_points;
   let new_avg_fee = (existing_fee * existing_points + new_fee * new_points) / new_data_points;
   ```

## Data Retrieval

1. **Single Hour Query**:
   ```cairo
   fn get_avg_fee(timestamp: u64) -> u64 {
       assert!(timestamp % HOUR_IN_SECONDS == 0, "Timestamp must be a multiple of 3600");
       let curr_state = self.avg_fees.entry(timestamp);
       curr_state.avg_fee.read()
   }
   ```

2. **Range Query**:
   ```cairo
   fn get_avg_fees_in_range(
       start_timestamp: u64, 
       end_timestamp: u64
   ) -> Array<u64>
   ```
   - Returns array of hourly fees
   - Timestamps must be multiples of 3600
   - Start must be less than or equal to end timestamp

## Validation Rules

1. All timestamps must be hour-aligned (multiple of 3600 seconds)
2. Range queries must have valid start/end ordering
3. Data points are preserved to maintain accurate weighted averages 