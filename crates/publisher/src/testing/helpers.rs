/// Test helper functions and utilities
///
/// This module provides common helper functions that are useful
/// across multiple test modules.
use crate::{
    config::{AccountConfig, PublisherConfig},
    error::PublisherResult,
};

/// Creates a test configuration with custom values
pub fn create_custom_test_config(rpc_url: &str, chain_id: u64, batch_size: u64) -> PublisherConfig {
    PublisherConfig {
        rpc_url: rpc_url.to_string(),
        chain_id,
        verifier_address: "0x1234567890123456789012345678901234567890".to_string(),
        store_address: "0x0987654321098765432109876543210987654321".to_string(),
        batch_size,
    }
}

/// Creates an invalid configuration for testing error conditions
pub fn create_invalid_config() -> PublisherConfig {
    PublisherConfig {
        rpc_url: "".to_string(), // Invalid empty URL
        chain_id: 0,
        verifier_address: "".to_string(), // Invalid empty address
        store_address: "".to_string(),    // Invalid empty address
        batch_size: 0,                    // Invalid batch size
    }
}

/// Creates a test account configuration with custom values
pub fn create_custom_account_config(private_key: &str, address: &str) -> AccountConfig {
    AccountConfig::new(private_key.to_string(), address.to_string())
}

/// Creates an invalid account configuration for testing
pub fn create_invalid_account_config() -> AccountConfig {
    AccountConfig::new("".to_string(), "".to_string())
}

/// Test assertion helpers
pub mod assertions {
    use super::*;

    /// Asserts that a result is a specific PublisherError variant
    pub fn assert_publisher_error_type(result: &PublisherResult<()>, expected_msg_contains: &str) {
        match result {
            Err(err) => assert!(err.to_string().contains(expected_msg_contains)),
            Ok(_) => panic!("Expected error but got Ok"),
        }
    }

    /// Asserts that a configuration is valid
    pub fn assert_valid_config(config: &PublisherConfig) {
        assert!(!config.rpc_url.is_empty(), "RPC URL should not be empty");
        assert!(
            !config.verifier_address.is_empty(),
            "Verifier address should not be empty"
        );
        assert!(
            !config.store_address.is_empty(),
            "Store address should not be empty"
        );
        assert!(config.batch_size > 0, "Batch size should be greater than 0");
    }

    /// Asserts that an account configuration is valid
    pub fn assert_valid_account_config(config: &AccountConfig) {
        assert!(
            !config.private_key.is_empty(),
            "Private key should not be empty"
        );
        assert!(!config.address.is_empty(), "Address should not be empty");
    }
}

/// Mock data generators
pub mod generators {

    /// Generates a sequence of test block hashes
    pub fn generate_block_hashes(count: usize) -> Vec<String> {
        (0..count).map(|i| format!("0x{:0>64x}", i)).collect()
    }

    /// Generates test batch indices
    pub fn generate_batch_indices(start: u64, count: usize) -> Vec<u64> {
        (start..start + count as u64).collect()
    }

    /// Generates a test proof structure
    pub fn generate_test_proof() -> mmr::Proof {
        mmr::Proof {
            element_index: 1,
            element_hash: "0x1234".to_string(),
            siblings_hashes: vec!["0x5678".to_string()],
            peaks_hashes: vec!["0x9abc".to_string()],
            elements_count: 10,
        }
    }
}

/// Test environment setup helpers
pub mod setup {
    use std::env;

    use crate::error::{PublisherError, PublisherResult};

    /// Sets up test environment variables
    pub fn setup_test_env() {
        env::set_var("RUST_LOG", "debug");
        env::set_var("RUST_BACKTRACE", "1");
    }

    /// Creates a temporary test directory
    pub fn create_temp_test_dir() -> PublisherResult<std::path::PathBuf> {
        use std::fs;
        let temp_dir =
            std::env::temp_dir().join(format!("publisher_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir)
            .map_err(|e| PublisherError::io(format!("Failed to create temp dir: {}", e)))?;
        Ok(temp_dir)
    }

    /// Cleans up temporary test directory
    pub fn cleanup_temp_dir(path: &std::path::Path) -> PublisherResult<()> {
        use std::fs;
        fs::remove_dir_all(path)
            .map_err(|e| PublisherError::io(format!("Failed to cleanup temp dir: {}", e)))
    }
}

/// Performance testing utilities
pub mod performance {
    use std::time::{Duration, Instant};

    /// Measures execution time of a function
    pub async fn measure_async<F, Fut, T>(f: F) -> (T, Duration)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        (result, duration)
    }

    /// Asserts that execution time is within expected bounds
    pub fn assert_execution_time(duration: Duration, max_expected: Duration) {
        assert!(
            duration <= max_expected,
            "Execution took {:?} but expected at most {:?}",
            duration,
            max_expected
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::constants::*;

    #[test]
    fn test_create_custom_test_config() {
        let config = super::create_custom_test_config(TEST_RPC_URL, TEST_CHAIN_ID, TEST_BATCH_SIZE);

        assert_eq!(config.rpc_url, TEST_RPC_URL);
        assert_eq!(config.chain_id, TEST_CHAIN_ID);
        assert_eq!(config.batch_size, TEST_BATCH_SIZE);
        super::assertions::assert_valid_config(&config);
    }

    #[test]
    fn test_create_invalid_config() {
        let config = super::create_invalid_config();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_generate_block_hashes() {
        let hashes = super::generators::generate_block_hashes(3);
        assert_eq!(hashes.len(), 3);
        assert_eq!(
            hashes[0],
            "0x0000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(
            hashes[1],
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        );
        assert_eq!(
            hashes[2],
            "0x0000000000000000000000000000000000000000000000000000000000000002"
        );
    }

    #[test]
    fn test_generate_batch_indices() {
        let indices = super::generators::generate_batch_indices(5, 3);
        assert_eq!(indices, vec![5, 6, 7]);
    }

    #[tokio::test]
    async fn test_measure_async() {
        let (result, duration) = super::performance::measure_async(|| async {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            42
        })
        .await;

        assert_eq!(result, 42);
        assert!(duration >= tokio::time::Duration::from_millis(10));
    }
}
