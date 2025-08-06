use std::time::Duration;

use thiserror::Error;

/// Main error type for the relayer application
#[derive(Debug, Error)]
pub enum RelayerError {
    #[error("Configuration error: {0}")]
    /// Configuration error
    Config(#[from] ConfigError),

    #[error("Network error: {0}")]
    /// Network error
    Network(#[from] NetworkError),

    #[error("Contract interaction failed: {0}")]
    /// Contract interaction error
    Contract(String),

    #[error("Environment error: {0}")]
    /// Environment variable error
    Environment(#[from] std::env::VarError),

    #[error("IO error: {0}")]
    /// IO error
    Io(#[from] std::io::Error),
}

/// Configuration-related errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid private key format: {0}")]
    /// Invalid private key format
    InvalidPrivateKey(String),

    #[error("Invalid L2 address format: expected 0x + 64 hex chars, got {0}")]
    /// Invalid L2 address format
    InvalidL2Address(String),

    #[error("Missing environment variable: {0}")]
    /// Missing environment variable
    MissingEnvVar(String),

    #[error("Invalid address format: {0}")]
    /// Invalid address format
    InvalidAddress(String),

    #[error("Invalid URL format: {0}")]
    /// Invalid URL format
    InvalidUrl(String),

    #[error("Invalid duration: {0}")]
    /// Invalid duration
    InvalidDuration(String),
}

/// Network-related errors
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    /// Connection failed
    ConnectionFailed(String),

    #[error("Transaction failed: {0}")]
    /// Transaction failed
    TransactionFailed(String),

    #[error("Provider error: {0}")]
    /// Provider error
    ProviderError(String),

    #[error("Timeout after {timeout:?}: {operation}")]
    /// Operation timeout
    Timeout {
        /// The timeout duration
        timeout: Duration,
        /// The operation that timed out
        operation: String,
    },

    #[error("Invalid response: {0}")]
    /// Invalid response received
    InvalidResponse(String),
}

/// Result type alias for the relayer
pub type Result<T> = std::result::Result<T, RelayerError>;
