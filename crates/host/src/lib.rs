pub mod accumulator;
pub mod proof_generator;
pub mod types;

pub use accumulator::AccumulatorBuilder;
use bytemuck::cast;
use eyre::{eyre, Result};
use methods::{MMR_GUEST_ELF, MMR_GUEST_ID};
use mmr_accumulator::processor_utils::{create_database_file, ensure_directory_exists};
pub use proof_generator::{ProofGenerator, ProofType};
use starknet_crypto::Felt;
use starknet_handler::provider::StarknetProvider;

pub async fn update_mmr_and_verify_onchain(
    db_file: &str,          // Path to the existing SQLite database file
    start_block: u64,       // Start block to update the MMR
    end_block: u64,         // End block to update the MMR
    rpc_url: &str,          // RPC URL for Starknet
    verifier_address: &str, // Verifier contract address
) -> Result<(bool, String)> {
    let mmr_guest_id_bytes: [u8; 32] = cast(MMR_GUEST_ID);

    // Use mmr_guest_id_bytes as needed
    if MMR_GUEST_ELF.is_empty() || mmr_guest_id_bytes == [0; 32] {
        return Err(eyre!(
            "Guest code is not available. Please ensure the guest code is built."
        ));
    }
    // Initialize proof generator
    let proof_generator = ProofGenerator::new(MMR_GUEST_ELF, MMR_GUEST_ID);

    // Initialize accumulator builder
    let mut builder = AccumulatorBuilder::new(db_file, proof_generator, 1024).await?;

    // Update the MMR with new block headers and get the proof calldata
    let (proof_calldata, new_mmr_root_hash) = builder
        .update_mmr_with_new_headers(start_block, end_block)
        .await?;

    let provider = StarknetProvider::new(rpc_url)?;

    // Attempt to verify the Groth16 proof on-chain
    let verification_result = provider
        .verify_groth16_proof_onchain(verifier_address, &proof_calldata)
        .await
        .map_err(|e| eyre!("Failed to verify final Groth16 proof: {}", e))?;

    let verified = *verification_result
        .first()
        .ok_or_else(|| eyre!("Verification result is empty"))?
        == Felt::from(1);

    Ok((verified, new_mmr_root_hash))
}

pub fn get_store_path(db_file: Option<String>) -> Result<String> {
    // Load the database file path from the environment or use the provided argument
    let store_path = if let Some(db_file) = db_file {
        db_file
    } else {
        // Otherwise, create a new database file
        let current_dir = ensure_directory_exists("db-instances")?;
        create_database_file(&current_dir, 0)?
    };

    Ok(store_path)
}
