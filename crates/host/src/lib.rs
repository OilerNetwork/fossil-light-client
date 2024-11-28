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
use mmr_utils::MMRUtilsError;
pub use proof_generator::{ProofGenerator, ProofType};
use starknet_crypto::Felt;
use starknet_handler::{MmrState, StarknetHandlerError};
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

pub async fn prove_mmr_update(
    db_file: &str,
    start_block: u64,
    end_block: u64,
) -> Result<(MmrState, Vec<Felt>), HostError> {
    let proof_generator = ProofGenerator::new(MMR_GUEST_ELF, MMR_GUEST_ID, false);
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
