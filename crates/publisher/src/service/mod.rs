/// MMR service for managing MMR operations
pub mod mmr_service;
/// Proof service for generating and managing proofs
pub mod proof_service;

pub use mmr_service::MmrService;
pub use proof_service::ProofService;

#[cfg(test)]
mod tests {
    use mockall::{mock, predicate::*};

    use super::*;
    use crate::{
        config::{AccountConfig, PublisherConfig},
        error::PublisherResult,
    };

    mock! {
        pub TestStarknetProvider {
            pub fn new(rpc_url: &str) -> PublisherResult<Self>;
            pub async fn get_mmr_state(
                &self,
                address: &str,
                batch_index: u64,
            ) -> PublisherResult<starknet_handler::MmrSnapshot>;
        }
    }

    mock! {
        pub TestDbConnection {
            pub async fn new() -> PublisherResult<std::sync::Arc<Self>>;
            pub async fn get_block_header_by_hash(
                &self,
                block_hash: &str,
            ) -> PublisherResult<eth_rlp_types::BlockHeader>;
        }
    }

    #[test]
    fn test_config_creation() {
        let config = PublisherConfig {
            rpc_url: "http://localhost:8545".to_string(),
            chain_id: 1,
            verifier_address: "0x123".to_string(),
            store_address: "0x456".to_string(),
            batch_size: 100,
        };

        assert_eq!(config.rpc_url, "http://localhost:8545");
        assert_eq!(config.chain_id, 1);
        assert_eq!(config.batch_size, 100);
    }

    #[test]
    fn test_account_config_validation() {
        let config = AccountConfig::new("private_key".to_string(), "address".to_string());

        assert!(config.validate().is_ok());

        let invalid_config = AccountConfig::new("".to_string(), "address".to_string());

        assert!(invalid_config.validate().is_err());
    }

    #[tokio::test]
    async fn test_proof_service_creation() {
        let config = PublisherConfig {
            rpc_url: "http://localhost:8545".to_string(),
            chain_id: 1,
            verifier_address: "0x123".to_string(),
            store_address: "0x456".to_string(),
            batch_size: 100,
        };

        let service = ProofService::with_config(config.clone());
        // Test that service was created successfully
        // This is a basic smoke test
        assert_eq!(service.config.rpc_url, config.rpc_url);
        assert_eq!(service.config.chain_id, config.chain_id);
    }
}
