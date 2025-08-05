//! Integration tests for the Fossil Light Client.
//!
//! These tests verify the complete integration of all client components
//! including configuration, event processing, MMR operations, and error handling.

use std::time::Duration;

use client::{
    async_utils::{with_timeout, TimeoutConfig},
    config_manager::{
        AuthConfig, ClientConfiguration, ContractConfig, LoggingConfig, NetworkConfig,
        ProcessingConfig, TimeoutSettings,
    },
    logging::{init_structured_logging, ClientContext, PerformanceLogger},
    types::*,
    ClientError,
};

/// Test configuration creation and validation.
#[test]
fn test_config_creation_and_validation() {
    let config = ClientConfiguration {
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

    assert!(config.validate().is_ok());
    assert_eq!(
        config.network.starknet_rpc_url,
        "https://starknet-mainnet.public.blastapi.io"
    );
    assert_eq!(config.network.chain_id, 1);
    assert_eq!(config.contracts.l2_store_address, "0x1234567890abcdef");
    assert_eq!(config.contracts.verifier_address, "0xabcdef1234567890");
}

/// Test basic file configuration loading.
#[test]
fn test_file_config_format() {
    // Test that the TOML format is structured correctly
    let config_content = r#"
[network]
starknet_rpc_url = "https://example.com/rpc"
chain_id = 2
network_name = "testnet"

[contracts]
l2_store_address = "0xdeadbeef"
verifier_address = "0xbeefdead"

[auth]
starknet_private_key = "0xprivatekey"
starknet_account_address = "0xaccount"

[processing]
polling_interval_secs = 15
batch_size = 50
start_block = 100
blocks_per_run = 200
auto_start_block = true

[timeouts]
network_timeout_secs = 45
database_timeout_secs = 15
crypto_timeout_secs = 90
max_retries = 5
initial_retry_delay_ms = 2000

[logging]
level = "debug"
structured = true
format = "json"
"#;

    // Test that the TOML can be parsed into our config structure
    let parsed: std::result::Result<ClientConfiguration, _> = toml::from_str(config_content);
    assert!(parsed.is_ok(), "TOML configuration should parse correctly");

    let config = parsed.unwrap();
    assert_eq!(config.network.starknet_rpc_url, "https://example.com/rpc");
    assert_eq!(config.network.chain_id, 2);
    assert_eq!(config.processing.polling_interval_secs, 15);
    assert_eq!(config.processing.batch_size, 50);
    assert_eq!(config.logging.level, "debug");
}

/// Test timeout utilities with successful operations.
#[tokio::test]
async fn test_timeout_success() {
    let result = with_timeout(
        async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok::<i32, std::io::Error>(42)
        },
        Duration::from_millis(100),
        "test operation",
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

/// Test timeout utilities with timeout.
#[tokio::test]
async fn test_timeout_failure() {
    let result = with_timeout(
        async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok::<i32, std::io::Error>(42)
        },
        Duration::from_millis(100),
        "slow operation",
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ClientError::OperationTimeout {
            operation,
            duration_ms,
        } => {
            assert_eq!(operation, "slow operation");
            assert_eq!(duration_ms, 100);
        }
        _ => panic!("Expected OperationTimeout error"),
    }
}

/// Test structured logging context building.
#[test]
fn test_client_context_building() {
    let context = ClientContext::with_block(100)
        .with_events(5)
        .with_chain_info(1, "0x123".to_string());

    assert_eq!(context.current_block, Some(100));
    assert_eq!(context.events_in_batch, Some(5));
    assert_eq!(context.chain_id, Some(1));
    assert_eq!(context.account_address, Some("0x123".to_string()));
}

/// Test performance logger.
#[tokio::test]
async fn test_performance_logger() {
    init_structured_logging("debug", false, "compact");

    let context = ClientContext::with_operation("test operation");

    let logger = PerformanceLogger::start_operation("integration test", context);

    tokio::time::sleep(Duration::from_millis(10)).await;
    logger.log_milestone("halfway", Some("test milestone"));

    tokio::time::sleep(Duration::from_millis(10)).await;
    logger.log_success(Some("test completed successfully"));
}

/// Test type validation and newtype wrappers.
#[test]
fn test_newtype_validation() {
    // Test BlockNumber
    assert!(PollingInterval::new(0).is_none()); // Invalid
    assert!(PollingInterval::new(1).is_some()); // Valid

    // Test BatchSize
    assert!(BatchSize::new(0).is_none()); // Invalid
    assert!(BatchSize::new(100).is_some()); // Valid

    // Test Address validation
    assert!(Address::new("0x1234567890abcdef").is_some()); // Valid
    assert!(Address::new("1234567890abcdef").is_none()); // Missing 0x prefix
    assert!(Address::new("0x").is_none()); // Too short
    assert!(Address::new("").is_none()); // Empty

    // Test ChainId
    let chain_id = ChainId::new(1);
    assert_eq!(chain_id.value(), 1);

    // Test predefined chain IDs (if they exist)
    // Note: These constants might not be implemented yet
    // assert_eq!(ChainId::MAINNET.value(), 1);
}

/// Test error type creation and context.
#[test]
fn test_error_types() {
    let timeout_error = ClientError::operation_timeout("test operation", 1000);
    assert!(matches!(
        timeout_error,
        ClientError::OperationTimeout { .. }
    ));

    let async_error = ClientError::async_operation_failed(
        "test context",
        std::io::Error::new(std::io::ErrorKind::Other, "test error"),
    );
    assert!(matches!(
        async_error,
        ClientError::AsyncOperationFailed { .. }
    ));

    // Test error display
    let error_msg = format!("{}", timeout_error);
    assert!(error_msg.contains("timed out"));
    assert!(error_msg.contains("1000ms"));
    assert!(error_msg.contains("test operation"));
}

/// Test configuration validation.
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

    // Test timeout config conversion
    let timeout_config = valid_config.timeout_config();
    assert_eq!(timeout_config.network_timeout, Duration::from_secs(30));
    assert_eq!(timeout_config.database_timeout, Duration::from_secs(10));
    assert_eq!(timeout_config.crypto_timeout, Duration::from_secs(60));
}

/// Test invalid configuration detection.
#[test]
fn test_invalid_configuration() {
    let mut config = ClientConfiguration {
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

    // Should fail with invalid URL
    assert!(config.validate().is_err());

    // Fix URL but break polling interval
    config.network.starknet_rpc_url = "https://example.com".to_string();
    config.processing.polling_interval_secs = 0;
    assert!(config.validate().is_err());

    // Fix polling interval but break batch size
    config.processing.polling_interval_secs = 10;
    config.processing.batch_size = 0;
    assert!(config.validate().is_err());

    // Fix batch size but break address
    config.processing.batch_size = 100;
    config.contracts.l2_store_address = "invalid-address".to_string();
    assert!(config.validate().is_err());
}

/// Test TimeoutConfig default values.
#[test]
fn test_timeout_config_defaults() {
    let config = TimeoutConfig::default();

    assert_eq!(config.network_timeout, Duration::from_secs(30));
    assert_eq!(config.database_timeout, Duration::from_secs(10));
    assert_eq!(config.crypto_timeout, Duration::from_secs(60));
}

/// Test TimeoutConfig custom values.
#[test]
fn test_timeout_config_custom() {
    let config = TimeoutConfig::new(45, 15, 90);

    assert_eq!(config.network_timeout, Duration::from_secs(45));
    assert_eq!(config.database_timeout, Duration::from_secs(15));
    assert_eq!(config.crypto_timeout, Duration::from_secs(90));
}
