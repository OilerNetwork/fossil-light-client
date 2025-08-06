use std::path::Path;

use guest_mmr::{core::GuestMMR, helper::find_peaks};
use guest_types::GuestMMRProof;
use mmr;
use starknet_handler::{
    account::StarknetAccount,
    provider::{LatestRelayBlock, StarknetProvider},
};

use crate::{
    config::{AccountConfig, PublisherConfig},
    core::{AccumulatorBuilder, BatchProcessor, MMRStateManager, ProofGenerator},
    db::DbConnection,
    error::{PublisherError, PublisherResult},
};

/// Service responsible for proof generation operations
pub struct ProofService {
    pub(crate) config: PublisherConfig,
}

impl ProofService {
    /// Create a new proof service with individual parameters (legacy method)
    pub fn new(
        rpc_url: String,
        chain_id: u64,
        verifier_address: String,
        store_address: String,
    ) -> Self {
        let config = PublisherConfig {
            rpc_url,
            chain_id,
            verifier_address,
            store_address,
            batch_size: 100, // Default batch size
        };
        Self { config }
    }

    /// Create a new proof service using a configuration object
    pub fn with_config(config: PublisherConfig) -> Self {
        Self { config }
    }

    /// Generate MMR update proof
    pub async fn prove_mmr_update(
        &self,
        account_private_key: &str,
        account_address: &str,
        batch_size: u64,
        start_block: u64,
        latest_relayed_block_and_hash: LatestRelayBlock,
    ) -> PublisherResult<()> {
        let account_config =
            AccountConfig::new(account_private_key.to_string(), account_address.to_string());
        account_config.validate()?;

        let mut builder = self
            .create_accumulator_builder(&account_config, batch_size)
            .await?;

        tracing::info!("Starting MMR update and proof generation");

        self.execute_mmr_update(&mut builder, start_block, latest_relayed_block_and_hash)
            .await?;

        tracing::debug!("Successfully generated proof for block range");
        Ok(())
    }

    /// Update MMR without generating proofs
    pub async fn update_mmr(
        &self,
        account_private_key: &str,
        account_address: &str,
        batch_size: u64,
        start_block: u64,
        latest_relayed_block_and_hash: LatestRelayBlock,
    ) -> PublisherResult<()> {
        let account_config =
            AccountConfig::new(account_private_key.to_string(), account_address.to_string());
        account_config.validate()?;

        let mut builder = self
            .create_accumulator_builder(&account_config, batch_size)
            .await?;

        self.execute_mmr_update(&mut builder, start_block, latest_relayed_block_and_hash)
            .await?;

        Ok(())
    }

    /// Get a single block hash proof
    pub async fn get_single_block_hash_proof(
        &self,
        block_hash: &str,
        batch_size: u64,
    ) -> PublisherResult<(u64, GuestMMR, mmr::Proof)> {
        tracing::info!("Looking up proof for block hash: {}", block_hash);

        let (batch_index, mmr_state) = self
            .get_batch_info_for_block_hash(block_hash, batch_size)
            .await?;

        let batch_file_name = self
            .fetch_and_validate_mmr_db(batch_index, &mmr_state)
            .await?;

        // Initialize the MMR from the downloaded DB
        let (store_manager, mmr, pool) = mmr_utils::initialize_mmr(&batch_file_name)
            .await
            .map_err(|e| {
                PublisherError::mmr_operation(format!("Failed to initialize MMR: {}", e))
            })?;

        let (mmr_elements_count, mmr_leaves_count) =
            self.validate_mmr_state(&mmr, &mmr_state).await?;

        let peaks = mmr
            .retrieve_peaks_hashes(find_peaks(mmr_elements_count as usize), None)
            .await
            .map_err(|e| {
                PublisherError::mmr_operation(format!("Failed to retrieve peaks: {}", e))
            })?;

        // Get the element index for the block hash
        let element_index = store_manager
            .get_element_index_for_value(&pool, block_hash)
            .await
            .map_err(|e| PublisherError::validation(format!("Failed to get element index: {}", e)))?
            .ok_or_else(|| PublisherError::validation("Block hash not found in MMR"))?;

        let guest_mmr = GuestMMR::new(
            peaks,
            mmr_elements_count as usize,
            mmr_leaves_count as usize,
        );

        // Get the Merkle proof for the block hash
        let proof = mmr
            .get_proof(element_index, None)
            .await
            .map_err(|e| PublisherError::mmr_operation(format!("Failed to get proof: {}", e)))?;

        tracing::info!(
            "Successfully generated Merkle proof for block hash: {}",
            block_hash
        );

        Ok((batch_index, guest_mmr, proof))
    }

    /// Get block hash inclusion proof as a serializable response
    pub async fn get_block_hash_inclusion_proof(
        &self,
        block_hash: &str,
        batch_size: u64,
    ) -> PublisherResult<(u64, GuestMMR, GuestMMRProof)> {
        let (batch_index, guest_mmr, proof) = self
            .get_single_block_hash_proof(block_hash, batch_size)
            .await?;

        // Convert mmr::Proof to GuestMMRProof
        let guest_proof = GuestMMRProof {
            element_index: proof.element_index,
            element_hash: proof.element_hash,
            siblings_hashes: proof.siblings_hashes,
            peaks_hashes: proof.peaks_hashes,
            elements_count: proof.elements_count,
        };

        Ok((batch_index, guest_mmr, guest_proof))
    }

    /// Helper method to create Starknet provider and account
    async fn create_starknet_components(
        &self,
        account_config: &AccountConfig,
    ) -> PublisherResult<(StarknetProvider, StarknetAccount)> {
        let starknet_provider = StarknetProvider::new(&self.config.rpc_url).map_err(|e| {
            PublisherError::starknet_provider(format!("Failed to create provider: {}", e))
        })?;

        let starknet_account = StarknetAccount::new(
            starknet_provider.provider(),
            &account_config.private_key,
            &account_config.address,
        )
        .map_err(|e| {
            PublisherError::starknet_provider(format!("Failed to create account: {}", e))
        })?;

        Ok((starknet_provider, starknet_account))
    }

    /// Helper method to create AccumulatorBuilder with all required components
    async fn create_accumulator_builder(
        &self,
        account_config: &AccountConfig,
        batch_size: u64,
    ) -> PublisherResult<AccumulatorBuilder<'_>> {
        let (_starknet_provider, starknet_account) =
            self.create_starknet_components(account_config).await?;

        let proof_generator = ProofGenerator::new(methods::MMR_BUILD_ELF, methods::MMR_BUILD_ID)
            .map_err(|e| {
                PublisherError::proof_generation(format!("Failed to create proof generator: {}", e))
            })?;

        let mmr_state_manager = MMRStateManager::new(
            starknet_account,
            &self.config.store_address,
            &self.config.rpc_url,
        );

        let batch_processor = BatchProcessor::new(batch_size, proof_generator, mmr_state_manager)
            .map_err(|e| {
            PublisherError::proof_generation(format!("Failed to create batch processor: {}", e))
        })?;

        AccumulatorBuilder::new(
            &self.config.rpc_url,
            self.config.chain_id,
            &self.config.verifier_address,
            batch_processor,
            0, // current_batch
            0, // total_batches
        )
        .await
        .map_err(|e| {
            PublisherError::proof_generation(format!("Failed to create AccumulatorBuilder: {}", e))
        })
    }

    /// Helper method to execute MMR update
    async fn execute_mmr_update(
        &self,
        builder: &mut AccumulatorBuilder<'_>,
        start_block: u64,
        latest_relayed_block_and_hash: LatestRelayBlock,
    ) -> PublisherResult<()> {
        builder
            .update_mmr_with_new_headers(start_block, latest_relayed_block_and_hash, false)
            .await
            .map_err(|e| {
                PublisherError::proof_generation(format!(
                    "Failed to update MMR with new headers: {}",
                    e
                ))
            })
    }

    /// Helper method to fetch and validate MMR database from IPFS
    async fn fetch_and_validate_mmr_db(
        &self,
        batch_index: u64,
        mmr_state: &starknet_handler::MmrSnapshot,
    ) -> PublisherResult<String> {
        let ipfs_hash = mmr_state.ipfs_hash();
        let ipfs_hash_str = String::try_from(ipfs_hash)
            .map_err(|_| PublisherError::serialization("Invalid IPFS hash format"))?;

        let batch_file_name = common::get_or_create_db_path(&format!("batch_{}.db", batch_index))
            .map_err(|e| {
            PublisherError::configuration(format!("Failed to create DB path: {}", e))
        })?;

        let ipfs_manager = ipfs_utils::IpfsManager::with_endpoint()
            .map_err(|e| PublisherError::ipfs(format!("Failed to create IPFS manager: {}", e)))?;

        match ipfs_manager
            .fetch_db(&ipfs_hash_str, Path::new(&batch_file_name))
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "Successfully downloaded DB from IPFS for batch {}",
                    batch_index
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    batch_index = batch_index,
                    "Failed to fetch DB from IPFS, falling back to local file"
                );

                if !std::path::Path::new(&batch_file_name).exists() {
                    return Err(PublisherError::ipfs(
                        "Failed to fetch DB from IPFS and no local file exists",
                    ));
                }
            }
        }

        Ok(batch_file_name)
    }

    /// Helper method to validate MMR against onchain state
    async fn validate_mmr_state(
        &self,
        mmr: &mmr::MMR,
        mmr_state: &starknet_handler::MmrSnapshot,
    ) -> PublisherResult<(u64, u64)> {
        let mmr_elements_count = mmr.elements_count.get().await.map_err(|e| {
            PublisherError::mmr_operation(format!("Failed to get elements count: {}", e))
        })?;

        let bag = mmr
            .bag_the_peaks(Some(mmr_elements_count))
            .await
            .map_err(|e| PublisherError::mmr_operation(format!("Failed to bag peaks: {}", e)))?;

        let mmr_root_hex = mmr
            .calculate_root_hash(&bag, mmr_elements_count)
            .map_err(|e| {
                PublisherError::mmr_operation(format!("Failed to calculate root hash: {}", e))
            })?
            .to_string();

        let mmr_root = starknet_handler::u256_from_hex(&mmr_root_hex).map_err(|e| {
            PublisherError::serialization(format!("Failed to parse MMR root: {}", e))
        })?;

        if mmr_root != mmr_state.root_hash() {
            return Err(PublisherError::validation(format!(
                "MMR root mismatch: expected {} but got {}",
                mmr_state.root_hash(),
                mmr_root
            )));
        }

        let mmr_leaves_count = mmr.leaves_count.get().await.map_err(|e| {
            PublisherError::mmr_operation(format!("Failed to get leaves count: {}", e))
        })?;

        if mmr_leaves_count as u64 != mmr_state.leaves_count() {
            return Err(PublisherError::validation(format!(
                "Leaves count mismatch: expected {} but got {}",
                mmr_state.leaves_count(),
                mmr_leaves_count
            )));
        }

        tracing::info!("MMR state validation successful");
        Ok((mmr_elements_count as u64, mmr_leaves_count as u64))
    }

    /// Helper method to get batch information for a block hash
    async fn get_batch_info_for_block_hash(
        &self,
        block_hash: &str,
        batch_size: u64,
    ) -> PublisherResult<(u64, starknet_handler::MmrSnapshot)> {
        // Connect to Starknet
        let provider = StarknetProvider::new(&self.config.rpc_url).map_err(|e| {
            PublisherError::starknet_provider(format!("Failed to create provider: {}", e))
        })?;

        // Get the block header to determine which batch it belongs to
        let db_connection = DbConnection::new().await.map_err(|e| {
            PublisherError::validation(format!("Failed to connect to database: {}", e))
        })?;

        let header = db_connection
            .get_block_header_by_hash(block_hash)
            .await
            .map_err(|e| {
                PublisherError::validation(format!("Failed to get block header: {}", e))
            })?;

        // Calculate the batch index based on the block number and batch size
        let batch_index = header.number as u64 / batch_size;
        tracing::info!("Block belongs to batch index: {}", batch_index);

        // Fetch the MMR state from onchain
        let mmr_state = provider
            .get_mmr_state(&self.config.store_address, batch_index)
            .await
            .map_err(|e| {
                PublisherError::starknet_provider(format!("Failed to get MMR state: {}", e))
            })?;

        Ok((batch_index, mmr_state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{constants::*, create_test_account_config, create_test_config};

    #[test]
    fn test_proof_service_creation() {
        let config = create_test_config();
        let service = ProofService::with_config(config.clone());

        assert_eq!(service.config.rpc_url, config.rpc_url);
        assert_eq!(service.config.chain_id, config.chain_id);
        assert_eq!(service.config.verifier_address, config.verifier_address);
        assert_eq!(service.config.store_address, config.store_address);
        assert_eq!(service.config.batch_size, config.batch_size);
    }

    #[test]
    fn test_proof_service_legacy_creation() {
        let service = ProofService::new(
            TEST_RPC_URL.to_string(),
            TEST_CHAIN_ID,
            TEST_VERIFIER_ADDRESS.to_string(),
            TEST_STORE_ADDRESS.to_string(),
        );

        assert_eq!(service.config.rpc_url, TEST_RPC_URL);
        assert_eq!(service.config.chain_id, TEST_CHAIN_ID);
        assert_eq!(service.config.verifier_address, TEST_VERIFIER_ADDRESS);
        assert_eq!(service.config.store_address, TEST_STORE_ADDRESS);
        assert_eq!(service.config.batch_size, 100); // Default batch size
    }

    #[test]
    fn test_config_validation() {
        let config = create_test_config();
        assert!(config.validate().is_ok());

        // Test invalid config
        let invalid_config = PublisherConfig {
            rpc_url: "".to_string(),
            chain_id: 1,
            verifier_address: "".to_string(),
            store_address: "".to_string(),
            batch_size: 0,
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_account_config_validation() {
        let account_config = create_test_account_config();
        assert!(account_config.validate().is_ok());

        // Test invalid account config
        let invalid_account_config = AccountConfig::new("".to_string(), "".to_string());
        assert!(invalid_account_config.validate().is_err());
    }

    // Integration test placeholders - these would require actual implementation
    // when the dependencies are available for proper mocking

    #[tokio::test]
    #[ignore = "Requires external dependencies"]
    async fn test_prove_mmr_update_integration() {
        // This test would be implemented with proper mocking
        // of StarknetProvider, StarknetAccount, etc.
        let config = create_test_config();
        let service = ProofService::with_config(config);

        let account_config = create_test_account_config();
        let latest_relay_block = crate::testing::create_mock_latest_relay_block();

        // This would test the actual prove_mmr_update functionality
        // with mocked dependencies
        let result = service
            .prove_mmr_update(
                &account_config.private_key,
                &account_config.address,
                TEST_BATCH_SIZE,
                TEST_START_BLOCK,
                latest_relay_block,
            )
            .await;

        // This test would be completed when mocking infrastructure is in place
        // For now, we just verify the service was created correctly
        assert!(result.is_err()); // Expected since we don't have real dependencies
    }

    #[test]
    fn test_error_conversion() {
        use crate::config::ConfigError;

        // Test ConfigError to PublisherError conversion
        let config_error = ConfigError::MissingField("test_field");
        let publisher_error: PublisherError = config_error.into();

        assert!(matches!(publisher_error, PublisherError::Configuration(_)));
        assert!(publisher_error.to_string().contains("test_field"));
    }

    #[test]
    fn test_publisher_error_helpers() {
        let db_error = PublisherError::database("Database connection failed");
        assert!(matches!(db_error, PublisherError::Database(_)));
        assert!(db_error.to_string().contains("Database connection failed"));

        let ipfs_error = PublisherError::ipfs("IPFS fetch failed");
        assert!(matches!(ipfs_error, PublisherError::Ipfs(_)));
        assert!(ipfs_error.to_string().contains("IPFS fetch failed"));

        let proof_error = PublisherError::proof_generation("Proof generation failed");
        assert!(matches!(proof_error, PublisherError::ProofGeneration(_)));
        assert!(proof_error.to_string().contains("Proof generation failed"));
    }

    // Performance test
    #[tokio::test]
    async fn test_service_creation_performance() {
        use std::time::Duration;

        use crate::testing::helpers::performance::measure_async;

        let config = create_test_config();

        let (_, duration) = measure_async(|| async {
            let _service = ProofService::with_config(config);
            // Service creation should be fast
        })
        .await;

        // Service creation should be nearly instantaneous
        assert!(duration < Duration::from_millis(10));
    }
}
