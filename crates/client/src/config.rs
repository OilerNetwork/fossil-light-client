//! Configuration management for the Fossil Light Client.
//!
//! This module handles loading, validating, and managing configuration
//! parameters from environment variables and command-line arguments.

use common::get_env_var;
use secrecy::{ExposeSecret, Secret};

use crate::{
    error::{ClientError, Result},
    types::{Address, BatchSize, ChainId, PollingInterval},
};

/// Configuration parameters for the light client.
///
/// This struct holds all the configuration needed to initialize and run
/// the light client, with validation to ensure all parameters are valid.
#[derive(Debug, Clone)]
pub struct LightClientConfig {
    pub polling_interval: PollingInterval,
    pub batch_size: BatchSize,
    pub start_block: u64,    // Keep as u64 for now since it can be 0
    pub blocks_per_run: u64, // Keep as u64 for now since it can be 0 (unlimited)
    pub starknet_rpc_url: String,
    pub l2_store_addr: Address,
    pub verifier_addr: Address,
    pub starknet_private_key: Secret<String>,
    pub starknet_account_address: Address,
    pub chain_id: ChainId,
}

impl LightClientConfig {
    /// Creates a new configuration by loading and validating environment variables.
    ///
    /// # Arguments
    ///
    /// * `polling_interval` - How often to poll for new events (in seconds, must be > 0)
    /// * `batch_size` - Number of blocks to process in each batch (must be > 0)
    /// * `start_block` - The block number to start processing from
    /// * `blocks_per_run` - Maximum number of blocks to process per run (0 for unlimited)
    ///
    /// # Returns
    ///
    /// Returns a validated `LightClientConfig` instance.
    ///
    /// # Errors
    ///
    /// * `ClientError::InvalidPollingInterval` - If polling_interval is 0
    /// * `ClientError::InvalidBatchSize` - If batch_size is 0
    /// * `ClientError::MissingEnvironmentVariable` - If required env vars are missing
    /// * `ClientError::InvalidAddress` - If addresses are malformed
    /// * `ClientError::InvalidUrl` - If the RPC URL is malformed
    /// * `ClientError::EnvironmentVariableParseError` - If CHAIN_ID parsing fails
    pub async fn from_env(
        polling_interval_secs: u64,
        batch_size_value: u64,
        start_block: u64,
        blocks_per_run: u64,
    ) -> Result<Self> {
        // Create and validate typed parameters
        let polling_interval = PollingInterval::new(polling_interval_secs)
            .ok_or(ClientError::InvalidPollingInterval(polling_interval_secs))?;

        let batch_size = BatchSize::new(batch_size_value)
            .ok_or(ClientError::InvalidBatchSize(batch_size_value))?;

        // Load and validate environment variables
        let starknet_rpc_url = get_env_var("STARKNET_RPC_URL")
            .map_err(|_| ClientError::missing_env_var("STARKNET_RPC_URL"))?;
        Self::validate_url(&starknet_rpc_url)?;

        let l2_store_addr_str = get_env_var("FOSSIL_STORE")
            .map_err(|_| ClientError::missing_env_var("FOSSIL_STORE"))?;
        let l2_store_addr = Address::new(l2_store_addr_str.clone())
            .ok_or_else(|| ClientError::invalid_address(l2_store_addr_str))?;

        let verifier_addr_str = get_env_var("FOSSIL_VERIFIER")
            .map_err(|_| ClientError::missing_env_var("FOSSIL_VERIFIER"))?;
        let verifier_addr = Address::new(verifier_addr_str.clone())
            .ok_or_else(|| ClientError::invalid_address(verifier_addr_str))?;

        let starknet_private_key = get_env_var("STARKNET_PRIVATE_KEY")
            .map_err(|_| ClientError::missing_env_var("STARKNET_PRIVATE_KEY"))?;
        let starknet_private_key = Secret::new(starknet_private_key);

        let starknet_account_address_str = get_env_var("STARKNET_ACCOUNT_ADDRESS")
            .map_err(|_| ClientError::missing_env_var("STARKNET_ACCOUNT_ADDRESS"))?;
        let starknet_account_address = Address::new(starknet_account_address_str.clone())
            .ok_or_else(|| ClientError::invalid_address(starknet_account_address_str))?;

        let chain_id_str =
            get_env_var("CHAIN_ID").map_err(|_| ClientError::missing_env_var("CHAIN_ID"))?;
        let chain_id_value = chain_id_str
            .parse::<u64>()
            .map_err(|e| ClientError::env_parse_error("CHAIN_ID", e))?;
        let chain_id = ChainId::new(chain_id_value);

        Ok(Self {
            polling_interval,
            batch_size,
            start_block,
            blocks_per_run,
            starknet_rpc_url,
            l2_store_addr,
            verifier_addr,
            starknet_private_key,
            starknet_account_address,
            chain_id,
        })
    }

    /// Validates a URL format.
    fn validate_url(url: &str) -> Result<()> {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ClientError::invalid_url(url));
        }
        Ok(())
    }

    /// Exposes the private key securely for use in operations.
    ///
    /// # Returns
    ///
    /// Returns the private key string for use in cryptographic operations.
    /// This should only be called when the key is actually needed.
    pub fn private_key(&self) -> &str {
        self.starknet_private_key.expose_secret()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_type_validation() {
        // Test invalid polling interval
        assert!(PollingInterval::new(0).is_none());

        // Test valid polling interval
        assert!(PollingInterval::new(5).is_some());

        // Test invalid batch size
        assert!(BatchSize::new(0).is_none());

        // Test valid batch size
        assert!(BatchSize::new(100).is_some());
    }

    #[test]
    fn test_address_validation() {
        // Test valid address
        assert!(Address::new("0x1234567890abcdef").is_some());

        // Test invalid addresses
        assert!(Address::new("1234567890abcdef").is_none());
        assert!(Address::new("0x").is_none());
        assert!(Address::new("").is_none());
    }

    #[test]
    fn test_url_validation() {
        // Test valid URLs
        assert!(LightClientConfig::validate_url("http://localhost:5050").is_ok());
        assert!(LightClientConfig::validate_url("https://api.starknet.io").is_ok());

        // Test invalid URLs
        assert!(LightClientConfig::validate_url("ftp://example.com").is_err());
        assert!(LightClientConfig::validate_url("localhost:5050").is_err());
        assert!(LightClientConfig::validate_url("").is_err());
    }
}
