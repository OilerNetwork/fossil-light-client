use std::fmt;

/// Domain-specific error types for the publisher crate
#[derive(Debug, thiserror::Error)]
pub enum PublisherError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IPFS error: {0}")]
    Ipfs(String),

    #[error("Proof generation failed: {0}")]
    ProofGeneration(String),

    #[error("MMR operation failed: {0}")]
    MmrOperation(String),

    #[error("Starknet provider error: {0}")]
    StarknetProvider(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Network error: {0}")]
    Network(String),
}

impl PublisherError {
    pub fn ipfs<T: fmt::Display>(msg: T) -> Self {
        Self::Ipfs(msg.to_string())
    }

    pub fn proof_generation<T: fmt::Display>(msg: T) -> Self {
        Self::ProofGeneration(msg.to_string())
    }

    pub fn mmr_operation<T: fmt::Display>(msg: T) -> Self {
        Self::MmrOperation(msg.to_string())
    }

    pub fn starknet_provider<T: fmt::Display>(msg: T) -> Self {
        Self::StarknetProvider(msg.to_string())
    }

    pub fn configuration<T: fmt::Display>(msg: T) -> Self {
        Self::Configuration(msg.to_string())
    }

    pub fn validation<T: fmt::Display>(msg: T) -> Self {
        Self::Validation(msg.to_string())
    }

    pub fn serialization<T: fmt::Display>(msg: T) -> Self {
        Self::Serialization(msg.to_string())
    }

    pub fn network<T: fmt::Display>(msg: T) -> Self {
        Self::Network(msg.to_string())
    }
}

// Keep existing Result<T> exports unchanged for backward compatibility
pub type Result<T> = std::result::Result<T, eyre::Error>;

// New Result type using PublisherError for internal use
pub type PublisherResult<T> = std::result::Result<T, PublisherError>;

// Helper function for converting PublisherError to eyre::Error
impl PublisherError {
    pub fn into_eyre(self) -> eyre::Error {
        eyre::Error::new(self)
    }
}
