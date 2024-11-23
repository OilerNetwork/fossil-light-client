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
use starknet_handler::{provider::StarknetProvider, MmrState, StarknetHandlerError};
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
    db_file: &str,
    start_block: u64,
    end_block: u64,
    rpc_url: &str,
    verifier_address: &str,
) -> Result<(bool, MmrState), HostError> {
    let proof_generator = ProofGenerator::new(MMR_GUEST_ELF, MMR_GUEST_ID);
    let mut builder = AccumulatorBuilder::new(db_file, proof_generator, 1024).await?;

    tracing::debug!(
        db_file,
        start_block,
        end_block,
        "Starting MMR update and proof generation"
    );

    let (proof_calldata, new_mmr_state) = builder
        .update_mmr_with_new_headers(start_block, end_block)
        .await?;
    tracing::debug!(
        start_block,
        end_block,
        "Successfully generated proof for block range"
    );

    let provider = StarknetProvider::new(rpc_url)?;
    tracing::debug!(verifier_address, "Submitting proof for verification");

    let verification_result = provider
        .verify_groth16_proof_onchain(verifier_address, &proof_calldata)
        .await?;

    let verified = *verification_result
        .first()
        .ok_or_else(|| HostError::VerificationError)?
        == Felt::from(1);

    if verified {
        tracing::info!("Proof verification successful on-chain");
    } else {
        tracing::warn!("Proof verification failed on-chain");
    }

    Ok((verified, new_mmr_state))
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
