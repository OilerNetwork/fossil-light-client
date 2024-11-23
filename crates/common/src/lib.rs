#![deny(unused_crate_dependencies)]

use eyre::Result;
use starknet_crypto::Felt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UtilsError {
    #[error("Environment variable {0} not set")]
    EnvVarNotSet(String),
    #[error("Unable to parse string: {0}")]
    ParseError(String),
    #[error("Logger initialization failed")]
    LoggerInitFailed,
    #[error("Alloy contract error: {0}")]
    AlloyContractError(#[from] alloy_contract::Error),
    #[error("Failed to convert Uint to u64")]
    UintError(#[from] ruint::FromUintError<u64>),
}

/// Retrieves an environment variable or returns an error if not set.
pub fn get_env_var(key: &str) -> Result<String, UtilsError> {
    dotenv::var(key).map_err(|_| UtilsError::EnvVarNotSet(key.to_string()))
}

/// Parses an environment variable into the desired type or returns an error.
pub fn get_var<T: FromStr>(name: &str) -> Result<T, UtilsError>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let var_value = get_env_var(name)?;
    var_value
        .parse::<T>()
        .map_err(|e| UtilsError::ParseError(format!("{}: {}", name, e)))
}

/// Function to initialize logging and environment variables
pub fn initialize_logger_and_env() -> Result<(), UtilsError> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt().init();
    Ok(())
}

pub fn string_array_to_felt_array(string_array: Vec<String>) -> Result<Vec<Felt>, UtilsError> {
    string_array
        .iter()
        .map(|s| felt(s).map_err(|_| UtilsError::ParseError(s.clone())))
        .collect()
}

pub fn felt(str: &str) -> Result<Felt, UtilsError> {
    Felt::from_hex(str).map_err(|_| UtilsError::ParseError(str.to_string()))
}
