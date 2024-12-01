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
pub mod validator;

pub use accumulator::AccumulatorBuilder;
use methods::{MMR_APPEND_ELF, MMR_APPEND_ID};
use mmr_utils::MMRUtilsError;
pub use proof_generator::ProofGenerator;
use starknet_crypto::Felt;
use starknet_handler::{MmrState, StarknetHandlerError};
use thiserror::Error;
pub use validator::{ValidatorBuilder, ValidatorError};

// Import CombinedInput for ProofGenerator
use guest_types::CombinedInput;

#[derive(Error, Debug)]
pub enum PublisherError {
    #[error("Verification result is empty")]
    VerificationError,
    #[error("Accumulator error: {0}")]
    Accumulator(#[from] AccumulatorError),
    #[error("StarknetHandler error: {0}")]
    StarknetHandler(#[from] StarknetHandlerError),
    #[error("MMRUtils error: {0}")]
    MMRUtils(#[from] MMRUtilsError),
    #[error("Headers Validator error: {0}")]
    Validator(#[from] ValidatorError),
}

pub async fn prove_mmr_update(
    rpc_url: &String,
    verifier_address: &String,
    account_private_key: &String,
    account_address: &String,
    batch_size: u64,
    start_block: u64,
    end_block: u64,
    skip_proof_verification: bool,
) -> Result<(MmrState, Vec<Felt>), PublisherError> {
    // Initialize ProofGenerator with the correct type parameter and arguments
    let proof_generator = ProofGenerator::<CombinedInput>::new(
        MMR_APPEND_ELF,
        MMR_APPEND_ID,
        false, // skip_seal_verification
    );

    // Update AccumulatorBuilder::new call with new parameters
    let mut builder = AccumulatorBuilder::new(
        rpc_url,
        verifier_address,
        account_private_key,
        account_address,
        proof_generator,
        batch_size,
        skip_proof_verification,
    )
    .await?;

    tracing::debug!(
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

    Ok((new_mmr_state, proof_calldata))
}

pub async fn prove_headers_validity_and_inclusion(
    headers: &Vec<eth_rlp_types::BlockHeader>,
    batch_size: u64,
    skip_proof_verification: Option<bool>,
) -> Result<bool, PublisherError> {
    let skip_proof = skip_proof_verification.unwrap_or(false);
    let validator = ValidatorBuilder::new(batch_size, skip_proof).await?;

    let result = validator
        .verify_blocks_validity_and_inclusion(headers)
        .await?;

    Ok(result)
}
