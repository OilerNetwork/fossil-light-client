/// Integration tests for the publisher crate
///
/// These tests verify that the publisher components work correctly
/// together and maintain backward compatibility.
use publisher::{
    api::operations::get_block_hash_inclusion_proof,
    config::{AccountConfig, PublisherConfig},
    error::PublisherError,
    prove_mmr_update_with_config,
    service::ProofService,
    update_mmr_with_config, ProveMMRUpdateConfigBuilder, UpdateMMRConfigBuilder,
};
use starknet_handler::provider::LatestRelayBlock;

/// Test utilities for integration tests
mod utils {
    use super::*;

    pub fn create_test_config() -> (String, u64, String, String, u64) {
        (
            "http://localhost:8545".to_string(), // rpc_url
            1,                                   // chain_id
            "0x1234567890123456789012345678901234567890".to_string(), // verifier_address
            "0x0987654321098765432109876543210987654321".to_string(), // store_address
            100,                                 // batch_size
        )
    }

    pub fn create_test_account() -> (String, String) {
        (
            "0xprivatekey123456789".to_string(), // private_key
            "0xaddress123456789".to_string(),    // address
        )
    }

    pub fn create_mock_latest_relay_block() -> LatestRelayBlock {
        LatestRelayBlock {
            block_number: 100,
            block_hash: "0xmockblockhash".to_string(),
        }
    }
}

/// Test backward compatibility of API functions
#[tokio::test]
#[ignore = "Requires external dependencies (database, network)"]
async fn test_api_backward_compatibility() {
    let (rpc_url, chain_id, verifier_address, store_address, batch_size) =
        utils::create_test_config();
    let (private_key, address) = utils::create_test_account();
    let latest_relay_block = utils::create_mock_latest_relay_block();

    // Test prove_mmr_update_with_config API compatibility
    let config = ProveMMRUpdateConfigBuilder::new()
        .rpc_url(&rpc_url)
        .chain_id(chain_id)
        .verifier_address(&verifier_address)
        .store_address(&store_address)
        .account_private_key(&private_key)
        .account_address(&address)
        .batch_size(batch_size)
        .start_block(1)
        .latest_relayed_block(latest_relay_block.clone())
        .build()
        .expect("Config build should succeed");

    let result = prove_mmr_update_with_config(config).await;

    // We expect this to fail in test environment, but the API should be callable
    assert!(result.is_err());

    // Test update_mmr_with_config API compatibility
    let config = UpdateMMRConfigBuilder::new()
        .rpc_url(&rpc_url)
        .chain_id(chain_id)
        .verifier_address(&verifier_address)
        .store_address(&store_address)
        .account_private_key(&private_key)
        .account_address(&address)
        .batch_size(batch_size)
        .start_block(1)
        .latest_relayed_block(latest_relay_block)
        .build()
        .expect("Config build should succeed");

    let result = update_mmr_with_config(config).await;

    // We expect this to fail in test environment, but the API should be callable
    assert!(result.is_err());
}

#[tokio::test]
#[ignore = "Requires external dependencies (database, IPFS)"]
async fn test_get_block_hash_inclusion_proof_api() {
    let test_block_hash =
        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string();
    let rpc_url = "http://localhost:8545".to_string();
    let store_address = "0x0987654321098765432109876543210987654321".to_string();
    let batch_size = 100;

    let result =
        get_block_hash_inclusion_proof(test_block_hash, rpc_url, store_address, batch_size).await;

    // We expect this to fail in test environment, but the API should be callable
    assert!(result.is_err());
}

/// Test service layer integration
#[tokio::test]
async fn test_service_integration() {
    let config = PublisherConfig {
        rpc_url: "http://localhost:8545".to_string(),
        chain_id: 1,
        verifier_address: "0x1234567890123456789012345678901234567890".to_string(),
        store_address: "0x0987654321098765432109876543210987654321".to_string(),
        batch_size: 100,
    };

    // Test service creation with config
    let _service = ProofService::with_config(config.clone());
    // Service should be created successfully - we can't test private fields directly

    // Test legacy service creation
    let _legacy_service = ProofService::new(
        config.rpc_url.clone(),
        config.chain_id,
        config.verifier_address.clone(),
        config.store_address.clone(),
    );
    // Legacy service should be created successfully
}

/// Test configuration validation integration
#[test]
fn test_configuration_validation_integration() {
    // Test valid configuration
    let valid_config = PublisherConfig {
        rpc_url: "http://localhost:8545".to_string(),
        chain_id: 1,
        verifier_address: "0x1234567890123456789012345678901234567890".to_string(),
        store_address: "0x0987654321098765432109876543210987654321".to_string(),
        batch_size: 100,
    };

    assert!(valid_config.validate().is_ok());

    // Test invalid configurations
    let invalid_configs = vec![
        PublisherConfig {
            rpc_url: "".to_string(), // Empty RPC URL
            chain_id: 1,
            verifier_address: "0x1234567890123456789012345678901234567890".to_string(),
            store_address: "0x0987654321098765432109876543210987654321".to_string(),
            batch_size: 100,
        },
        PublisherConfig {
            rpc_url: "http://localhost:8545".to_string(),
            chain_id: 1,
            verifier_address: "".to_string(), // Empty verifier address
            store_address: "0x0987654321098765432109876543210987654321".to_string(),
            batch_size: 100,
        },
        PublisherConfig {
            rpc_url: "http://localhost:8545".to_string(),
            chain_id: 1,
            verifier_address: "0x1234567890123456789012345678901234567890".to_string(),
            store_address: "".to_string(), // Empty store address
            batch_size: 100,
        },
        PublisherConfig {
            rpc_url: "http://localhost:8545".to_string(),
            chain_id: 1,
            verifier_address: "0x1234567890123456789012345678901234567890".to_string(),
            store_address: "0x0987654321098765432109876543210987654321".to_string(),
            batch_size: 0, // Invalid batch size
        },
    ];

    for config in invalid_configs {
        assert!(config.validate().is_err());
    }
}

/// Test account configuration validation
#[test]
fn test_account_configuration_validation() {
    // Test valid account configuration
    let valid_account = AccountConfig::new("private_key".to_string(), "address".to_string());

    assert!(valid_account.validate().is_ok());

    // Test invalid account configurations
    let invalid_accounts = vec![
        AccountConfig::new("".to_string(), "address".to_string()), // Empty private key
        AccountConfig::new("private_key".to_string(), "".to_string()), // Empty address
        AccountConfig::new("".to_string(), "".to_string()),        // Both empty
    ];

    for account in invalid_accounts {
        assert!(account.validate().is_err());
    }
}

/// Test error handling integration
#[test]
fn test_error_handling_integration() {
    use publisher::error::PublisherError;

    // Test error creation and conversion
    let db_error = PublisherError::database("Database connection failed");
    assert!(matches!(db_error, PublisherError::Database(_)));
    assert!(db_error.to_string().contains("Database connection failed"));

    let ipfs_error = PublisherError::ipfs("IPFS timeout");
    assert!(matches!(ipfs_error, PublisherError::Ipfs(_)));

    let proof_error = PublisherError::proof_generation("Invalid proof");
    assert!(matches!(proof_error, PublisherError::ProofGeneration(_)));

    // Test error into eyre conversion
    let eyre_error = db_error.into_eyre();
    assert!(eyre_error
        .to_string()
        .contains("Database connection failed"));
}

/// Test configuration builder pattern
#[test]
fn test_configuration_builder_integration() {
    // Builder pattern test is included

    let config = PublisherConfig::builder()
        .rpc_url("http://localhost:8545")
        .chain_id(1)
        .verifier_address("0x1234567890123456789012345678901234567890")
        .store_address("0x0987654321098765432109876543210987654321")
        .batch_size(100)
        .build()
        .expect("Should build valid config");

    assert_eq!(config.rpc_url, "http://localhost:8545");
    assert_eq!(config.chain_id, 1);
    assert_eq!(config.batch_size, 100);

    // Test missing required field
    let result = PublisherConfig::builder()
        .chain_id(1)
        .batch_size(100)
        .build();

    assert!(result.is_err());
}

/// Performance integration test
#[tokio::test]
async fn test_performance_integration() {
    use std::time::{Duration, Instant};

    let config = PublisherConfig {
        rpc_url: "http://localhost:8545".to_string(),
        chain_id: 1,
        verifier_address: "0x1234567890123456789012345678901234567890".to_string(),
        store_address: "0x0987654321098765432109876543210987654321".to_string(),
        batch_size: 100,
    };

    // Test service creation performance
    let start = Instant::now();
    let _service = ProofService::with_config(config.clone());
    let creation_time = start.elapsed();

    // Service creation should be very fast (< 1ms)
    assert!(creation_time < Duration::from_millis(1));

    // Test configuration validation performance
    let start = Instant::now();
    let result = config.validate();
    let validation_time = start.elapsed();

    assert!(result.is_ok());
    assert!(validation_time < Duration::from_millis(1));
}

/// Regression test to ensure backward compatibility
#[test]
fn test_backward_compatibility_regression() {
    // Test that all public APIs are still available and callable

    // Configuration types should be available
    let _config: PublisherConfig = PublisherConfig {
        rpc_url: "test".to_string(),
        chain_id: 1,
        verifier_address: "test".to_string(),
        store_address: "test".to_string(),
        batch_size: 100,
    };

    let _account: AccountConfig =
        AccountConfig::new("private_key".to_string(), "address".to_string());

    // Service types should be available
    let _service = ProofService::new(
        "http://localhost:8545".to_string(),
        1,
        "0x123".to_string(),
        "0x456".to_string(),
    );

    // Error types should be available
    let _error: PublisherError = PublisherError::database("test");

    // All expected types are available - regression test passes
}
