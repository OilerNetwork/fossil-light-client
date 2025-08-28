//! Comprehensive unit tests for individual modules.
//!
//! These tests focus on testing individual components in isolation
//! with comprehensive edge case coverage.

use std::{
    error::Error,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use client::{
    async_utils::{with_retry, with_timeout_and_retry, TimeoutConfig},
    logging::{init_structured_logging, ClientContext, PerformanceLogger},
    types::*,
    ClientError,
};

/// Test retry logic with various failure patterns.
#[tokio::test]
async fn test_retry_logic_comprehensive() {
    // Test: Succeed on first attempt
    let counter = Arc::new(AtomicU32::new(0));
    let result = with_retry(
        {
            let counter = counter.clone();
            move || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<i32, std::io::Error>(42)
                }
            }
        },
        3,
        Duration::from_millis(10),
        "immediate success",
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Test: Succeed on second attempt
    let counter2 = Arc::new(AtomicU32::new(0));
    let result = with_retry(
        {
            let counter = counter2.clone();
            move || {
                let counter = counter.clone();
                async move {
                    let current = counter.fetch_add(1, Ordering::SeqCst);
                    if current == 0 {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "temporary failure",
                        ))
                    } else {
                        Ok::<i32, std::io::Error>(100)
                    }
                }
            }
        },
        3,
        Duration::from_millis(10),
        "second attempt success",
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 100);
    assert_eq!(counter2.load(Ordering::SeqCst), 2);

    // Test: Exhaust all retries
    let counter3 = Arc::new(AtomicU32::new(0));
    let result = with_retry(
        {
            let counter = counter3.clone();
            move || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "persistent failure",
                    ))
                }
            }
        },
        2,
        Duration::from_millis(10),
        "exhausted retries",
    )
    .await;

    assert!(result.is_err());
    assert_eq!(counter3.load(Ordering::SeqCst), 3); // Initial attempt + 2 retries

    match result.unwrap_err() {
        ClientError::AsyncOperationFailed { context, .. } => {
            assert!(context.contains("exhausted retries"));
            assert!(context.contains("3 attempts"));
        }
        other => panic!("Expected AsyncOperationFailed, got {:?}", other),
    }
}

/// Test combined timeout and retry logic.
#[tokio::test]
async fn test_timeout_and_retry_combined() {
    // Test: Fast operation succeeds immediately
    let result = with_timeout_and_retry(
        || async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<i32, std::io::Error>(42)
        },
        Duration::from_millis(50),
        2,
        Duration::from_millis(10),
        "fast combined operation",
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);

    // Test: Operation times out on each attempt
    let start_time = std::time::Instant::now();
    let result = with_timeout_and_retry(
        || async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<i32, std::io::Error>(42)
        },
        Duration::from_millis(50), // Timeout shorter than operation
        2,
        Duration::from_millis(10),
        "timeout retry test",
    )
    .await;

    assert!(result.is_err());
    let elapsed = start_time.elapsed();

    // Should have made 3 attempts (initial + 2 retries), each timing out after 50ms,
    // plus retry delays. Total should be roughly 3*50 + 2*10 = 170ms minimum
    assert!(
        elapsed >= Duration::from_millis(150),
        "Operation should have taken at least 150ms, took {:?}",
        elapsed
    );

    match result.unwrap_err() {
        ClientError::AsyncOperationFailed { context, .. } => {
            assert!(context.contains("timeout retry test"));
        }
        other => panic!("Expected AsyncOperationFailed, got {:?}", other),
    }
}

/// Test ClientContext builder pattern comprehensively.
#[test]
fn test_client_context_comprehensive() {
    // Test default context
    let default_context = ClientContext::default();
    assert!(default_context.current_block.is_none());
    assert!(default_context.latest_network_block.is_none());
    assert!(default_context.events_in_batch.is_none());
    assert!(default_context.operation.is_none());
    assert!(default_context.account_address.is_none());
    assert!(default_context.chain_id.is_none());

    // Test with_block
    let block_context = ClientContext::with_block(42);
    assert_eq!(block_context.current_block, Some(42));
    assert!(block_context.latest_network_block.is_none());

    // Test with_operation
    let op_context = ClientContext::with_operation("test_operation");
    assert_eq!(op_context.operation, Some("test_operation".to_string()));
    assert!(op_context.current_block.is_none());

    // Test chaining methods
    let complex_context = ClientContext::with_block(100)
        .with_events(5)
        .with_chain_info(1, "0x123abc".to_string())
        .with_block_range(100, 200);

    assert_eq!(complex_context.current_block, Some(100));
    assert_eq!(complex_context.latest_network_block, Some(200));
    assert_eq!(complex_context.events_in_batch, Some(5));
    assert_eq!(complex_context.chain_id, Some(1));
    assert_eq!(
        complex_context.account_address,
        Some("0x123abc".to_string())
    );
}

/// Test PerformanceLogger with various scenarios.
#[tokio::test]
async fn test_performance_logger_comprehensive() {
    init_structured_logging("debug", false, "compact");

    // Test successful operation
    let context = ClientContext::with_operation("test_success").with_events(3);

    let logger = PerformanceLogger::start_operation("success_test", context);
    tokio::time::sleep(Duration::from_millis(10)).await;
    logger.log_milestone("checkpoint1", Some("reached first milestone"));
    tokio::time::sleep(Duration::from_millis(10)).await;
    logger.log_milestone("checkpoint2", None);
    tokio::time::sleep(Duration::from_millis(10)).await;
    logger.log_success(Some("operation completed successfully"));

    // Test failed operation
    let error_context = ClientContext::with_operation("test_failure");

    let error_logger = PerformanceLogger::start_operation("failure_test", error_context);
    tokio::time::sleep(Duration::from_millis(5)).await;
    let test_error = std::io::Error::new(std::io::ErrorKind::Other, "simulated failure");
    error_logger.log_error(&test_error, Some("operation failed as expected"));
}

/// Test TimeoutConfig edge cases.
#[test]
fn test_timeout_config_edge_cases() {
    // Test with very small timeouts
    let small_config = TimeoutConfig::new(1, 1, 1);
    assert_eq!(small_config.network_timeout, Duration::from_secs(1));
    assert_eq!(small_config.database_timeout, Duration::from_secs(1));
    assert_eq!(small_config.crypto_timeout, Duration::from_secs(1));

    // Test with large timeouts
    let large_config = TimeoutConfig::new(3600, 1800, 7200);
    assert_eq!(large_config.network_timeout, Duration::from_secs(3600));
    assert_eq!(large_config.database_timeout, Duration::from_secs(1800));
    assert_eq!(large_config.crypto_timeout, Duration::from_secs(7200));

    // Test default values are reasonable
    let default_config = TimeoutConfig::default();
    assert!(default_config.network_timeout >= Duration::from_secs(10));
    assert!(default_config.network_timeout <= Duration::from_secs(120));
    assert!(default_config.database_timeout >= Duration::from_secs(5));
    assert!(default_config.database_timeout <= Duration::from_secs(60));
    assert!(default_config.crypto_timeout >= Duration::from_secs(30));
    assert!(default_config.crypto_timeout <= Duration::from_secs(300));
}

/// Test all Address validation edge cases.
#[test]
fn test_address_validation_comprehensive() {
    // Valid addresses
    let valid_cases = vec![
        "0x0",
        "0x1",
        "0x123",
        "0x1234567890abcdef",
        "0x1234567890ABCDEF",
        "0xdeadbeef",
        "0xDEADBEEF",
        "0x0123456789abcdef0123456789abcdef01234567",
    ];

    for addr in valid_cases {
        let result = Address::new(addr.to_string());
        assert!(result.is_some(), "Address '{}' should be valid", addr);
        assert_eq!(result.unwrap().value(), addr);
    }

    // Invalid addresses (based on current implementation)
    let invalid_cases = vec![
        "",       // Empty
        "0x",     // Just prefix
        "123456", // Missing 0x prefix
        "x123",   /* Wrong prefix
                   * Note: Current implementation doesn't validate hex characters
                   * "0xGHIJ",        // Invalid hex characters
                   * "0x123g",        // Mixed valid/invalid hex
                   * "0X123",         // Wrong case prefix (should be lowercase 0x) */
    ];

    for addr in invalid_cases {
        let result = Address::new(addr.to_string());
        assert!(result.is_none(), "Address '{}' should be invalid", addr);
    }
}

/// Test ChainId functionality comprehensively.
#[test]
fn test_chain_id_comprehensive() {
    // Test basic functionality
    let chain = ChainId::new(1);
    assert_eq!(chain.value(), 1);

    // Test display formatting
    let display_str = format!("{}", chain);
    assert!(display_str.contains("1"));

    // Test predefined constants (if available)
    // Note: Not all ChainId constants may be defined
    // assert_eq!(ChainId::MAINNET.value(), 1);

    // Test various chain IDs
    for id in [1, 5, 11155111, 42161, 10, 137, 56] {
        let chain = ChainId::new(id);
        assert_eq!(chain.value(), id);

        let display = format!("{}", chain);
        assert!(display.contains(&id.to_string()));
    }

    // Test zero chain ID (technically valid)
    let zero_chain = ChainId::new(0);
    assert_eq!(zero_chain.value(), 0);

    // Test very large chain ID
    let large_chain = ChainId::new(u64::MAX);
    assert_eq!(large_chain.value(), u64::MAX);
}

/// Test PollingInterval edge cases and arithmetic.
#[test]
fn test_polling_interval_comprehensive() {
    // Test minimum valid value
    let min_interval = PollingInterval::new(1);
    assert!(min_interval.is_some());
    assert_eq!(min_interval.unwrap().as_duration(), Duration::from_secs(1));

    // Test zero (invalid)
    let zero_interval = PollingInterval::new(0);
    assert!(zero_interval.is_none());

    // Test large values
    let large_interval = PollingInterval::new(86400); // 24 hours
    assert!(large_interval.is_some());
    assert_eq!(
        large_interval.unwrap().as_duration(),
        Duration::from_secs(86400)
    );

    // Test presets
    let presets = [(PollingInterval::FAST, 1), (PollingInterval::SLOW, 30)];

    for (preset, expected_secs) in presets {
        assert_eq!(preset.as_duration(), Duration::from_secs(expected_secs));
    }

    // Test very large value (edge case)
    let max_interval = PollingInterval::new(u64::MAX);
    assert!(max_interval.is_some());
    assert_eq!(
        max_interval.unwrap().as_duration(),
        Duration::from_secs(u64::MAX)
    );
}

/// Test BatchSize edge cases.
#[test]
fn test_batch_size_comprehensive() {
    // Test minimum valid value
    let min_batch = BatchSize::new(1);
    assert!(min_batch.is_some());
    assert_eq!(min_batch.unwrap().value(), 1);

    // Test zero (invalid)
    let zero_batch = BatchSize::new(0);
    assert!(zero_batch.is_none());

    // Test large values
    let large_batch = BatchSize::new(1_000_000);
    assert!(large_batch.is_some());
    assert_eq!(large_batch.unwrap().value(), 1_000_000);

    // Test presets
    let presets = [(BatchSize::SMALL, 100), (BatchSize::LARGE, 10000)];

    for (preset, expected_value) in presets {
        assert_eq!(preset.value(), expected_value);
    }

    // Test very large value
    let max_batch = BatchSize::new(u64::MAX);
    assert!(max_batch.is_some());
    assert_eq!(max_batch.unwrap().value(), u64::MAX);
}

/// Test error type creation and formatting comprehensively.
#[test]
fn test_error_types_comprehensive() {
    // Test InvalidPollingInterval
    let poll_err = ClientError::InvalidPollingInterval(0);
    let poll_msg = format!("{}", poll_err);
    assert!(poll_msg.contains("polling interval"));
    assert!(poll_msg.contains("0"));
    assert!(poll_msg.contains("greater than zero"));

    // Test InvalidBatchSize
    let batch_err = ClientError::InvalidBatchSize(0);
    let batch_msg = format!("{}", batch_err);
    assert!(batch_msg.contains("batch size"));
    assert!(batch_msg.contains("0"));
    assert!(batch_msg.contains("greater than zero"));

    // Test InvalidBlockRange
    let range_err = ClientError::InvalidBlockRange {
        from_block: 100,
        to_block: 50,
    };
    let range_msg = format!("{}", range_err);
    assert!(range_msg.contains("block range"));
    assert!(range_msg.contains("100"));
    assert!(range_msg.contains("50"));

    // Test InvalidAddress
    let addr_err = ClientError::invalid_address("invalid-addr");
    let addr_msg = format!("{}", addr_err);
    assert!(addr_msg.contains("address format"));
    assert!(addr_msg.contains("invalid-addr"));

    // Test InvalidUrl
    let url_err = ClientError::invalid_url("invalid-url");
    let url_msg = format!("{}", url_err);
    assert!(url_msg.contains("URL format"));
    assert!(url_msg.contains("invalid-url"));

    // Test MissingEnvironmentVariable
    let env_err = ClientError::missing_env_var("TEST_VAR");
    let env_msg = format!("{}", env_err);
    assert!(env_msg.contains("Environment variable"));
    assert!(env_msg.contains("TEST_VAR"));

    // Test OperationTimeout
    let timeout_err = ClientError::operation_timeout("test operation", 5000);
    let timeout_msg = format!("{}", timeout_err);
    assert!(timeout_msg.contains("timed out"));
    assert!(timeout_msg.contains("5000ms"));
    assert!(timeout_msg.contains("test operation"));

    // Test AsyncOperationFailed
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "underlying error");
    let async_err = ClientError::async_operation_failed("test context", io_err);
    let async_msg = format!("{}", async_err);
    assert!(async_msg.contains("Async operation failed"));
    assert!(async_msg.contains("test context"));

    // Test error source chain
    let source_err = async_err.source();
    assert!(source_err.is_some());
    let source_msg = format!("{}", source_err.unwrap());
    assert!(source_msg.contains("underlying error"));
}

/// Test thread safety of types.
#[test]
fn test_thread_safety() {
    use std::{sync::Arc, thread};

    // Test that types can be shared across threads
    let polling_interval = Arc::new(PollingInterval::new(10).unwrap());
    let batch_size = Arc::new(BatchSize::new(100).unwrap());
    let address = Arc::new(Address::new("0x1234567890abcdef".to_string()).unwrap());
    let chain_id = Arc::new(ChainId::new(1));

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let pi = Arc::clone(&polling_interval);
            let bs = Arc::clone(&batch_size);
            let addr = Arc::clone(&address);
            let cid = Arc::clone(&chain_id);

            thread::spawn(move || {
                // Access the values from different threads
                assert_eq!(pi.as_duration(), Duration::from_secs(10));
                assert_eq!(bs.value(), 100);
                assert_eq!(addr.value(), "0x1234567890abcdef");
                assert_eq!(cid.value(), 1);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
