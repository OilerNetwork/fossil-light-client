pub mod account;
pub mod error;
pub mod provider;
use crate::error::StarknetHandlerError;
use starknet::core::types::Felt;

pub fn get_selector(name: &str) -> Result<Felt, StarknetHandlerError> {
    starknet::core::utils::get_selector_from_name(name)
        .map_err(|_| StarknetHandlerError::SelectorError(name.to_string()))
}
