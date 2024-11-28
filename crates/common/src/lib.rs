#![deny(unused_crate_dependencies)]

use starknet_crypto::Felt;
use std::str::FromStr;

#[derive(thiserror::Error, Debug)]
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
    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] dotenv::Error),
    #[error("Parse error: {0}")]
    ParseStringError(String),
    #[error("Felt conversion error: {0}")]
    FeltError(String),
}

/// Retrieves an environment variable or returns an error if not set.
pub fn get_env_var(key: &str) -> Result<String, UtilsError> {
    Ok(dotenv::var(key)?)
}

/// Parses an environment variable into the desired type or returns an error.
pub fn get_var<T: FromStr>(name: &str) -> Result<T, UtilsError>
where
    T::Err: std::fmt::Display,
{
    let var_value = get_env_var(name)?;
    var_value
        .parse()
        .map_err(|e| UtilsError::ParseError(format!("{}: {}", name, e)))
}

/// Function to initialize logging and environment variables
pub fn initialize_logger_and_env() -> Result<(), UtilsError> {
    dotenv::dotenv().ok();

    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let directive = match "sqlx=off".parse() {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to parse sqlx filter directive: {}", e);
                Default::default()
            }
        };
        tracing_subscriber::EnvFilter::new("info").add_directive(directive)
    });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_file(true)
        .init();
    Ok(())
}

pub fn string_array_to_felt_array(string_array: Vec<String>) -> Result<Vec<Felt>, UtilsError> {
    string_array.iter().map(|s| felt(s)).collect()
}

pub fn felt(str: &str) -> Result<Felt, UtilsError> {
    Felt::from_hex(str).map_err(|_| UtilsError::FeltError(format!("Invalid hex string: {}", str)))
}
