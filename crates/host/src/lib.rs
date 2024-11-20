#![deny(unused_crate_dependencies)]
use accumulator::AccumulatorError;
use clap as _;
use common as _;
use risc0_groth16 as _;
use tracing as _;

pub mod accumulator;
pub mod db_access;
pub mod proof_generator;
pub mod types;

pub use accumulator::AccumulatorBuilder;
use methods::{MMR_GUEST_ELF, MMR_GUEST_ID};
use mmr_utils::{create_database_file, ensure_directory_exists, MMRUtilsError};
pub use proof_generator::{ProofGenerator, ProofType};
use starknet_crypto::Felt;
use starknet_handler::provider::StarknetProvider;
use starknet_handler::StarknetHandlerError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HostError {
    #[error("Verification result is empty")]
    VerificationError,
    #[error("Accumulator error: {0}")]
    Accumulator(#[from] AccumulatorError),
    #[error("StarknetHandler error: {0}")]
    StarknetHandler(#[from] StarknetHandlerError),
    #[error("MMRUtils error: {0}")]
    MMRUtils(#[from] MMRUtilsError),
}

pub async fn update_mmr_and_verify_onchain(
    db_file: &str,          // Path to the existing SQLite database file
    start_block: u64,       // Start block to update the MMR
    end_block: u64,         // End block to update the MMR
    rpc_url: &str,          // RPC URL for Starknet
    verifier_address: &str, // Verifier contract address
) -> Result<(bool, String), HostError> {
    // Initialize proof generator
    let proof_generator = ProofGenerator::new(MMR_GUEST_ELF, MMR_GUEST_ID);

    // Initialize accumulator builder
    let mut builder = AccumulatorBuilder::new(db_file, proof_generator, 1024).await?;

    tracing::info!(
        "Publisher received proving request for blocks from {} to {}",
        start_block,
        end_block
    );
    // Update the MMR with new block headers and get the proof calldata
    let (proof_calldata, new_mmr_root_hash) = builder
        .update_mmr_with_new_headers(start_block, end_block)
        .await?;
    tracing::info!("Updated MMR with new block headers and got proof calldata");

    let provider = StarknetProvider::new(rpc_url)?;

    tracing::info!("Verifying proof onchain...");
    let verification_result = provider
        .verify_groth16_proof_onchain(verifier_address, &proof_calldata)
        .await?;

    let verified = *verification_result
        .first()
        .ok_or_else(|| HostError::VerificationError)?
        == Felt::from(1);
    tracing::info!("Proof verification successful");

    Ok((verified, new_mmr_root_hash))
}

pub fn get_store_path(db_file: Option<String>) -> Result<String, HostError> {
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
