use common::UtilsError;
use mmr::{InStoreTableError, MMRError, StoreError};
use mmr_utils::MMRUtilsError;
use starknet_types_core::felt::FromStrError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PublisherError {
    #[error("Verification failed: no verification result was produced")]
    VerificationError,
    #[error("Accumulator operation failed: {0}")]
    Accumulator(#[from] AccumulatorError),
    #[error("Starknet interaction failed: {0}")]
    StarknetHandler(#[from] starknet_handler::StarknetHandlerError),
    #[error("MMR utilities operation failed: {0}")]
    MMRUtils(#[from] MMRUtilsError),
    #[error("Header validation failed: {0}")]
    Validator(#[from] ValidatorError),
    #[error("Invalid Stark proof receipt: receipt format or signature verification failed")]
    ReceiptError,
}

#[derive(Error, Debug)]
pub enum AccumulatorError {
    #[error(
        "Invalid state transition detected: total elements count decreased from previous state"
    )]
    InvalidStateTransition,
    #[error(
        "Peak verification failed: stored peaks hash doesn't match computed peaks after update"
    )]
    PeaksVerificationError,
    #[error("Invalid MMR root format: value '{0}' cannot be converted to a valid Starknet field element")]
    InvalidU256Hex(String),
    #[error("Database operation failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Utility operation failed: {0}")]
    Utils(#[from] UtilsError),
    #[error("MMR operation failed: {0}")]
    MMRError(#[from] MMRError),
    #[error("Storage operation failed: {0}")]
    Store(#[from] StoreError),
    #[error("Proof generation failed: {0}")]
    ProofGenerator(#[from] ProofGeneratorError),
    #[error("MMR utilities operation failed: {0}")]
    MMRUtils(#[from] MMRUtilsError),
    #[error("In-store table operation failed: {0}")]
    InStoreTable(#[from] InStoreTableError),
    #[error("Starknet interaction failed: {0}")]
    StarknetHandler(#[from] starknet_handler::StarknetHandlerError),
    #[error("No headers available for block range {start_block} to {end_block}. The range might be invalid or the data might not be synced")]
    EmptyHeaders { start_block: u64, end_block: u64 },
    #[error("Invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("Blockchain operation failed: {0}")]
    BlockchainError(String),
    #[error("Invalid block range: start block {start_block} is greater than end block {end_block}")]
    InvalidBlockRange { start_block: u64, end_block: u64 },
}

#[derive(thiserror::Error, Debug)]
pub enum ValidatorError {
    #[error("Utility operation failed: {0}")]
    Utils(#[from] common::UtilsError),
    #[error("MMR utilities operation failed: {0}")]
    MMRUtils(#[from] mmr_utils::MMRUtilsError),
    #[error("Database operation failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Storage operation failed: {0}")]
    Store(#[from] store::StoreError),
    #[error("MMR operation failed: {0}")]
    MMRError(#[from] MMRError),
    #[error("Proof generation failed: {0}")]
    ProofGenerator(#[from] ProofGeneratorError),
    #[error("Proof count mismatch: expected {expected} proofs but found {actual}. This might indicate data corruption or synchronization issues")]
    InvalidProofsCount { expected: usize, actual: usize },
    #[error("Invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("Starknet provider error: {0}")]
    StarknetProvider(#[from] starknet_handler::StarknetHandlerError),
    #[error("Invalid MMR root: expected {expected} but found {actual}")]
    InvalidMmrRoot { expected: String, actual: String },
    #[error("Failed to parse Felt value: {0}")]
    FeltParsing(#[from] FromStrError),
}

#[derive(Error, Debug)]
pub enum ProofGeneratorError {
    #[error("Invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("Failed to write input to executor env: {0}")]
    ExecutorEnvError(String),
    #[error("Failed to generate receipt: {0}")]
    ReceiptError(String),
    #[error("Failed to compute image id: {0}")]
    ImageIdError(String),
    #[error("Failed to encode seal: {0}")]
    SealError(String),
    #[error("Failed to generate StarkNet calldata: {0}")]
    CalldataError(String),
    #[error("Failed to spawn blocking task: {0}")]
    SpawnBlocking(String),
    #[error("Tokio task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("Risc0 serde error: {0}")]
    Risc0Serde(#[from] risc0_zkvm::serde::Error),
}
