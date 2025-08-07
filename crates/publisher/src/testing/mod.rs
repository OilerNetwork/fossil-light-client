/// Helper functions and utilities for testing
pub mod helpers;
/// Test utilities and helpers for the publisher crate
///
/// This module provides common test utilities, mock implementations,
/// and helper functions to facilitate testing across the crate.
pub mod mocks;

pub use helpers::*;
pub use mocks::*;
use starknet_handler::provider::LatestRelayBlock;

use crate::config::{AccountConfig, PublisherConfig};

/// Creates a default test configuration for testing
pub fn create_test_config() -> PublisherConfig {
    PublisherConfig {
        rpc_url: "http://localhost:8545".to_string(),
        chain_id: 1,
        verifier_address: "0x1234567890123456789012345678901234567890".to_string(),
        store_address: "0x0987654321098765432109876543210987654321".to_string(),
        batch_size: 100,
    }
}

/// Creates a default test account configuration
pub fn create_test_account_config() -> AccountConfig {
    AccountConfig::new(
        "0xprivatekey123456789".to_string(),
        "0xaddress123456789".to_string(),
    )
}

/// Creates a mock LatestRelayBlock for testing
pub fn create_mock_latest_relay_block() -> LatestRelayBlock {
    LatestRelayBlock {
        block_number: 100,
        block_hash: "0xmockblockhash123456789".to_string(),
    }
}

/// Creates a test block hash
pub fn create_test_block_hash() -> String {
    "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string()
}

/// Creates a test MMR state for testing
pub fn create_test_mmr_snapshot() -> starknet_handler::MmrSnapshot {
    // This would need to be implemented based on the actual MmrSnapshot structure
    // For now, this is a placeholder that would need real implementation
    todo!("Implement create_test_mmr_snapshot when MmrSnapshot structure is available")
}

/// Test constants commonly used across tests
pub mod constants {
    /// Default RPC URL for testing
    pub const TEST_RPC_URL: &str = "http://localhost:8545";
    /// Default chain ID for testing
    pub const TEST_CHAIN_ID: u64 = 1;
    /// Default verifier contract address for testing
    pub const TEST_VERIFIER_ADDRESS: &str = "0x1234567890123456789012345678901234567890";
    /// Default store contract address for testing
    pub const TEST_STORE_ADDRESS: &str = "0x0987654321098765432109876543210987654321";
    /// Default batch size for testing
    pub const TEST_BATCH_SIZE: u64 = 100;
    /// Default start block number for testing
    pub const TEST_START_BLOCK: u64 = 1;
    /// Default private key for testing (not a real key)
    pub const TEST_PRIVATE_KEY: &str = "0xprivatekey123456789";
    /// Default account address for testing
    pub const TEST_ADDRESS: &str = "0xaddress123456789";
    /// Default block hash for testing
    pub const TEST_BLOCK_HASH: &str =
        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
}
