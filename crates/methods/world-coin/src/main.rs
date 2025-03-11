use eth_rlp_verify::are_blocks_and_chain_valid;
use guest_types::{WorldCoinInput, WorldCoinOutput};
use risc0_zkvm::guest::env;

fn main() {
    let input: WorldCoinInput = env::read();

    let header = input.header();
    let chain_id = input.chain_id();

    assert!(
        are_blocks_and_chain_valid(&[header.clone()], chain_id),
        "Invalid block headers"
    );

    let output = WorldCoinOutput::new(
        header.number as u64,
        header.block_hash.clone(),
        header.state_root.clone().unwrap(),
    );

    env::commit(&output);
}
