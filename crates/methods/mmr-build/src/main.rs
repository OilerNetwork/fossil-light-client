// main.rs
use eth_rlp_verify::are_blocks_and_chain_valid;
use risc0_zkvm::guest::env;
use guest_mmr::core::GuestMMR;
use guest_types::{CombinedInput, GuestOutput};
use eth_rlp_types::BlockHeader;

const HOUR_IN_SECONDS: i64 = 3600;

fn main() {
    // Read combined input
    let input: CombinedInput = env::read();
    // Flatten headers for validation
    let flattened_headers: Vec<BlockHeader> = input.headers()
        .iter()
        .flat_map(|(_, headers)| headers.iter())
        .cloned()
        .collect();

    assert!(
        are_blocks_and_chain_valid(&flattened_headers, input.chain_id()),
        "Invalid block headers"
    );

    // Initialize MMR with previous state
    let mut mmr = GuestMMR::new(
        input.mmr_input().initial_peaks(),
        input.mmr_input().elements_count(),
        input.mmr_input().leaves_count(),
    );

    // Append block hashes to MMR
    for (_, batch_headers) in input.headers() {
        for header in batch_headers {
            let block_hash = header.block_hash.clone();
            match mmr.append(block_hash) {
                Ok(_) => {}
                Err(e) => {
                    assert!(false, "MMR append failed: {:?}", e);
                }
            }
        }
    }

    let root_hash = mmr.calculate_root_hash(mmr.get_elements_count()).unwrap();

    let first_header = &input.headers()[0].1[0];
    let last_batch = input.headers().last().expect("No batches found");
    let last_header = last_batch.1.last().expect("No headers in last batch");

    let first_block_number = first_header.number as u64;
    let last_block_number = last_header.number as u64;
    let last_block_hash = last_header.block_hash.clone();

    let first_batch_index = first_block_number / input.batch_size();
    let last_batch_index = last_block_number / input.batch_size();

    assert!(first_batch_index == last_batch_index, "Batch index mismatch");

    // Calculate fee averages for hourly groups
    let mut avg_fees: Vec<(usize, usize, u64)> = Vec::new(); // (timestamp, data_points, avg_fee)

    for (claimed_timestamp, hour_group) in input.headers() {
        if hour_group.is_empty() {
            continue;
        }

        // Verify the claimed timestamp is valid for this group
        let group_timestamps: Vec<i64> = hour_group.iter()
            .filter_map(|header| {
                header.timestamp.as_ref()
                    .and_then(|ts| i64::from_str_radix(ts.trim_start_matches("0x"), 16).ok())
            })
            .collect();

        // Verify all timestamps are within the same hour as claimed_timestamp
        assert!(
            group_timestamps.iter().all(|ts| ts / HOUR_IN_SECONDS == claimed_timestamp / HOUR_IN_SECONDS),
            "Timestamps in group don't belong to claimed hour"
        );

        // Verify claimed_timestamp is exactly on the hour
        assert!(
            claimed_timestamp % HOUR_IN_SECONDS == 0,
            "Claimed timestamp is not exactly on the hour"
        );

        let total_fees: u64 = hour_group
            .iter()
            .filter_map(|header| {
                header.base_fee_per_gas.as_ref()
                    .and_then(|fee| u64::from_str_radix(fee.trim_start_matches("0x"), 16).ok())
            })
            .sum();

        let avg_fee = total_fees / hour_group.len() as u64;
        let data_points = hour_group.len();

        avg_fees.push((*claimed_timestamp as usize, data_points, avg_fee));
    }

    let first_block_parent_hash = if first_batch_index == 0 {
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string()
    } else {
        first_header.parent_hash.clone().expect("Parent hash is missing")
    };

    // Create output with avg_fees
    let output = GuestOutput::new(
        first_batch_index,
        last_block_number,
        last_block_hash,
        root_hash,
        mmr.get_leaves_count(),
        first_block_parent_hash,
        avg_fees,
    );

    // Commit the output
    env::commit(&output);
}