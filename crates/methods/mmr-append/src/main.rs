// main.rs
use eth_rlp_verify::are_blocks_and_chain_valid;
use risc0_zkvm::guest::env;
use guest_mmr::core::GuestMMR;
use guest_types::{CombinedInput, GuestOutput};

fn main() {
    // Read combined input
    let input: CombinedInput = env::read();

    // Only verify proofs if skip_proof_verification is false
    if !input.skip_proof_verification() {
        if let Some(proofs) = input.mmr_input().previous_proofs() {
            for proof in proofs {
                proof
                    .receipt()
                    .verify(proof.method_id())
                    .expect("Invalid previous proof");
            }
        }
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

    let root_hash = mmr.calculate_root_hash(mmr.get_elements_count()).unwrap();

    // Create output
    let output = GuestOutput::new(
        root_hash,
        mmr.get_elements_count(),
        mmr.get_leaves_count(),
        mmr.get_all_hashes(),
        append_results,
    );
    // Commit the output
    env::commit(&output);
}
