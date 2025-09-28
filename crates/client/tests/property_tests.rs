//! Property-based tests for the Fossil Light Client.
//!
//! These tests use the proptest library to generate random inputs and verify
//! that the client behaves correctly across a wide range of scenarios.

use std::time::Duration;

use client::{
    async_utils::{with_timeout, TimeoutConfig},
    config_manager::{
        AuthConfig, ClientConfiguration, ContractConfig, LoggingConfig, NetworkConfig,
        ProcessingConfig, TimeoutSettings,
    },
    types::*,
    ClientError,
};
use proptest::prelude::*;

// Property test strategies for generating test data

/// Strategy for generating valid polling intervals (1 to 3600 seconds).
fn polling_interval_strategy() -> impl Strategy<Value = u64> {
    1u64..=3600
}

/// Strategy for generating valid batch sizes (1 to 10000).
fn batch_size_strategy() -> impl Strategy<Value = u64> {
    1u64..=10000
}

/// Strategy for generating valid block numbers.
fn block_number_strategy() -> impl Strategy<Value = u64> {
    0u64..=1_000_000
}

/// Strategy for generating valid chain IDs.
fn chain_id_strategy() -> impl Strategy<Value = u64> {
    1u64..=100000
}

/// Strategy for generating valid Ethereum-style addresses.
fn address_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<u8>(), 20).prop_map(|bytes| format!("0x{}", hex::encode(bytes)))
}

/// Strategy for generating valid URLs.
fn url_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("https://starknet-mainnet.public.blastapi.io".to_string()),
        Just("https://alpha4.starknet.io".to_string()),
        Just("http://localhost:5050".to_string()),
        Just("https://starknet-goerli.public.blastapi.io".to_string()),
    ]
}

/// Strategy for generating timeout values (1 to 300 seconds).
fn timeout_strategy() -> impl Strategy<Value = u64> {
    1u64..=300
}

proptest! {
    /// Test that PollingInterval accepts valid values and rejects invalid ones.
    #[test]
    fn test_polling_interval_validation(
        valid_interval in polling_interval_strategy(),
        invalid_interval in 0u64..=0
    ) {
        // Valid intervals should succeed
        let valid = PollingInterval::new(valid_interval);
        prop_assert!(valid.is_some());
        prop_assert_eq!(valid.unwrap().as_duration(), Duration::from_secs(valid_interval));

        // Invalid intervals should fail
        let invalid = PollingInterval::new(invalid_interval);
        prop_assert!(invalid.is_none());
    }

    /// Test that BatchSize accepts valid values and rejects invalid ones.
    #[test]
    fn test_batch_size_validation(
        valid_size in batch_size_strategy(),
        invalid_size in 0u64..=0
    ) {
        // Valid batch sizes should succeed
        let valid = BatchSize::new(valid_size);
        prop_assert!(valid.is_some());
        prop_assert_eq!(valid.unwrap().value(), valid_size);

        // Invalid batch sizes should fail
        let invalid = BatchSize::new(invalid_size);
        prop_assert!(invalid.is_none());
    }

    /// Test that ChainId handles various chain ID values correctly.
    #[test]
    fn test_chain_id_handling(chain_id in chain_id_strategy()) {
        let chain = ChainId::new(chain_id);
        prop_assert_eq!(chain.value(), chain_id);

        // Test display formatting
        let display_str = format!("{}", chain);
        prop_assert!(display_str.contains(&chain_id.to_string()));
    }

    /// Test that Address validation works correctly for various inputs.
    #[test]
    fn test_address_validation(
        valid_addr in address_strategy(),
        invalid_addr in "[a-fA-F0-9]{1,39}" // Invalid length or missing 0x
    ) {
        // Valid addresses should succeed
        let valid = Address::new(valid_addr.clone());
        prop_assert!(valid.is_some());
        let valid_address = valid.unwrap();
        prop_assert_eq!(valid_address.value(), valid_addr);

        // Invalid addresses should fail (unless they happen to be valid)
        if !invalid_addr.starts_with("0x") || invalid_addr.len() < 3 {
            let invalid = Address::new(invalid_addr);
            prop_assert!(invalid.is_none());
        }
    }

    /// Test timeout configuration with various values.
    #[test]
    fn test_timeout_config_properties(
        network_timeout in timeout_strategy(),
        database_timeout in timeout_strategy(),
        crypto_timeout in timeout_strategy()
    ) {
        let config = TimeoutConfig::new(network_timeout, database_timeout, crypto_timeout);

        prop_assert_eq!(config.network_timeout, Duration::from_secs(network_timeout));
        prop_assert_eq!(config.database_timeout, Duration::from_secs(database_timeout));
        prop_assert_eq!(config.crypto_timeout, Duration::from_secs(crypto_timeout));
    }

    /// Test configuration validation with randomly generated valid configurations.
    #[test]
    fn test_configuration_validation_properties(
        rpc_url in url_strategy(),
        chain_id in chain_id_strategy(),
        store_addr in address_strategy(),
        verifier_addr in address_strategy(),
        account_addr in address_strategy(),
        polling_interval in polling_interval_strategy(),
        batch_size in batch_size_strategy(),
        start_block in block_number_strategy(),
        blocks_per_run in 0u64..=1000,
        network_timeout in timeout_strategy(),
        database_timeout in timeout_strategy(),
        crypto_timeout in timeout_strategy()
    ) {
        let config = ClientConfiguration {
            network: NetworkConfig {
                starknet_rpc_url: rpc_url,
                chain_id,
                network_name: "test".to_string(),
            },
            contracts: ContractConfig {
                l2_store_address: store_addr,
                verifier_address: verifier_addr,
            },
            auth: AuthConfig {
                starknet_private_key: "0x123456".to_string(),
                starknet_account_address: account_addr,
            },
            processing: ProcessingConfig {
                polling_interval_secs: polling_interval,
                batch_size,
                start_block,
                blocks_per_run,
                auto_start_block: false,
            },
            timeouts: TimeoutSettings {
                network_timeout_secs: network_timeout,
                database_timeout_secs: database_timeout,
                crypto_timeout_secs: crypto_timeout,
                max_retries: 3,
                initial_retry_delay_ms: 1000,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                structured: true,
                format: "pretty".to_string(),
            },
        };

        // All generated configurations should be valid
        prop_assert!(config.validate().is_ok());

        // Test conversion to legacy config
        let legacy_config = config.to_legacy_config();
        prop_assert!(legacy_config.is_ok());

        let legacy = legacy_config.unwrap();
        prop_assert_eq!(legacy.polling_interval.as_duration(), Duration::from_secs(polling_interval));
        prop_assert_eq!(legacy.batch_size.value(), batch_size);
        prop_assert_eq!(legacy.start_block, start_block);
        prop_assert_eq!(legacy.blocks_per_run, blocks_per_run);
        prop_assert_eq!(legacy.chain_id.value(), chain_id);
    }

    /// Test error creation with various inputs.
    #[test]
    fn test_error_creation_properties(
        operation_name in "[a-zA-Z0-9 ]{1,100}",
        timeout_ms in 1u64..=60000,
        context_desc in "[a-zA-Z0-9 ]{1,200}",
        polling_interval in 0u64..=0, // Invalid values only
        batch_size in 0u64..=0        // Invalid values only
    ) {
        // Test timeout error creation
        let timeout_error = ClientError::operation_timeout(&operation_name, timeout_ms);
        let timeout_msg = format!("{}", timeout_error);
        prop_assert!(timeout_msg.contains(&operation_name));
        prop_assert!(timeout_msg.contains(&timeout_ms.to_string()));

        // Test async operation error
        let io_error = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        let async_error = ClientError::async_operation_failed(&context_desc, io_error);
        let async_msg = format!("{}", async_error);
        prop_assert!(async_msg.contains(&context_desc));

        // Test validation errors
        let polling_error = ClientError::InvalidPollingInterval(polling_interval);
        let polling_msg = format!("{}", polling_error);
        prop_assert!(polling_msg.contains("polling interval"));
        prop_assert!(polling_msg.contains(&polling_interval.to_string()));

        let batch_error = ClientError::InvalidBatchSize(batch_size);
        let batch_msg = format!("{}", batch_error);
        prop_assert!(batch_msg.contains("batch size"));
        prop_assert!(batch_msg.contains(&batch_size.to_string()));
    }
}

// Async property tests require a different approach

/// Test timeout behavior with various durations.
#[tokio::test]
async fn test_timeout_behavior_properties() {
    // Test a range of timeout values
    for timeout_ms in [10, 50, 100, 200, 500] {
        let timeout_duration = Duration::from_millis(timeout_ms);

        // Fast operation should succeed
        let fast_result = with_timeout(
            async {
                tokio::time::sleep(Duration::from_millis(timeout_ms / 2)).await;
                Ok::<i32, std::io::Error>(42)
            },
            timeout_duration,
            "fast operation",
        )
        .await;

        assert!(
            fast_result.is_ok(),
            "Fast operation should succeed with timeout {}ms",
            timeout_ms
        );

        // Slow operation should timeout
        let slow_result = with_timeout(
            async {
                tokio::time::sleep(Duration::from_millis(timeout_ms * 2)).await;
                Ok::<i32, std::io::Error>(42)
            },
            timeout_duration,
            "slow operation",
        )
        .await;

        assert!(
            slow_result.is_err(),
            "Slow operation should timeout with timeout {}ms",
            timeout_ms
        );

        // Verify it's the right kind of error
        match slow_result.unwrap_err() {
            ClientError::OperationTimeout { duration_ms, .. } => {
                assert_eq!(duration_ms, timeout_ms, "Timeout duration should match");
            }
            other => panic!("Expected OperationTimeout, got {:?}", other),
        }
    }
}

/// Test that block range calculations work correctly for various inputs.
#[test]
fn test_block_range_calculations() {
    use client::events::EventProcessor;
    use starknet_handler::provider::StarknetProvider;

    let test_cases = vec![
        // (start_block, blocks_per_run, latest_block, expected_from, expected_to)
        (100, 50, 200, 100, 149),   // Normal case
        (100, 0, 200, 100, 200),    // Unlimited blocks_per_run
        (100, 1000, 200, 100, 200), // blocks_per_run larger than available
        (1, 100, 50, 1, 50),        // Starting from block 1
        (50, 10, 55, 50, 55),       // Small range
    ];

    for (start_block, blocks_per_run, latest_block, expected_from, expected_to) in test_cases {
        let provider = StarknetProvider::new("http://localhost:5050").unwrap();
        let processor = EventProcessor::new(
            provider,
            "0x1234".to_string(),
            start_block, // EventProcessor will set latest_processed_block to start_block - 1
            blocks_per_run,
            None,
        );

        let result = processor.calculate_block_range(latest_block);
        assert!(result.is_ok(), "Block range calculation should succeed for case: start={}, blocks_per_run={}, latest={}",
                start_block, blocks_per_run, latest_block);

        let (from, to) = result.unwrap();
        assert_eq!(
            from, expected_from,
            "From block should match for case: start={}, blocks_per_run={}, latest={}",
            start_block, blocks_per_run, latest_block
        );
        assert_eq!(
            to, expected_to,
            "To block should match for case: start={}, blocks_per_run={}, latest={}",
            start_block, blocks_per_run, latest_block
        );
    }
}

/// Test invalid block range detection.
#[test]
fn test_invalid_block_ranges() {
    use client::events::EventProcessor;
    use starknet_handler::provider::StarknetProvider;

    // Test cases that should result in invalid ranges
    let invalid_cases = vec![
        (200, 50, 150), // start_block higher than latest_block
        (100, 50, 99),  // latest_block less than current position
    ];

    for (start_block, blocks_per_run, latest_block) in invalid_cases {
        let provider = StarknetProvider::new("http://localhost:5050").unwrap();
        let processor =
            EventProcessor::new(provider, "0x1234".to_string(), start_block, blocks_per_run, None);

        let result = processor.calculate_block_range(latest_block);
        assert!(
            result.is_err(),
            "Should detect invalid block range for: start={}, latest={}",
            start_block,
            latest_block
        );

        match result.unwrap_err() {
            ClientError::InvalidBlockRange {
                from_block,
                to_block,
            } => {
                assert!(
                    from_block > to_block,
                    "From block should be greater than to block"
                );
            }
            other => panic!("Expected InvalidBlockRange error, got {:?}", other),
        }
    }
}
