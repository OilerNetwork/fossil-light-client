//! Advanced configuration management for the Fossil Light Client.
//!
//! This module provides a centralized configuration system that can load
//! settings from multiple sources: files, environment variables, and defaults.

use std::path::Path;

use config::{builder::DefaultState, Config, ConfigBuilder, ConfigError, Environment, File};
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{
    async_utils::TimeoutConfig,
    error::{ClientError, Result},
    types::{Address, BatchSize, ChainId, PollingInterval},
};

/// Comprehensive configuration for the Fossil Light Client.
///
/// This struct can be loaded from configuration files, environment variables,
/// or constructed programmatically. It supports multiple configuration sources
/// with a clear precedence order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfiguration {
    /// Network and RPC settings
    pub network: NetworkConfig,

    /// Contract addresses and blockchain identifiers
    pub contracts: ContractConfig,

    /// Account and authentication settings
    pub auth: AuthConfig,

    /// Processing behavior settings
    pub processing: ProcessingConfig,

    /// Timeout and retry settings
    pub timeouts: TimeoutSettings,

    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Network and RPC configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Starknet RPC URL
    pub starknet_rpc_url: String,

    /// Chain ID for the target network
    pub chain_id: u64,

    /// Network name (for logging and identification)
    #[serde(default = "default_network_name")]
    pub network_name: String,
}

/// Contract addresses configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    /// L2 store contract address
    pub l2_store_address: String,

    /// Verifier contract address
    pub verifier_address: String,
}

/// Authentication and account configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Starknet private key (will be wrapped in Secret)
    pub starknet_private_key: String,

    /// Starknet account address
    pub starknet_account_address: String,
}

/// Processing behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Polling interval in seconds
    pub polling_interval_secs: u64,

    /// Batch size for processing
    pub batch_size: u64,

    /// Starting block number (0 for auto-detection)
    #[serde(default)]
    pub start_block: u64,

    /// Maximum blocks to process per run (0 for unlimited)
    #[serde(default)]
    pub blocks_per_run: u64,

    /// Whether to auto-detect the starting block
    #[serde(default)]
    pub auto_start_block: bool,
}

/// Timeout and retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutSettings {
    /// Network operation timeout in seconds
    #[serde(default = "default_network_timeout")]
    pub network_timeout_secs: u64,

    /// Database operation timeout in seconds
    #[serde(default = "default_database_timeout")]
    pub database_timeout_secs: u64,

    /// Cryptographic operation timeout in seconds
    #[serde(default = "default_crypto_timeout")]
    pub crypto_timeout_secs: u64,

    /// Maximum retry attempts for network operations
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Initial retry delay in milliseconds
    #[serde(default = "default_initial_retry_delay")]
    pub initial_retry_delay_ms: u64,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Whether to enable structured logging
    #[serde(default = "default_structured_logging")]
    pub structured: bool,

    /// Log format (json, pretty, compact)
    #[serde(default = "default_log_format")]
    pub format: String,
}

impl ClientConfiguration {
    /// Loads configuration from multiple sources with precedence order:
    /// 1. Configuration file (if provided)
    /// 2. Environment variables
    /// 3. Default values
    ///
    /// # Arguments
    ///
    /// * `config_file_path` - Optional path to configuration file
    ///
    /// # Returns
    ///
    /// Returns a fully configured `ClientConfiguration` instance.
    ///
    /// # Errors
    ///
    /// * `ClientError::AsyncOperationFailed` - If configuration loading fails
    pub fn load(config_file_path: Option<&Path>) -> Result<Self> {
        info!("Loading client configuration");

        let mut builder = Config::builder();
        builder = Self::set_default_values(builder)?;
        builder = Self::add_config_file_source(builder, config_file_path);
        builder = Self::add_environment_sources(builder);

        let config = Self::build_config(builder)?;
        let client_config = Self::parse_config(config)?;

        Self::log_loaded_config(&client_config);
        Ok(client_config)
    }

    fn set_default_values(
        mut builder: ConfigBuilder<DefaultState>,
    ) -> Result<ConfigBuilder<DefaultState>> {
        builder = builder.set_default("timeouts.network_timeout_secs", 30)?;
        builder = builder.set_default("timeouts.database_timeout_secs", 10)?;
        builder = builder.set_default("timeouts.crypto_timeout_secs", 60)?;
        builder = builder.set_default("timeouts.max_retries", 3)?;
        builder = builder.set_default("timeouts.initial_retry_delay_ms", 1000)?;
        builder = builder.set_default("logging.level", "info")?;
        builder = builder.set_default("logging.structured", true)?;
        builder = builder.set_default("logging.format", "pretty")?;
        builder = builder.set_default("network.network_name", "starknet")?;
        builder = builder.set_default("processing.start_block", 0)?;
        builder = builder.set_default("processing.blocks_per_run", 0)?;
        builder = builder.set_default("processing.auto_start_block", false)?;
        Ok(builder)
    }

    fn add_config_file_source(
        mut builder: ConfigBuilder<DefaultState>,
        config_file_path: Option<&Path>,
    ) -> ConfigBuilder<DefaultState> {
        if let Some(config_path) = config_file_path {
            if config_path.exists() {
                info!("Loading configuration from file: {}", config_path.display());
                builder = builder.add_source(File::from(config_path));
            } else {
                debug!("Configuration file not found: {}", config_path.display());
            }
        }
        builder
    }

    fn add_environment_sources(
        mut builder: ConfigBuilder<DefaultState>,
    ) -> ConfigBuilder<DefaultState> {
        // Add environment variables with FOSSIL_CLIENT prefix
        builder = builder.add_source(
            Environment::with_prefix("FOSSIL_CLIENT")
                .separator("_")
                .list_separator(","),
        );

        // Also support legacy environment variables without prefix
        builder =
            builder.add_source(Environment::default().source(Some(Self::create_legacy_env_map())));
        builder
    }

    fn create_legacy_env_map() -> std::collections::HashMap<String, String> {
        let mut env_map = std::collections::HashMap::new();

        // Map legacy env vars to new structure
        if let Ok(val) = std::env::var("STARKNET_RPC_URL") {
            env_map.insert("network.starknet_rpc_url".to_string(), val);
        }
        if let Ok(val) = std::env::var("CHAIN_ID") {
            env_map.insert("network.chain_id".to_string(), val);
        }
        if let Ok(val) = std::env::var("FOSSIL_STORE") {
            env_map.insert("contracts.l2_store_address".to_string(), val);
        }
        if let Ok(val) = std::env::var("FOSSIL_VERIFIER") {
            env_map.insert("contracts.verifier_address".to_string(), val);
        }
        if let Ok(val) = std::env::var("STARKNET_PRIVATE_KEY") {
            env_map.insert("auth.starknet_private_key".to_string(), val);
        }
        if let Ok(val) = std::env::var("STARKNET_ACCOUNT_ADDRESS") {
            env_map.insert("auth.starknet_account_address".to_string(), val);
        }

        env_map
    }

    fn build_config(builder: ConfigBuilder<DefaultState>) -> Result<Config> {
        builder
            .build()
            .map_err(|e| ClientError::async_operation_failed("configuration loading", e))
    }

    fn parse_config(config: Config) -> Result<Self> {
        config
            .try_deserialize()
            .map_err(|e| ClientError::async_operation_failed("configuration parsing", e))
    }

    fn log_loaded_config(client_config: &Self) {
        info!("Configuration loaded successfully");
        debug!(
            "Network: {} (Chain ID: {})",
            client_config.network.network_name, client_config.network.chain_id
        );
    }

    /// Converts this configuration to the legacy `LightClientConfig` format.
    ///
    /// This method provides backward compatibility with the existing codebase
    /// while allowing for a smooth transition to the new configuration system.
    pub fn to_legacy_config(&self) -> Result<crate::config::LightClientConfig> {
        let polling_interval = PollingInterval::new(self.processing.polling_interval_secs).ok_or(
            ClientError::InvalidPollingInterval(self.processing.polling_interval_secs),
        )?;

        let batch_size = BatchSize::new(self.processing.batch_size)
            .ok_or(ClientError::InvalidBatchSize(self.processing.batch_size))?;

        let l2_store_addr = Address::new(self.contracts.l2_store_address.clone())
            .ok_or_else(|| ClientError::invalid_address(&self.contracts.l2_store_address))?;

        let verifier_addr = Address::new(self.contracts.verifier_address.clone())
            .ok_or_else(|| ClientError::invalid_address(&self.contracts.verifier_address))?;

        let starknet_account_address = Address::new(self.auth.starknet_account_address.clone())
            .ok_or_else(|| ClientError::invalid_address(&self.auth.starknet_account_address))?;

        let chain_id = ChainId::new(self.network.chain_id);

        Ok(crate::config::LightClientConfig {
            polling_interval,
            batch_size,
            start_block: self.processing.start_block,
            blocks_per_run: self.processing.blocks_per_run,
            starknet_rpc_url: self.network.starknet_rpc_url.clone(),
            l2_store_addr,
            verifier_addr,
            starknet_private_key: Secret::new(self.auth.starknet_private_key.clone()),
            starknet_account_address,
            chain_id,
        })
    }

    /// Creates a `TimeoutConfig` from the timeout settings.
    pub const fn timeout_config(&self) -> TimeoutConfig {
        TimeoutConfig::new(
            self.timeouts.network_timeout_secs,
            self.timeouts.database_timeout_secs,
            self.timeouts.crypto_timeout_secs,
        )
    }

    /// Validates all configuration values.
    ///
    /// This method performs comprehensive validation of all configuration
    /// parameters to ensure they are valid and consistent.
    pub fn validate(&self) -> Result<()> {
        // Validate network configuration
        if self.network.starknet_rpc_url.is_empty() {
            return Err(ClientError::missing_env_var("starknet_rpc_url"));
        }

        if !self.network.starknet_rpc_url.starts_with("http://")
            && !self.network.starknet_rpc_url.starts_with("https://")
        {
            return Err(ClientError::invalid_url(&self.network.starknet_rpc_url));
        }

        // Validate contract addresses
        if Address::new(self.contracts.l2_store_address.clone()).is_none() {
            return Err(ClientError::invalid_address(
                &self.contracts.l2_store_address,
            ));
        }

        if Address::new(self.contracts.verifier_address.clone()).is_none() {
            return Err(ClientError::invalid_address(
                &self.contracts.verifier_address,
            ));
        }

        // Validate auth configuration
        if self.auth.starknet_private_key.is_empty() {
            return Err(ClientError::missing_env_var("starknet_private_key"));
        }

        if Address::new(self.auth.starknet_account_address.clone()).is_none() {
            return Err(ClientError::invalid_address(
                &self.auth.starknet_account_address,
            ));
        }

        // Validate processing configuration
        if self.processing.polling_interval_secs == 0 {
            return Err(ClientError::InvalidPollingInterval(
                self.processing.polling_interval_secs,
            ));
        }

        if self.processing.batch_size == 0 {
            return Err(ClientError::InvalidBatchSize(self.processing.batch_size));
        }

        // Validate timeout configuration
        if self.timeouts.network_timeout_secs == 0 {
            return Err(ClientError::async_operation_failed(
                "timeout validation",
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "network timeout cannot be zero",
                ),
            ));
        }

        info!("Configuration validation passed");
        Ok(())
    }
}

impl From<ConfigError> for ClientError {
    fn from(err: ConfigError) -> Self {
        Self::async_operation_failed("configuration error", err)
    }
}

// Default value functions
fn default_network_name() -> String {
    "starknet".to_string()
}

const fn default_network_timeout() -> u64 {
    30
}
const fn default_database_timeout() -> u64 {
    10
}
const fn default_crypto_timeout() -> u64 {
    60
}
const fn default_max_retries() -> u32 {
    3
}
const fn default_initial_retry_delay() -> u64 {
    1000
}
fn default_log_level() -> String {
    "info".to_string()
}
const fn default_structured_logging() -> bool {
    true
}
fn default_log_format() -> String {
    "pretty".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_validation() {
        let valid_config = ClientConfiguration {
            network: NetworkConfig {
                starknet_rpc_url: "https://starknet-mainnet.public.blastapi.io".to_string(),
                chain_id: 1,
                network_name: "mainnet".to_string(),
            },
            contracts: ContractConfig {
                l2_store_address: "0x1234567890abcdef".to_string(),
                verifier_address: "0xabcdef1234567890".to_string(),
            },
            auth: AuthConfig {
                starknet_private_key: "0x123456".to_string(),
                starknet_account_address: "0x987654321".to_string(),
            },
            processing: ProcessingConfig {
                polling_interval_secs: 10,
                batch_size: 100,
                start_block: 0,
                blocks_per_run: 1000,
                auto_start_block: false,
            },
            timeouts: TimeoutSettings {
                network_timeout_secs: 30,
                database_timeout_secs: 10,
                crypto_timeout_secs: 60,
                max_retries: 3,
                initial_retry_delay_ms: 1000,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                structured: true,
                format: "pretty".to_string(),
            },
        };

        assert!(valid_config.validate().is_ok());
    }

    #[test]
    fn test_invalid_configuration() {
        let invalid_config = ClientConfiguration {
            network: NetworkConfig {
                starknet_rpc_url: "invalid-url".to_string(), // Invalid URL
                chain_id: 1,
                network_name: "test".to_string(),
            },
            contracts: ContractConfig {
                l2_store_address: "0x1234567890abcdef".to_string(),
                verifier_address: "0xabcdef1234567890".to_string(),
            },
            auth: AuthConfig {
                starknet_private_key: "0x123456".to_string(),
                starknet_account_address: "0x987654321".to_string(),
            },
            processing: ProcessingConfig {
                polling_interval_secs: 10,
                batch_size: 100,
                start_block: 0,
                blocks_per_run: 1000,
                auto_start_block: false,
            },
            timeouts: TimeoutSettings {
                network_timeout_secs: 30,
                database_timeout_secs: 10,
                crypto_timeout_secs: 60,
                max_retries: 3,
                initial_retry_delay_ms: 1000,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                structured: true,
                format: "pretty".to_string(),
            },
        };

        assert!(invalid_config.validate().is_err());
    }
}
