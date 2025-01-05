#![deny(unused_crate_dependencies)]

use starknet_crypto::Felt;
use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
    str::FromStr,
};

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
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Retry exhausted after {0} attempts: {1}")]
    RetryExhausted(u32, String),
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
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Define default filter directives - adjust these based on your needs
        let directives = [
            "sqlx=off",
            "info",
            "handle_events=warn", // Reduce verbosity of handle_events
            "publisher=info",     // Keep publisher at info level
        ];

        let mut filter = tracing_subscriber::EnvFilter::new("");
        for directive in directives {
            if let Ok(d) = directive.parse() {
                filter = filter.add_directive(d);
            }
        }
        filter
    });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false) // Removes module path from output
        .with_thread_ids(false) // Removes thread IDs
        .with_thread_names(false) // Removes thread names
        .with_file(true)
        .with_line_number(true)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE) // Reduces span noise
        .compact() // Uses more compact format
        .init();
    Ok(())
}

pub fn string_array_to_felt_array(string_array: Vec<String>) -> Result<Vec<Felt>, UtilsError> {
    string_array.iter().map(|s| felt(s)).collect()
}

pub fn felt(str: &str) -> Result<Felt, UtilsError> {
    Felt::from_hex(str).map_err(|_| UtilsError::FeltError(format!("Invalid hex string: {}", str)))
}

pub fn get_or_create_db_path(db_name: &str) -> Result<String, UtilsError> {
    // Get path to the db-instances directory relative to the test file
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| UtilsError::ParseError("Missing parent directory".to_string()))?
        .parent()
        .ok_or_else(|| UtilsError::ParseError("Missing root directory".to_string()))?
        .join("db-instances");

    // Ensure the directory exists
    if !test_dir.exists() {
        fs::create_dir_all(&test_dir)?;
    }

    // Construct the full path to the database file
    let db_file_path = test_dir.join(db_name);

    // Ensure the file exists
    if !db_file_path.exists() {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&db_file_path)?;
    }

    // Convert to string
    let db_path_str = db_file_path
        .to_str()
        .ok_or_else(|| UtilsError::ParseError("Invalid path".to_string()))?;

    Ok(db_path_str.to_string())
}
