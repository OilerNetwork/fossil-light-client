use std::path::Path;

use guest_mmr::{core::GuestMMR, helper::find_peaks};
use guest_types::GuestMMRProof;
use mmr;
use starknet_handler::{
    account::StarknetAccount,
    provider::{LatestRelayBlock, StarknetProvider},
};

use crate::{
    core::{AccumulatorBuilder, BatchProcessor, MMRStateManager, ProofGenerator},
    db::DbConnection,
    error::{PublisherError, PublisherResult},
};

/// Service responsible for proof generation operations
pub struct ProofService {
    rpc_url: String,
    chain_id: u64,
    verifier_address: String,
    store_address: String,
}

impl ProofService {
    pub fn new(
        rpc_url: String,
        chain_id: u64,
        verifier_address: String,
        store_address: String,
    ) -> Self {
        Self {
            rpc_url,
            chain_id,
            verifier_address,
            store_address,
        }
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
        let starknet_provider = StarknetProvider::new(&self.rpc_url).map_err(|e| {
            PublisherError::starknet_provider(format!("Failed to create provider: {}", e))
        })?;

        let starknet_account = StarknetAccount::new(
            starknet_provider.provider(),
            account_private_key,
            account_address,
        )
        .map_err(|e| {
            PublisherError::starknet_provider(format!("Failed to create account: {}", e))
        })?;

        // Create components for AccumulatorBuilder
        let proof_generator = ProofGenerator::new(methods::MMR_BUILD_ELF, methods::MMR_BUILD_ID)
            .map_err(|e| {
                PublisherError::proof_generation(format!("Failed to create proof generator: {}", e))
            })?;

        let mmr_state_manager =
            MMRStateManager::new(starknet_account, &self.store_address, &self.rpc_url);

        let batch_processor = BatchProcessor::new(batch_size, proof_generator, mmr_state_manager)
            .map_err(|e| {
            PublisherError::proof_generation(format!("Failed to create batch processor: {}", e))
        })?;

        let mut builder = AccumulatorBuilder::new(
            &self.rpc_url,
            self.chain_id,
            &self.verifier_address,
            batch_processor,
            0, // current_batch
            0, // total_batches
        )
        .await
        .map_err(|e| {
            PublisherError::proof_generation(format!("Failed to create AccumulatorBuilder: {}", e))
        })?;

        tracing::info!("Starting MMR update and proof generation");

        builder
            .update_mmr_with_new_headers(start_block, latest_relayed_block_and_hash, false)
            .await
            .map_err(|e| {
                PublisherError::proof_generation(format!(
                    "Failed to update MMR with new headers: {}",
                    e
                ))
            })?;

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
        let starknet_provider = StarknetProvider::new(&self.rpc_url).map_err(|e| {
            PublisherError::starknet_provider(format!("Failed to create provider: {}", e))
        })?;

        let starknet_account = StarknetAccount::new(
            starknet_provider.provider(),
            account_private_key,
            account_address,
        )
        .map_err(|e| {
            PublisherError::starknet_provider(format!("Failed to create account: {}", e))
        })?;

        // Create components for AccumulatorBuilder
        let proof_generator = ProofGenerator::new(methods::MMR_BUILD_ELF, methods::MMR_BUILD_ID)
            .map_err(|e| {
                PublisherError::proof_generation(format!("Failed to create proof generator: {}", e))
            })?;

        let mmr_state_manager =
            MMRStateManager::new(starknet_account, &self.store_address, &self.rpc_url);

        let batch_processor = BatchProcessor::new(batch_size, proof_generator, mmr_state_manager)
            .map_err(|e| {
            PublisherError::proof_generation(format!("Failed to create batch processor: {}", e))
        })?;

        let mut builder = AccumulatorBuilder::new(
            &self.rpc_url,
            self.chain_id,
            &self.verifier_address,
            batch_processor,
            0, // current_batch
            0, // total_batches
        )
        .await
        .map_err(|e| {
            PublisherError::proof_generation(format!("Failed to create AccumulatorBuilder: {}", e))
        })?;

        // Always generate and verify proofs (false = don't skip proof verification)
        builder
            .update_mmr_with_new_headers(start_block, latest_relayed_block_and_hash, false)
            .await
            .map_err(|e| PublisherError::mmr_operation(format!("Failed to update MMR: {}", e)))?;

        Ok(())
    }

    /// Get a single block hash proof
    pub async fn get_single_block_hash_proof(
        &self,
        block_hash: &str,
        batch_size: u64,
    ) -> PublisherResult<(u64, GuestMMR, mmr::Proof)> {
        tracing::info!("Looking up proof for block hash: {}", block_hash);

        // Connect to Starknet
        let provider = StarknetProvider::new(&self.rpc_url).map_err(|e| {
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
            .get_mmr_state(&self.store_address, batch_index)
            .await
            .map_err(|e| {
                PublisherError::starknet_provider(format!("Failed to get MMR state: {}", e))
            })?;

        // Get the IPFS hash from the MMR state
        let ipfs_hash = mmr_state.ipfs_hash();
        let ipfs_hash_str = String::try_from(ipfs_hash)
            .map_err(|_| PublisherError::serialization("Invalid IPFS hash format"))?;

        // Set up temporary file path for the downloaded DB
        let batch_file_name = common::get_or_create_db_path(&format!("batch_{}.db", batch_index))
            .map_err(|e| {
            PublisherError::configuration(format!("Failed to create DB path: {}", e))
        })?;

        // Initialize IPFS manager and download the DB file
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

        // Initialize the MMR from the downloaded DB
        let (store_manager, mmr, pool) = mmr_utils::initialize_mmr(&batch_file_name)
            .await
            .map_err(|e| {
                PublisherError::mmr_operation(format!("Failed to initialize MMR: {}", e))
            })?;

        // Verify that the MMR root in the downloaded DB matches the onchain state
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

        // Verify leaves count
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

        let peaks = mmr
            .retrieve_peaks_hashes(find_peaks(mmr_elements_count), None)
            .await
            .map_err(|e| {
                PublisherError::mmr_operation(format!("Failed to retrieve peaks: {}", e))
            })?;

        tracing::info!("MMR state verification successful");

        // Get the element index for the block hash
        let element_index = store_manager
            .get_element_index_for_value(&pool, block_hash)
            .await
            .map_err(|e| PublisherError::validation(format!("Failed to get element index: {}", e)))?
            .ok_or_else(|| PublisherError::validation("Block hash not found in MMR"))?;

        let guest_mmr = GuestMMR::new(peaks, mmr_elements_count, mmr_leaves_count);

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
}
