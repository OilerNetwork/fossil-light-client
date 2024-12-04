use common::UtilsError;
use mmr::{InStoreTableError, MMRError, StoreError};
use mmr_utils::MMRUtilsError;
use thiserror::Error;

use crate::core::ProofGeneratorError;

#[derive(Error, Debug)]
pub enum PublisherError {
    #[error("Verification result is empty")]
    VerificationError,
    #[error("Accumulator error: {0}")]
    Accumulator(#[from] AccumulatorError),
    #[error("StarknetHandler error: {0}")]
    StarknetHandler(#[from] starknet_handler::StarknetHandlerError),
    #[error("MMRUtils error: {0}")]
    MMRUtils(#[from] MMRUtilsError),
    #[error("Headers Validator error: {0}")]
    Validator(#[from] ValidatorError),
}

#[derive(Error, Debug)]
pub enum AccumulatorError {
    #[error("Invalid state transition: elements count decreased")]
    InvalidStateTransition,
    #[error("Failed to verify stored peaks after update")]
    PeaksVerificationError,
    #[error("MMR root is not a valid Starknet field element: {0}")]
    InvalidU256Hex(String),
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Utils error: {0}")]
    Utils(#[from] UtilsError),
    #[error("MMR error: {0}")]
    MMRError(#[from] MMRError),
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    #[error("ProofGenerator error: {0}")]
    ProofGenerator(#[from] ProofGeneratorError),
    #[error("MMRUtils error: {0}")]
    MMRUtils(#[from] MMRUtilsError),
    #[error("InStoreTable error: {0}")]
    InStoreTable(#[from] InStoreTableError),
    #[error("StarknetHandler error: {0}")]
    StarknetHandler(#[from] starknet_handler::StarknetHandlerError),
    #[error("No headers found for block range {start_block} to {end_block}")]
    EmptyHeaders { start_block: u64, end_block: u64 },
}

#[derive(thiserror::Error, Debug)]
pub enum ValidatorError {
    #[error("Utils error: {0}")]
    Utils(#[from] common::UtilsError),
    #[error("MMR error: {0}")]
    MMRUtils(#[from] mmr_utils::MMRUtilsError),
    #[error("Store error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Store error: {0}")]
    Store(#[from] store::StoreError),
    #[error("MMR error: {0}")]
    MMRError(#[from] MMRError),
    #[error("ProofGenerator error: {0}")]
    ProofGenerator(#[from] ProofGeneratorError),
    #[error("Invalid proofs count {expected} != {actual}")]
    InvalidProofsCount { expected: usize, actual: usize },
}
