//! # Configuration Management
//!
//! This module provides comprehensive configuration management for the Publisher crate,
//! including validation, builder patterns, and error handling.
//!
//! ## Features
//!
//! - **Type-safe configuration**: All configuration parameters are strongly typed
//! - **Builder pattern**: Fluent API for easy configuration construction
//! - **Validation**: Built-in validation for all configuration parameters
//! - **Error handling**: Detailed error messages for configuration issues
//!
//! ## Examples
//!
//! ### Basic Configuration
//!
//! ```rust,no_run
//! use publisher::config::{PublisherConfig, AccountConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = PublisherConfig {
//!     rpc_url: "http://localhost:8545".to_string(),
//!     chain_id: 1,
//!     verifier_address: "0x1234567890123456789012345678901234567890".to_string(),
//!     store_address: "0x0987654321098765432109876543210987654321".to_string(),
//!     batch_size: 100,
//! };
//!
//! // Validate configuration
//! config.validate()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Using Builder Pattern
//!
//! ```rust,no_run
//! use publisher::config::PublisherConfig;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = PublisherConfig::builder()
//!     .rpc_url("http://localhost:8545")
//!     .chain_id(1)
//!     .verifier_address("0x1234567890123456789012345678901234567890")
//!     .store_address("0x0987654321098765432109876543210987654321")
//!     .batch_size(100)
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Account Configuration
//!
//! ```rust,no_run
//! use publisher::config::AccountConfig;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let account = AccountConfig::new(
//!     "0xprivate_key".to_string(),
//!     "0xaddress".to_string(),
//! );
//!
//! account.validate()?;
//! # Ok(())
//! # }
//! ```

use std::fmt;

/// Configuration for the Publisher service operations
///
/// This structure contains all the necessary configuration parameters for
/// Publisher operations, including network settings, addresses, and operational parameters.
///
/// # Examples
///
/// ```rust
/// use publisher::config::PublisherConfig;
///
/// let config = PublisherConfig {
///     rpc_url: "http://localhost:8545".to_string(),
///     chain_id: 1,
///     verifier_address: "0x1234567890123456789012345678901234567890".to_string(),
///     store_address: "0x0987654321098765432109876543210987654321".to_string(),
///     batch_size: 100,
/// };
///
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct PublisherConfig {
    /// RPC URL for connecting to the blockchain network
    pub rpc_url: String,
    /// Chain ID of the target blockchain network
    pub chain_id: u64,
    /// Address of the verifier contract on Starknet
    pub verifier_address: String,
    /// Address of the store contract on Starknet
    pub store_address: String,
    /// Number of blocks to process in each batch
    pub batch_size: u64,
}

impl PublisherConfig {
    /// Create a new builder for `PublisherConfig`
    pub fn builder() -> PublisherConfigBuilder {
        PublisherConfigBuilder::default()
    }

    /// Validate the configuration
    pub const fn validate(&self) -> Result<(), ConfigError> {
        if self.rpc_url.is_empty() {
            return Err(ConfigError::MissingField("rpc_url"));
        }
        if self.verifier_address.is_empty() {
            return Err(ConfigError::MissingField("verifier_address"));
        }
        if self.store_address.is_empty() {
            return Err(ConfigError::MissingField("store_address"));
        }
        if self.batch_size == 0 {
            return Err(ConfigError::InvalidValue(
                "batch_size must be greater than 0",
            ));
        }
        Ok(())
    }
}

/// Builder for `PublisherConfig` using the builder pattern
///
/// Provides a fluent interface for constructing [`PublisherConfig`] instances
/// with validation and default values.
///
/// # Examples
///
/// ```rust,no_run
/// use publisher::config::PublisherConfig;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = PublisherConfig::builder()
///     .rpc_url("http://localhost:8545")
///     .chain_id(1)
///     .verifier_address("0x1234567890123456789012345678901234567890")
///     .store_address("0x0987654321098765432109876543210987654321")
///     .batch_size(100)
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Default, Clone)]
pub struct PublisherConfigBuilder {
    rpc_url: Option<String>,
    chain_id: Option<u64>,
    verifier_address: Option<String>,
    store_address: Option<String>,
    batch_size: Option<u64>,
}

impl PublisherConfigBuilder {
    /// Set the RPC URL for blockchain connection
    pub fn rpc_url<T: Into<String>>(mut self, rpc_url: T) -> Self {
        self.rpc_url = Some(rpc_url.into());
        self
    }

    /// Set the chain ID for blockchain operations
    pub const fn chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Set the verifier contract address
    pub fn verifier_address<T: Into<String>>(mut self, verifier_address: T) -> Self {
        self.verifier_address = Some(verifier_address.into());
        self
    }

    /// Set the store contract address
    pub fn store_address<T: Into<String>>(mut self, store_address: T) -> Self {
        self.store_address = Some(store_address.into());
        self
    }

    /// Set the batch size for processing
    pub const fn batch_size(mut self, batch_size: u64) -> Self {
        self.batch_size = Some(batch_size);
        self
    }

    /// Build the final `PublisherConfig`, validating all required fields
    pub fn build(self) -> Result<PublisherConfig, ConfigError> {
        let config = PublisherConfig {
            rpc_url: self.rpc_url.ok_or(ConfigError::MissingField("rpc_url"))?,
            chain_id: self.chain_id.unwrap_or(0),
            verifier_address: self
                .verifier_address
                .ok_or(ConfigError::MissingField("verifier_address"))?,
            store_address: self
                .store_address
                .ok_or(ConfigError::MissingField("store_address"))?,
            batch_size: self.batch_size.unwrap_or(100),
        };

        config.validate()?;
        Ok(config)
    }
}

/// Account configuration for operations that require account access
///
/// Contains the private key and address information needed for
/// Starknet account operations.
///
/// # Security Note
///
/// The private key is stored as a string. In production environments,
/// consider using more secure storage mechanisms.
///
/// # Examples
///
/// ```rust
/// use publisher::config::AccountConfig;
///
/// let account = AccountConfig::new(
///     "0xprivate_key".to_string(),
///     "0xaddress".to_string(),
/// );
///
/// assert!(account.validate().is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct AccountConfig {
    /// Private key for the Starknet account (hex-encoded)
    pub private_key: String,
    /// Address of the Starknet account (hex-encoded)
    pub address: String,
}

impl AccountConfig {
    /// Create a new account configuration with private key and address
    pub const fn new(private_key: String, address: String) -> Self {
        Self {
            private_key,
            address,
        }
    }

    /// Validate that the account configuration has all required fields
    pub const fn validate(&self) -> Result<(), ConfigError> {
        if self.private_key.is_empty() {
            return Err(ConfigError::MissingField("private_key"));
        }
        if self.address.is_empty() {
            return Err(ConfigError::MissingField("address"));
        }
        Ok(())
    }
}

/// Configuration errors
///
/// Represents various validation errors that can occur when
/// creating or validating configuration objects.
#[derive(Debug)]
pub enum ConfigError {
    /// A required field is missing from the configuration
    MissingField(&'static str),
    /// A field contains an invalid value
    InvalidValue(&'static str),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "Missing required field: {field}"),
            Self::InvalidValue(msg) => write!(f, "Invalid configuration value: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publisher_config_builder() {
        let config = PublisherConfig::builder()
            .rpc_url("http://localhost:8545")
            .chain_id(1)
            .verifier_address("0x1234567890123456789012345678901234567890")
            .store_address("0x0987654321098765432109876543210987654321")
            .batch_size(100)
            .build()
            .unwrap();

        assert_eq!(config.rpc_url, "http://localhost:8545");
        assert_eq!(config.chain_id, 1);
        assert_eq!(config.batch_size, 100);
    }

    #[test]
    fn test_builder_missing_required_field() {
        let result = PublisherConfig::builder().chain_id(1).build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::MissingField("rpc_url")
        ));
    }

    #[test]
    fn test_config_validation() {
        let config = PublisherConfig {
            rpc_url: "".to_string(),
            chain_id: 1,
            verifier_address: "0x123".to_string(),
            store_address: "0x456".to_string(),
            batch_size: 100,
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::MissingField("rpc_url")
        ));
    }

    #[test]
    fn test_account_config_validation() {
        let config = AccountConfig::new("".to_string(), "0x123".to_string());
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::MissingField("private_key")
        ));
    }
}
