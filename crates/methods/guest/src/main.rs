// main.rs
use eth_rlp_verify::are_blocks_and_chain_valid;
use risc0_zkvm::guest::env;
mod guest_mmr;
use guest_mmr::GuestMMR;
use guest_types::{CombinedInput, GuestOutput};

fn main() {
    // Read combined input
    let input: CombinedInput = env::read();

    // Verify previous batch proofs
    for proof in input.mmr_input().previous_proofs() {
        // Verify each previous proof
        proof
            .receipt()
            .verify(proof.method_id())
            .expect("Invalid previous proof");
    }

    // Verify block headers
    assert!(
        are_blocks_and_chain_valid(&input.headers()),
        "Invalid block headers"
    );

    // Initialize MMR with previous state
    let mut mmr = GuestMMR::new(
        input.mmr_input().initial_peaks(),
        input.mmr_input().elements_count(),
        input.mmr_input().leaves_count(),
    );

    let mut append_results = Vec::new();
    // Append block hashes to MMR
    for (_, header) in input.headers().iter().enumerate() {
        let block_hash = header.block_hash.clone();
        match mmr.append(block_hash) {
            Ok(result) => {
                append_results.push(result);
            }
            Err(e) => {
                assert!(false, "MMR append failed: {:?}", e);
            }
        }
    }

    // Get final peaks
    let final_peaks = match mmr.get_peaks(Default::default()) {
        Ok(peaks) => peaks,
        Err(e) => {
            assert!(false, "Failed to get final peaks: {:?}", e);
            vec![] // This line will never be reached due to assert
        }
    };

    // Create output
    let output = GuestOutput::new(
        final_peaks,
        mmr.get_elements_count(),
        mmr.get_leaves_count(),
        append_results,
    );

    // Commit the output
    env::commit(&output);
}
