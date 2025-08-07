use starknet_handler::{
    account::StarknetAccount,
    provider::{LatestRelayBlock, StarknetProvider},
};

use crate::{
    core::{AccumulatorBuilder, BatchProcessor, MMRStateManager, ProofGenerator},
    error::{PublisherError, PublisherResult},
};

/// Service responsible for MMR operations
pub struct MmrService {
    rpc_url: String,
    chain_id: u64,
    verifier_address: String,
    store_address: String,
}

impl MmrService {
    /// Create a new MMR service with the specified configuration
    pub const fn new(
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

    /// Create a configured `AccumulatorBuilder`
    async fn create_accumulator_builder(
        &self,
        account_private_key: &str,
        account_address: &str,
        batch_size: u64,
    ) -> PublisherResult<AccumulatorBuilder<'_>> {
        let starknet_provider = StarknetProvider::new(&self.rpc_url).map_err(|e| {
            PublisherError::starknet_provider(format!("Failed to create provider: {e}"))
        })?;

        let starknet_account = StarknetAccount::new(
            starknet_provider.provider(),
            account_private_key,
            account_address,
        )
        .map_err(|e| PublisherError::starknet_provider(format!("Failed to create account: {e}")))?;

        // Create components for AccumulatorBuilder
        let proof_generator = ProofGenerator::new(methods::MMR_BUILD_ELF, methods::MMR_BUILD_ID)
            .map_err(|e| {
                PublisherError::proof_generation(format!("Failed to create proof generator: {e}"))
            })?;

        let mmr_state_manager =
            MMRStateManager::new(starknet_account, &self.store_address, &self.rpc_url);

        let batch_processor = BatchProcessor::new(batch_size, proof_generator, mmr_state_manager)
            .map_err(|e| {
            PublisherError::proof_generation(format!("Failed to create batch processor: {e}"))
        })?;

        AccumulatorBuilder::new(
            &self.rpc_url,
            self.chain_id,
            &self.verifier_address,
            batch_processor,
            0, // current_batch
            0, // total_batches
        )
        .await
        .map_err(|e| {
            PublisherError::mmr_operation(format!("Failed to create AccumulatorBuilder: {e}"))
        })
    }

    /// Update MMR with new headers and generate proofs
    pub async fn update_with_proof_generation(
        &self,
        account_private_key: &str,
        account_address: &str,
        batch_size: u64,
        start_block: u64,
        latest_relayed_block_and_hash: LatestRelayBlock,
    ) -> PublisherResult<()> {
        let mut builder = self
            .create_accumulator_builder(account_private_key, account_address, batch_size)
            .await?;

        tracing::info!("Starting MMR update with proof generation");

        builder
            .update_mmr_with_new_headers(start_block, latest_relayed_block_and_hash, false)
            .await
            .map_err(|e| {
                PublisherError::mmr_operation(format!(
                    "Failed to update MMR with proof generation: {e}"
                ))
            })?;

        tracing::debug!("Successfully updated MMR with proof generation");

        Ok(())
    }

    /// Update MMR with new headers without generating proofs
    pub async fn update_without_proof_generation(
        &self,
        account_private_key: &str,
        account_address: &str,
        batch_size: u64,
        start_block: u64,
        latest_relayed_block_and_hash: LatestRelayBlock,
    ) -> PublisherResult<()> {
        let mut builder = self
            .create_accumulator_builder(account_private_key, account_address, batch_size)
            .await?;

        tracing::info!("Starting MMR update without proof generation");

        builder
            .update_mmr_with_new_headers(start_block, latest_relayed_block_and_hash, true)
            .await
            .map_err(|e| {
                PublisherError::mmr_operation(format!(
                    "Failed to update MMR without proof generation: {e}"
                ))
            })?;

        tracing::debug!("Successfully updated MMR without proof generation");

        Ok(())
    }
}
