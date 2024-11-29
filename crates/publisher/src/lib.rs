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
pub use proof_generator::{ProofGenerator, ProofType};
use starknet_crypto::Felt;
use starknet_handler::{MmrState, StarknetHandlerError};
use thiserror::Error;
pub use validator::{ValidatorBuilder, ValidatorError};

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
    db_file: &str,
    start_block: u64,
    end_block: u64,
) -> Result<(MmrState, Vec<Felt>), PublisherError> {
    let proof_generator = ProofGenerator::new(MMR_APPEND_ELF, MMR_APPEND_ID, false);
    let mut builder = AccumulatorBuilder::new(db_file, proof_generator, 1024, false).await?;

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

    Ok((new_mmr_state, proof_calldata))
}

pub async fn prove_headers_validity_and_inclusion(
    headers: &Vec<eth_rlp_types::BlockHeader>,
    skip_proof_verification: Option<bool>,
) -> Result<bool, PublisherError> {
    let skip_proof = match skip_proof_verification {
        Some(skip) => skip,
        None => false,
    };
    let validator = ValidatorBuilder::new(skip_proof).await?;

    let result = validator
        .verify_blocks_validity_and_inclusion(headers)
        .await?;

    Ok(result)
}
