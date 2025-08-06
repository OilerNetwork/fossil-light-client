use std::fmt;

/// Domain-specific error types for the publisher crate
#[derive(Debug, Clone, thiserror::Error)]
pub enum PublisherError {
    #[error("Database error: {0}")]
    /// Database operation error
    Database(String),

    #[error("IPFS error: {0}")]
    /// IPFS operation error
    Ipfs(String),

    #[error("Proof generation failed: {0}")]
    /// Proof generation error
    ProofGeneration(String),

    #[error("MMR operation failed: {0}")]
    /// MMR operation error
    MmrOperation(String),

    #[error("Starknet provider error: {0}")]
    /// Starknet provider error
    StarknetProvider(String),

    #[error("Configuration error: {0}")]
    /// Configuration validation error
    Configuration(String),

    #[error("Validation error: {0}")]
    /// General validation error
    Validation(String),

    #[error("IO error: {0}")]
    /// File I/O operation error
    Io(String),

    #[error("Serialization error: {0}")]
    /// Data serialization error
    Serialization(String),

    #[error("Network error: {0}")]
    /// Network communication error
    Network(String),
}

impl PublisherError {
    /// Create a database error
    pub fn database<T: fmt::Display>(msg: T) -> Self {
        Self::Database(msg.to_string())
    }

    /// Create an IPFS error
    pub fn ipfs<T: fmt::Display>(msg: T) -> Self {
        Self::Ipfs(msg.to_string())
    }

    /// Create a proof generation error
    pub fn proof_generation<T: fmt::Display>(msg: T) -> Self {
        Self::ProofGeneration(msg.to_string())
    }

    /// Create an MMR operation error
    pub fn mmr_operation<T: fmt::Display>(msg: T) -> Self {
        Self::MmrOperation(msg.to_string())
    }

    /// Create a Starknet provider error
    pub fn starknet_provider<T: fmt::Display>(msg: T) -> Self {
        Self::StarknetProvider(msg.to_string())
    }

    /// Create a configuration error
    pub fn configuration<T: fmt::Display>(msg: T) -> Self {
        Self::Configuration(msg.to_string())
    }

    /// Create a validation error
    pub fn validation<T: fmt::Display>(msg: T) -> Self {
        Self::Validation(msg.to_string())
    }

    /// Create an I/O error
    pub fn io<T: fmt::Display>(msg: T) -> Self {
        Self::Io(msg.to_string())
    }

    /// Create a serialization error
    pub fn serialization<T: fmt::Display>(msg: T) -> Self {
        Self::Serialization(msg.to_string())
    }

    /// Create a network error
    pub fn network<T: fmt::Display>(msg: T) -> Self {
        Self::Network(msg.to_string())
    }
}

/// Validator-specific error types for backwards compatibility with validator module
#[derive(Debug, thiserror::Error)]
pub enum ValidatorError {
    #[error("Invalid input: {0}")]
    /// Invalid input parameter
    InvalidInput(&'static str),

    #[error("Invalid MMR root - expected: {expected}, got: {actual}")]
    /// MMR root validation failed
    InvalidMmrRoot {
        /// Expected root hash
        expected: String,
        /// Actual root hash
        actual: String,
    },

    #[error("Invalid proofs count - expected: {expected}, got: {actual}")]
    /// Invalid number of proofs provided
    InvalidProofsCount {
        /// Expected number of proofs
        expected: usize,
        /// Actual number of proofs
        actual: usize,
    },

    #[error("Store error: {0}")]
    /// Store operation error
    Store(#[from] store::StoreError),

    #[error("Publisher error: {0}")]
    /// Publisher operation error
    Publisher(#[from] PublisherError),
}

// Keep existing Result<T> exports unchanged for backward compatibility
/// General result type using eyre for error handling
pub type Result<T> = std::result::Result<T, eyre::Error>;

// New Result type using PublisherError for internal use
/// Result type specific to publisher operations
pub type PublisherResult<T> = std::result::Result<T, PublisherError>;

// Helper function for converting PublisherError to eyre::Error
impl PublisherError {
    /// Convert PublisherError into an eyre::Error for easier error handling
    pub fn into_eyre(self) -> eyre::Error {
        eyre::Error::new(self)
    }
}

// From trait implementation for converting eyre::Error to PublisherError
impl From<eyre::Error> for PublisherError {
    fn from(err: eyre::Error) -> Self {
        // Try to downcast to see if it's already a PublisherError
        if let Some(publisher_err) = err.downcast_ref::<PublisherError>() {
            return publisher_err.clone();
        }

        // Otherwise, wrap it as a generic error
        PublisherError::Network(err.to_string())
    }
}

// From trait implementations for common error types
impl From<sqlx::Error> for PublisherError {
    fn from(err: sqlx::Error) -> Self {
        PublisherError::Database(err.to_string())
    }
}

impl From<std::io::Error> for PublisherError {
    fn from(err: std::io::Error) -> Self {
        PublisherError::Io(err.to_string())
    }
}

impl From<store::StoreError> for PublisherError {
    fn from(err: store::StoreError) -> Self {
        PublisherError::Database(err.to_string())
    }
}

impl From<mmr::MMRError> for PublisherError {
    fn from(err: mmr::MMRError) -> Self {
        PublisherError::MmrOperation(err.to_string())
    }
}

impl From<tokio::task::JoinError> for PublisherError {
    fn from(err: tokio::task::JoinError) -> Self {
        PublisherError::Network(format!("Task execution failed: {}", err))
    }
}

impl From<risc0_zkvm::serde::Error> for PublisherError {
    fn from(err: risc0_zkvm::serde::Error) -> Self {
        PublisherError::Serialization(err.to_string())
    }
}

impl From<crate::config::ConfigError> for PublisherError {
    fn from(err: crate::config::ConfigError) -> Self {
        PublisherError::Configuration(err.to_string())
    }
}
