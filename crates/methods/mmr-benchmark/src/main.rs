// main.rs
use eth_rlp_verify::are_blocks_and_chain_valid;
use risc0_zkvm::guest::env;
use guest_mmr::core::GuestMMR;
use eth_rlp_types::BlockHeader;

const CHAIN_ID: u64 = 11155111;
fn main() {
    // Read combined input
    let input: Vec<BlockHeader> = env::read();
    // Verify block headers
    assert!(
        are_blocks_and_chain_valid(&input, CHAIN_ID),
        "Invalid block headers"
    );
    // Initialize MMR with previous state
    let mut mmr = GuestMMR::new_empty();
    // let mut append_results = Vec::new();
    // Append block hashes to MMR
    for header in input.iter() {
        let block_hash = header.block_hash.clone();
        match mmr.append(block_hash) {
            Ok(_) => {}
            Err(e) => {
                assert!(false, "MMR append failed: {:?}", e);
            }
        }
    }    

    let elements_count = mmr.get_elements_count();

    // Create output
    let output = mmr.calculate_root_hash(elements_count).unwrap();
    // Commit the output
    env::commit(&output);
}