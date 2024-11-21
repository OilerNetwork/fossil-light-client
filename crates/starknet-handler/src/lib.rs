#![deny(unused_crate_dependencies)]

pub mod account;
pub mod provider;
use starknet::accounts::single_owner::SignError;
use starknet::accounts::AccountError;
use starknet::core::{
    codec::{Decode, Encode},
    types::Felt,
};
use starknet::signers::local_wallet::SignError as LocalWalletSignError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StarknetHandlerError {
    #[error("Failed to parse: {0}")]
    ParseError(String),
    #[error("Failed to create selector: {0}")]
    SelectorError(String),
    #[error("Failed to execute transaction: {0}")]
    TransactionError(String),
    #[error("Starknet error: {0}")]
    Starknet(#[from] SignError<LocalWalletSignError>),
    #[error("Account error: {0}")]
    Account(#[from] AccountError<SignError<LocalWalletSignError>>),
}

#[derive(Debug, Encode, Decode)]
pub struct MmrState {
    root_hash: Felt,
    elements_count: u64,
    leaves_count: u64,
    peaks: Vec<Felt>,
}

impl MmrState {
    pub fn new(root_hash: Felt, elements_count: u64, leaves_count: u64, peaks: Vec<Felt>) -> Self {
        Self {
            root_hash,
            elements_count,
            leaves_count,
            peaks,
        }
    }
}

pub fn string_array_to_felt_array(
    string_array: Vec<String>,
) -> Result<Vec<Felt>, StarknetHandlerError> {
    string_array
        .iter()
        .map(|s| felt(s).map_err(|_| StarknetHandlerError::ParseError(s.clone())))
        .collect()
}

pub fn felt(str: &str) -> Result<Felt, StarknetHandlerError> {
    Felt::from_hex(str).map_err(|_| StarknetHandlerError::ParseError(str.to_string()))
}
