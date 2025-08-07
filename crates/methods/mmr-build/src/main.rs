// main.rs
use eth_rlp_types::BlockHeader;
use eth_rlp_verify::are_blocks_and_chain_valid;
use guest_fixed_utils::{UFixedPoint123x128, StorePacking};
use guest_mmr::core::GuestMMR;
use guest_types::{CombinedInput, GuestOutput};
use risc0_zkvm::guest::env;

const HOUR_IN_SECONDS: i64 = 3600;

fn main() {
    eprintln!("DEBUG: Guest program started!");
    
    // Read combined input
    let input: CombinedInput = env::read();
    
    eprintln!("DEBUG: Input read successfully, headers count: {}", input.headers().len());
    // Flatten headers for validation
    let flattened_headers: Vec<BlockHeader> = input
        .headers()
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

    assert!(
        first_batch_index == last_batch_index,
        "Batch index mismatch"
    );

    // Calculate fee averages for hourly groups using fixed-point arithmetic
    let mut avg_fees: Vec<(usize, usize, String)> = Vec::new(); // (timestamp, data_points, avg_fee_felt)

    env::log(&format!("DEBUG: Starting fee calculation for {} hour groups", input.headers().len()));

    for (claimed_timestamp, hour_group) in input.headers() {
        env::log(&format!("DEBUG: Processing hour group with claimed_timestamp: {}, headers: {}", claimed_timestamp, hour_group.len()));
        
        if hour_group.is_empty() {
            env::log("DEBUG: Skipping empty hour group");
            continue;
        }

        // Verify the claimed timestamp is valid for this group
        let group_timestamps: Vec<i64> = hour_group
            .iter()
            .filter_map(|header| {
                let ts_result = header
                    .timestamp
                    .as_ref()
                    .and_then(|ts| i64::from_str_radix(ts.trim_start_matches("0x"), 16).ok());
                env::log(&format!("DEBUG: Header timestamp: {:?} -> parsed: {:?}", header.timestamp, ts_result));
                ts_result
            })
            .collect();

        env::log(&format!("DEBUG: Group timestamps: {:?}", group_timestamps));

        // Verify all timestamps are within the same hour as claimed_timestamp
        assert!(
            group_timestamps
                .iter()
                .all(|ts| ts / HOUR_IN_SECONDS == claimed_timestamp / HOUR_IN_SECONDS),
            "Timestamps in group don't belong to claimed hour"
        );

        // Verify claimed_timestamp is exactly on the hour
        assert!(
            claimed_timestamp % HOUR_IN_SECONDS == 0,
            "Claimed timestamp is not exactly on the hour"
        );

        // Calculate total fees using fixed-point arithmetic
        let mut total_fees = UFixedPoint123x128::from(0.0);
        let mut valid_fee_count = 0;

        env::log(&format!("DEBUG: Starting fee calculation for {} headers", hour_group.len()));

        for (idx, header) in hour_group.iter().enumerate() {
            env::log(&format!("DEBUG: Header {}: base_fee_per_gas = {:?}", idx, header.base_fee_per_gas));
            
            if let Some(fee_str) = &header.base_fee_per_gas {
                env::log(&format!("DEBUG: Parsing fee string: '{}'", fee_str));
                
                if let Ok(fee) = u64::from_str_radix(fee_str.trim_start_matches("0x"), 16) {
                    env::log(&format!("DEBUG: Parsed fee: {} wei", fee));
                    
                    // Convert fee to fixed-point and add to total
                    let fee_fixed = UFixedPoint123x128::from(fee as f64);
                    env::log(&format!("DEBUG: Fee as fixed-point: integer={}, fractional={}", 
                        fee_fixed.get_integer(), fee_fixed.get_fractional()));
                    
                    total_fees = UFixedPoint123x128::from(
                        (total_fees.get_integer() as f64 + fee_fixed.get_integer() as f64) +
                        ((total_fees.get_fractional() as f64 + fee_fixed.get_fractional() as f64) / 2f64.powi(128))
                    );
                    valid_fee_count += 1;
                    
                    env::log(&format!("DEBUG: Running total: integer={}, fractional={}, count={}", 
                        total_fees.get_integer(), total_fees.get_fractional(), valid_fee_count));
                } else {
                    env::log(&format!("DEBUG: Failed to parse fee string: '{}'", fee_str));
                }
            } else {
                env::log(&format!("DEBUG: No base_fee_per_gas in header {}", idx));
            }
        }

        env::log(&format!("DEBUG: Final totals - valid_fee_count: {}, total_fees: integer={}, fractional={}", 
            valid_fee_count, total_fees.get_integer(), total_fees.get_fractional()));

        // Calculate average fee using fixed-point division
        let count_fixed = UFixedPoint123x128::from(valid_fee_count as f64);
        let avg_fee_fixed = if valid_fee_count > 0 {
            total_fees / count_fixed
        } else {
            env::log("DEBUG: No valid fees found, using zero");
            UFixedPoint123x128::from(0.0)
        };

        env::log(&format!("DEBUG: Average fee fixed-point: integer={}, fractional={}", 
            avg_fee_fixed.get_integer(), avg_fee_fixed.get_fractional()));

        // Pack the fixed-point value into a Felt
        let avg_fee_felt = UFixedPoint123x128::pack(avg_fee_fixed).to_hex_string();
        
        env::log(&format!("DEBUG: Packed avg_fee_felt: '{}'", avg_fee_felt));

        avg_fees.push((*claimed_timestamp as usize, valid_fee_count, avg_fee_felt.clone()));
        
        env::log(&format!("DEBUG: Added to avg_fees: timestamp={}, count={}, felt='{}'", 
            *claimed_timestamp as usize, valid_fee_count, avg_fee_felt));
    }

    env::log(&format!("DEBUG: Final avg_fees vector length: {}", avg_fees.len()));
    for (i, (timestamp, count, felt)) in avg_fees.iter().enumerate() {
        env::log(&format!("DEBUG: avg_fees[{}]: timestamp={}, count={}, felt='{}'", i, timestamp, count, felt));
    }

    let first_block_parent_hash = if first_batch_index == 0 {
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string()
    } else {
        first_header
            .parent_hash
            .clone()
            .expect("Parent hash is missing")
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
