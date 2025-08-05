//! Error types for the Fossil Light Client.
//!
//! This module defines all error types that can occur during light client
//! operations, using `thiserror` for ergonomic error handling.

use thiserror::Error;

/// Comprehensive error type for all light client operations.
///
/// This enum covers all possible error conditions that can occur during
/// light client initialization, configuration, and runtime operations.
/// Each variant provides specific context about what went wrong.
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Invalid polling interval: {0}. Must be greater than zero")]
    InvalidPollingInterval(u64),

    #[error("Invalid batch size: {0}. Must be greater than zero")]
    InvalidBatchSize(u64),

    #[error(
        "Invalid block range: from_block ({from_block}) is greater than to_block ({to_block})"
    )]
    InvalidBlockRange { from_block: u64, to_block: u64 },

    #[error("Invalid address format: {address}")]
    InvalidAddress { address: String },

    #[error("Invalid URL format: {url}")]
    InvalidUrl { url: String },

    #[error("Invalid chain ID: {chain_id}")]
    InvalidChainId { chain_id: String },

    #[error("Environment variable not found: {var_name}")]
    MissingEnvironmentVariable { var_name: String },

    #[error("Failed to parse environment variable {var_name}: {source}")]
    EnvironmentVariableParseError {
        var_name: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Starknet provider error: {0}")]
    StarknetProvider(#[from] eyre::Report),

    #[error("Publisher error: {0}")]
    Publisher(String),

    #[error("Starknet RPC error: {0}")]
    StarknetRpc(#[from] starknet::providers::ProviderError),

    #[error("Database file does not exist at path: {path}")]
    DatabaseFileNotFound { path: String },

    #[error("Failed to get block number after {attempts} attempts")]
    BlockNumberRetriesFailed { attempts: u32 },

    #[error("No new blocks to process")]
    NoNewBlocks,

    #[error("Block {block_number} already processed")]
    BlockAlreadyProcessed { block_number: u64 },

    #[error("Operation timed out after {duration_ms}ms: {operation}")]
    OperationTimeout { operation: String, duration_ms: u64 },

    #[error("Async operation failed with context: {context}")]
    AsyncOperationFailed {
        context: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl ClientError {
    /// Creates a new `MissingEnvironmentVariable` error.
    ///
    /// # Arguments
    ///
    /// * `var_name` - The name of the missing environment variable
    pub fn missing_env_var(var_name: impl Into<String>) -> Self {
        Self::MissingEnvironmentVariable {
            var_name: var_name.into(),
        }
    }

    /// Creates a new `EnvironmentVariableParseError` with context about the parsing failure.
    ///
    /// # Arguments
    ///
    /// * `var_name` - The name of the environment variable that failed to parse
    /// * `source` - The underlying parsing error
    pub fn env_parse_error(
        var_name: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::EnvironmentVariableParseError {
            var_name: var_name.into(),
            source: Box::new(source),
        }
    }

    /// Creates a new `InvalidAddress` error.
    ///
    /// # Arguments
    ///
    /// * `address` - The invalid address string
    pub fn invalid_address(address: impl Into<String>) -> Self {
        Self::InvalidAddress {
            address: address.into(),
        }
    }

    /// Creates a new `InvalidUrl` error.
    ///
    /// # Arguments
    ///
    /// * `url` - The invalid URL string
    pub fn invalid_url(url: impl Into<String>) -> Self {
        Self::InvalidUrl { url: url.into() }
    }

    /// Creates a new `Publisher` error.
    ///
    /// # Arguments
    ///
    /// * `msg` - The error message describing the publisher failure
    pub fn publisher_error(msg: impl Into<String>) -> Self {
        Self::Publisher(msg.into())
    }

    /// Creates a new `OperationTimeout` error.
    ///
    /// # Arguments
    ///
    /// * `operation` - Description of the operation that timed out
    /// * `duration_ms` - The timeout duration in milliseconds
    pub fn operation_timeout(operation: impl Into<String>, duration_ms: u64) -> Self {
        Self::OperationTimeout {
            operation: operation.into(),
            duration_ms,
        }
    }

    /// Creates a new `AsyncOperationFailed` error with context.
    ///
    /// # Arguments
    ///
    /// * `context` - Description of what was being attempted
    /// * `source` - The underlying error that caused the failure
    pub fn async_operation_failed(
        context: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::AsyncOperationFailed {
            context: context.into(),
            source: Box::new(source),
        }
    }
}

/// A specialized `Result` type for light client operations.
///
/// This is a convenience type alias that uses `ClientError` as the error type,
/// making function signatures more concise throughout the codebase.
pub type Result<T> = std::result::Result<T, ClientError>;
