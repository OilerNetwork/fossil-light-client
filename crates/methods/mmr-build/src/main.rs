// main.rs
use eth_rlp_verify::are_blocks_and_chain_valid;
use risc0_zkvm::guest::env;
use guest_mmr::core::GuestMMR;
use guest_types::{CombinedInput, GuestOutput};

fn main() {
    // Read combined input
    let input: CombinedInput = env::read();
    // Verify block headers
    assert!(
        are_blocks_and_chain_valid(&input.headers(), input.chain_id()),
        "Invalid block headers"
    );
    // Initialize MMR with previous state
    let mut mmr = GuestMMR::new(
        input.mmr_input().initial_peaks(),
        input.mmr_input().elements_count(),
        input.mmr_input().leaves_count(),
    );
    // let mut append_results = Vec::new();
    // Append block hashes to MMR
    for (_, header) in input.headers().iter().enumerate() {
        let block_hash = header.block_hash.clone();
        match mmr.append(block_hash) {
            Ok(_) => {}
            Err(e) => {
                assert!(false, "MMR append failed: {:?}", e);
            }
        }
    }

    let root_hash = mmr.calculate_root_hash(mmr.get_elements_count()).unwrap();

    let first_header = input.headers().first().unwrap();
    let last_header = input.headers().last().unwrap();

    let first_block_number = first_header.number as u64;
    let last_block_number = last_header.number as u64;
    let last_block_hash = last_header.block_hash.clone();

    let first_batch_index = first_block_number / input.batch_size();
    let last_batch_index = last_block_number / input.batch_size();

    assert!(first_batch_index == last_batch_index, "Batch index mismatch");

    let first_block_parent_hash = if first_batch_index == 0 {
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string()
    } else {
        first_header.parent_hash.clone().expect("Parent hash is missing")
    };

    // Create output
    let output = GuestOutput::new(
        first_batch_index,
        last_block_number,
        last_block_hash,
        root_hash,
        mmr.get_leaves_count(),
        first_block_parent_hash,
    );
    // Commit the output
    env::commit(&output);
}