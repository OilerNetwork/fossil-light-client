use ethereum::get_finalized_block_hash;
use starknet_crypto::Felt;
use starknet_handler::provider::{LatestRelayBlock, StarknetProvider};
use tracing::{debug, error, info, warn};

use super::BatchProcessor;
use crate::{
    error::{PublisherError, PublisherResult},
    utils::BatchResult,
};

/// Builder for MMR accumulator operations, managing batch processing and proof generation
pub struct AccumulatorBuilder<'a> {
    starknet_rpc_url: &'a str,
    chain_id: u64,
    verifier_address: &'a str,
    batch_processor: BatchProcessor<'a>,
    current_batch: u64,
    total_batches: u64,
}

impl<'a> AccumulatorBuilder<'a> {
    /// Create a new `AccumulatorBuilder` with the specified configuration
    pub async fn new(
        starknet_rpc_url: &'a str,
        chain_id: u64,
        verifier_address: &'a str,
        batch_processor: BatchProcessor<'a>,
        current_batch: u64,
        total_batches: u64,
    ) -> PublisherResult<Self> {
        if verifier_address.trim().is_empty() {
            return Err(PublisherError::validation(format!(
                "Verifier address cannot be empty: {verifier_address}"
            )));
        }

        Ok(Self {
            starknet_rpc_url,
            chain_id,
            verifier_address,
            batch_processor,
            current_batch,
            total_batches,
        })
    }

    /// Build the MMR using a specified number of batches
    pub async fn build_with_num_batches(&mut self, num_batches: u64) -> PublisherResult<()> {
        self.validate_batch_count(num_batches)?;
        let finalized_block_number = self.get_finalized_block_number().await?;
        self.initialize_batch_processing(num_batches);

        let mut current_end = finalized_block_number;
        for batch_num in 0..num_batches {
            if let Some(new_end) = self
                .process_single_batch(batch_num, current_end, finalized_block_number, num_batches)
                .await?
            {
                current_end = new_end;
            } else {
                break;
            }
        }

        info!("MMR build completed successfully");
        Ok(())
    }

    fn validate_batch_count(&self, num_batches: u64) -> PublisherResult<()> {
        if num_batches == 0 {
            return Err(PublisherError::validation(format!(
                "Number of batches must be greater than 0: {num_batches}"
            )));
        }
        Ok(())
    }

    async fn get_finalized_block_number(&self) -> PublisherResult<u64> {
        let (finalized_block_number, _) = get_finalized_block_hash().await.map_err(|e| {
            error!(error = %e, "Failed to get finalized block hash");
            PublisherError::validation(format!("Failed to get finalized block: {e}"))
        })?;
        Ok(finalized_block_number)
    }

    const fn initialize_batch_processing(&mut self, num_batches: u64) {
        self.total_batches = num_batches;
        self.current_batch = 0;
    }

    async fn process_single_batch(
        &mut self,
        batch_num: u64,
        current_end: u64,
        finalized_block_number: u64,
        num_batches: u64,
    ) -> PublisherResult<Option<u64>> {
        if current_end == 0 {
            warn!("Reached block 0 before completing all batches");
            return Ok(None);
        }

        let start_block = self.batch_processor.calculate_start_block(current_end)?;
        debug!(batch_num, start_block, current_end, "Processing batch");

        info!(
            finalized_block_number,
            num_batches,
            start_block,
            current_end,
            "Starting MMR build with specified number of batches"
        );

        let result = self
            .execute_batch_processing(batch_num, start_block, current_end)
            .await?;

        if let Some(batch_result) = result {
            self.handle_successful_batch(batch_result).await?;
        }

        Ok(Some(start_block.saturating_sub(1)))
    }

    async fn execute_batch_processing(
        &self,
        batch_num: u64,
        start_block: u64,
        current_end: u64,
    ) -> PublisherResult<Option<crate::utils::BatchResult>> {
        self.batch_processor
            .process_batch(self.chain_id, start_block, current_end, None)
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    batch_num,
                    start_block,
                    current_end,
                    "Failed to process batch"
                );
                e
            })
    }

    async fn handle_successful_batch(
        &mut self,
        batch_result: crate::utils::BatchResult,
    ) -> PublisherResult<()> {
        self.handle_batch_result(&batch_result, true).await?;
        self.current_batch += 1;
        info!(
            progress = format!("{}/{}", self.current_batch, self.total_batches),
            "Batch processed successfully"
        );
        Ok(())
    }

    /// Build MMR from finalized blockchain blocks
    pub async fn build_from_finalized(&mut self) -> PublisherResult<()> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        debug!(
            "Building MMR from finalized block {finalized_block_number} with batch size {}",
            self.batch_processor.batch_size()
        );

        let mut current_end = finalized_block_number;

        while current_end > 0 {
            let start_block = self.batch_processor.calculate_start_block(current_end)?;
            let batch_result = self
                .batch_processor
                .process_batch(self.chain_id, start_block, current_end, None)
                .await?;

            if let Some(result) = batch_result {
                self.handle_batch_result(&result, true).await?;
            }

            current_end = start_block.saturating_sub(1);
        }

        Ok(())
    }

    /// Update the MMR with new block headers from the specified range
    #[allow(clippy::cognitive_complexity)]
    pub async fn update_mmr_with_new_headers(
        &mut self,
        start_block: u64,
        latest_relayed_block_and_hash: LatestRelayBlock,
        is_build: bool,
    ) -> PublisherResult<()> {
        if latest_relayed_block_and_hash.block_number < start_block {
            return Err(PublisherError::validation(format!(
                "End block cannot be less than start block: {} < {start_block}",
                latest_relayed_block_and_hash.block_number
            )));
        }

        info!(
            total_blocks = latest_relayed_block_and_hash.block_number - start_block + 1,
            start_block,
            latest_relayed_block_and_hash.block_number,
            "Starting MMR update with new headers"
        );

        // Calculate batch indices for start and end blocks
        let start_batch_index = start_block / self.batch_processor.batch_size();
        let end_batch_index =
            latest_relayed_block_and_hash.block_number / self.batch_processor.batch_size();
        let total_batches = end_batch_index - start_batch_index + 1;

        info!(
            start_batch_index,
            end_batch_index, total_batches, "Updating Light Client with {total_batches} batches"
        );

        // Process each batch in sequence
        for batch_index in start_batch_index..=end_batch_index {
            let (batch_start, batch_end) =
                self.batch_processor.calculate_batch_bounds(batch_index)?;

            // Calculate effective start and end for this batch
            let effective_start = std::cmp::max(start_block, batch_start);
            let effective_end =
                std::cmp::min(latest_relayed_block_and_hash.block_number, batch_end);

            debug!(
                batch_index,
                effective_start, effective_end, "Processing batch within range"
            );

            // Process the batch and ensure we get a result
            let batch_result = self
                .batch_processor
                .process_batch(
                    self.chain_id,
                    effective_start,
                    effective_end,
                    // Only pass the block hash for the last batch
                    if batch_index == end_batch_index {
                        Some(latest_relayed_block_and_hash.block_hash.clone())
                    } else {
                        None
                    },
                )
                .await?
                .ok_or_else(|| {
                    PublisherError::validation(format!(
                        "No batch result returned for blocks {effective_start}-{effective_end}"
                    ))
                })?;

            // Always handle the batch result with the is_build flag
            self.handle_batch_result(&batch_result, is_build).await?;

            self.current_batch += 1;
            info!(
                progress = format!("{}/{}", batch_index - start_batch_index + 1, total_batches),
                "Batch processed successfully for blocks {effective_start}-{effective_end}"
            );
        }

        info!(
            "MMR update completed successfully for all blocks {start_block}-{}",
            latest_relayed_block_and_hash.block_number
        );

        Ok(())
    }

    async fn handle_batch_result(
        &self,
        batch_result: &BatchResult,
        is_build: bool,
    ) -> PublisherResult<()> {
        // Always attempt verification if proof is available
        if let Some(proof) = batch_result.proof() {
            self.verify_proof(proof.calldata(), batch_result.ipfs_hash(), is_build)
                .await?;
        } else {
            return Err(PublisherError::validation(format!(
                "No proof available for verification for batch: {batch_result:?}"
            )));
        }
        Ok(())
    }

    async fn verify_proof(
        &self,
        calldata: Vec<Felt>,
        ipfs_hash: String,
        is_build: bool,
    ) -> PublisherResult<()> {
        let starknet_account = self.batch_processor.mmr_state_manager().account();

        info!("Verifying MMR proof (is_build: {is_build})");
        starknet_account
            .verify_mmr_proof(self.verifier_address, calldata, ipfs_hash, is_build)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to verify MMR proof");
                e
            })?;

        Ok(())
    }

    /// Build MMR starting from a specific block number
    pub async fn build_from_block(
        &mut self,
        start_block: u64,
        is_build: bool,
    ) -> PublisherResult<()> {
        info!("Building MMR from block {start_block}");
        self.process_blocks_from(start_block, is_build).await
    }

    /// Build MMR from a specific block with a limited number of batches
    pub async fn build_from_block_with_batches(
        &mut self,
        start_block: u64,
        num_batches: u64,
        is_build: bool,
    ) -> PublisherResult<()> {
        info!("Building MMR from block {start_block} with {num_batches} batches");
        self.process_blocks_from_with_limit(start_block, num_batches, is_build)
            .await
    }

    async fn process_blocks_from(&self, start_block: u64, is_build: bool) -> PublisherResult<()> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        if start_block > finalized_block_number {
            return Err(PublisherError::validation(format!(
                "Start block cannot be greater than finalized block: {start_block} > {finalized_block_number}"
            )));
        }

        debug!(
            "Processing blocks from {start_block} with batch size {}",
            self.batch_processor.batch_size()
        );

        let mut current_end = start_block;

        while current_end > 0 {
            let start = self.batch_processor.calculate_start_block(current_end)?;
            let batch_result = self
                .batch_processor
                .process_batch(self.chain_id, start, current_end, None)
                .await?;

            if let Some(result) = batch_result {
                self.handle_batch_result(&result, is_build).await?;
            }

            current_end = start.saturating_sub(1);
        }

        Ok(())
    }

    #[allow(clippy::cognitive_complexity)]
    async fn process_blocks_from_with_limit(
        &mut self,
        start_block: u64,
        num_batches: u64,
        is_build: bool,
    ) -> PublisherResult<()> {
        if num_batches == 0 {
            return Err(PublisherError::validation(format!(
                "Number of batches must be greater than 0: {num_batches}"
            )));
        }

        let (finalized_block_number, _) = get_finalized_block_hash().await.map_err(|e| {
            PublisherError::validation(format!("Failed to get finalized block: {e}"))
        })?;

        if start_block > finalized_block_number {
            return Err(PublisherError::validation(format!(
                "Start block cannot be greater than finalized block: {start_block} > {finalized_block_number}"
            )));
        }

        self.total_batches = num_batches;
        self.current_batch = 0;
        let mut current_end = start_block;

        for batch_num in 0..num_batches {
            if current_end == 0 {
                warn!("Reached block 0 before completing all batches");
                break;
            }

            let start = self.batch_processor.calculate_start_block(current_end)?;
            debug!(batch_num, start, current_end, "Processing batch");

            let result = self
                .batch_processor
                .process_batch(self.chain_id, start, current_end, None)
                .await
                .map_err(|e| {
                    error!(
                        error = %e,
                        batch_num,
                        start,
                        current_end,
                        "Failed to process batch"
                    );
                    e
                })?;

            if let Some(batch_result) = result {
                self.handle_batch_result(&batch_result, is_build).await?;
                self.current_batch += 1;
                info!(
                    progress = format!("{}/{}", self.current_batch, self.total_batches),
                    "Batch processed successfully"
                );
            }

            current_end = start.saturating_sub(1);
        }

        info!("MMR accumulation completed successfully");
        Ok(())
    }

    /// Build MMR from the latest available block
    pub async fn build_from_latest(&mut self, is_build: bool) -> PublisherResult<()> {
        let provider = StarknetProvider::new(self.starknet_rpc_url).map_err(|e| {
            PublisherError::validation(format!("Failed to create Starknet provider: {e}"))
        })?;

        let latest_mmr_block = provider
            .get_min_mmr_block(self.batch_processor.mmr_state_manager().store_address())
            .await
            .map_err(|e| {
                PublisherError::validation(format!("Failed to get latest MMR block: {e}"))
            })?;

        info!("Building MMR from minimum MMR block {latest_mmr_block} - 1");
        self.process_blocks_from(latest_mmr_block - 1, is_build)
            .await
    }

    /// Build MMR from the latest block with a limited number of batches
    pub async fn build_from_latest_with_batches(
        &mut self,
        num_batches: u64,
        is_build: bool,
    ) -> PublisherResult<()> {
        let provider = StarknetProvider::new(self.starknet_rpc_url).map_err(|e| {
            PublisherError::validation(format!("Failed to create Starknet provider: {e}"))
        })?;

        let min_mmr_block = provider
            .get_min_mmr_block(self.batch_processor.mmr_state_manager().store_address())
            .await
            .map_err(|e| {
                PublisherError::validation(format!("Failed to get minimum MMR block: {e}"))
            })?;

        if min_mmr_block == 0 {
            error!("No MMR has been built yet (min_mmr_block = 0)");
            std::process::exit(1);
        }

        info!(
            "Building MMR from latest MMR block {} with {num_batches} batches",
            min_mmr_block - 1
        );
        self.process_blocks_from_with_limit(min_mmr_block - 1, num_batches, is_build)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::{env, sync::Arc};

    use mockall::{mock, predicate::*};
    use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient, Url};
    use starknet_handler::account::StarknetAccount;

    use super::*;
    use crate::core::{MMRStateManager, ProofGenerator};

    // Setup test environment variables
    fn setup_test_env() {
        env::set_var("IPFS_ADD_URL", "http://localhost:5001/api/v0/add");
        env::set_var("IPFS_FETCH_BASE_URL", "http://localhost/ipfs/");
        env::set_var("IPFS_TOKEN", "test_token_placeholder");
    }

    mock! {
        #[derive(Clone)]
        pub StarknetAccount {
            fn verify_mmr_proof(&self, verifier_address: &str, calldata: Vec<Felt>, ipfs_hash: String, is_build: bool) -> PublisherResult<()>;
        }
    }

    // Add conversion impl
    impl From<MockStarknetAccount> for StarknetAccount {
        #[allow(clippy::unused_self)]
        fn from(_mock: MockStarknetAccount) -> Self {
            // Create a new StarknetAccount for testing
            let transport = HttpTransport::new(Url::parse("http://localhost:8545").unwrap());
            let provider = Arc::new(JsonRpcClient::new(transport));

            StarknetAccount::new(provider, "0x123", "0x456").unwrap()
        }
    }

    #[tokio::test]
    async fn test_accumulator_builder_new() {
        setup_test_env();

        let account = MockStarknetAccount::new();
        // Create longer-lived String values
        let rpc_url = "http://localhost:8545".to_string();
        let verifier_addr = "0x123".to_string();
        let store_addr = "0x456".to_string();

        let batch_processor = BatchProcessor::new(
            100,
            ProofGenerator::mock_for_tests(),
            MMRStateManager::new(account.into(), &store_addr, &rpc_url),
        )
        .unwrap();

        let result =
            AccumulatorBuilder::new(&rpc_url, 1, &verifier_addr, batch_processor, 0, 0).await;

        assert!(result.is_ok());
        let builder = result.unwrap();
        assert_eq!(builder.chain_id, 1);
        assert_eq!(builder.current_batch, 0);
        assert_eq!(builder.total_batches, 0);
    }

    #[tokio::test]
    async fn test_accumulator_builder_new_invalid_inputs() {
        setup_test_env();

        let account = MockStarknetAccount::new();
        let rpc_url = "http://localhost:8545".to_string();
        let store_addr = "0x456".to_string();

        let batch_processor = BatchProcessor::new(
            100,
            ProofGenerator::mock_for_tests(),
            MMRStateManager::new(account.into(), &store_addr, &rpc_url),
        )
        .unwrap();

        // Test empty verifier address
        let binding = "".to_string();
        let result = AccumulatorBuilder::new(&rpc_url, 1, &binding, batch_processor, 0, 0).await;
        assert!(
            matches!(result, Err(e) if e.to_string().contains("Verifier address cannot be empty"))
        );
    }

    #[tokio::test]
    async fn test_build_with_num_batches_invalid_input() {
        setup_test_env();

        let account = MockStarknetAccount::new();
        let rpc_url = "http://localhost:8545".to_string();
        let verifier_addr = "0x123".to_string();
        let store_addr = "0x456".to_string();

        let batch_processor = BatchProcessor::new(
            100,
            ProofGenerator::mock_for_tests(),
            MMRStateManager::new(account.into(), &store_addr, &rpc_url),
        )
        .unwrap();

        let mut builder =
            AccumulatorBuilder::new(&rpc_url, 1, &verifier_addr, batch_processor, 0, 0)
                .await
                .unwrap();

        let result = builder.build_with_num_batches(0).await;
        assert!(
            matches!(result, Err(e) if e.to_string().contains("Number of batches must be greater than 0"))
        );
    }

    #[tokio::test]
    async fn test_update_mmr_with_new_headers_invalid_input() {
        setup_test_env();

        let account = MockStarknetAccount::new();
        let rpc_url = "http://localhost:8545".to_string();
        let verifier_addr = "0x123".to_string();
        let store_addr = "0x456".to_string();

        let batch_processor = BatchProcessor::new(
            100,
            ProofGenerator::mock_for_tests(),
            MMRStateManager::new(account.into(), &store_addr, &rpc_url),
        )
        .unwrap();

        let mut builder =
            AccumulatorBuilder::new(&rpc_url, 1, &verifier_addr, batch_processor, 0, 0)
                .await
                .unwrap();

        let result = builder
            .update_mmr_with_new_headers(
                100,
                LatestRelayBlock {
                    block_number: 50,
                    block_hash: "0x123".to_string(),
                },
                false,
            )
            .await;
        assert!(
            matches!(result, Err(e) if e.to_string().contains("End block cannot be less than start block"))
        );
    }
}
