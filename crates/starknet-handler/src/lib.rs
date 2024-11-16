pub mod account;
pub mod provider;
use starknet::core::types::Felt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StarknetHandlerError {
    #[error("Failed to parse: {0}")]
    ParseError(String),
    #[error("Failed to create selector: {0}")]
    SelectorError(String),
    #[error("Failed to execute transaction: {0}")]
    TransactionError(String),
}

pub fn get_selector(name: &str) -> Result<Felt, StarknetHandlerError> {
    starknet::core::utils::get_selector_from_name(name)
        .map_err(|_| StarknetHandlerError::SelectorError(name.to_string()))
}

pub fn get_felt_from_str(str: &str) -> Result<Felt, StarknetHandlerError> {
    Felt::from_hex(str).map_err(|_| StarknetHandlerError::ParseError(str.to_string()))
}
