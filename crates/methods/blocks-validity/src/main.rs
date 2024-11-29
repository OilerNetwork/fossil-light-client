// main.rs
use eth_rlp_verify::are_blocks_and_chain_valid;
use risc0_zkvm::guest::env;
use guest_mmr::core::GuestMMR;
use guest_types::BlocksValidityInput;

fn main() {
    // Read combined input
    let input: BlocksValidityInput = env::read();

    // Verify block headers
    if !are_blocks_and_chain_valid(&input.headers()) {
        env::commit(&false);
    }
    // Initialize MMR with previous state
    let mmr = GuestMMR::new(
        input.mmr_input().initial_peaks(),
        input.mmr_input().elements_count(),
        input.mmr_input().leaves_count(),
    );
    // Append block hashes to MMR
    for (i, header) in input.headers().iter().enumerate() {
        let block_hash = header.block_hash.clone();
        let proof = mmr.get_proof(input.hash_indexes()[i]).unwrap();
        if !mmr.verify_proof(proof, block_hash, None).unwrap() {
            env::commit(&false);
        }
    }

    env::commit(&true);
}
