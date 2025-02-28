use crate::utils::BatchResult;
use ethereum::get_finalized_block_hash;
use eyre::{eyre, Result};
use starknet_crypto::Felt;
use starknet_handler::provider::StarknetProvider;
use tracing::{debug, error, info, warn};

use super::BatchProcessor;

pub struct AccumulatorBuilder<'a> {
    starknet_rpc_url: &'a String,
    chain_id: u64,
    verifier_address: &'a String,
    batch_processor: BatchProcessor<'a>,
    current_batch: u64,
    total_batches: u64,
}

impl<'a> AccumulatorBuilder<'a> {
    pub async fn new(
        starknet_rpc_url: &'a String,
        chain_id: u64,
        verifier_address: &'a String,
        batch_processor: BatchProcessor<'a>,
        current_batch: u64,
        total_batches: u64,
    ) -> Result<Self> {
        if verifier_address.trim().is_empty() {
            return Err(eyre!(
                "Verifier address cannot be empty: {}",
                verifier_address
            ));
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
    pub async fn build_with_num_batches(&mut self, num_batches: u64) -> Result<()> {
        if num_batches == 0 {
            return Err(eyre!(
                "Number of batches must be greater than 0: {}",
                num_batches
            ));
        }

        let (finalized_block_number, _) = get_finalized_block_hash().await.map_err(|e| {
            error!(error = %e, "Failed to get finalized block hash");
            eyre!("Failed to get finalized block: {}", e)
        })?;

        self.total_batches = num_batches;
        self.current_batch = 0;
        let mut current_end = finalized_block_number;

        for batch_num in 0..num_batches {
            if current_end == 0 {
                warn!("Reached block 0 before completing all batches");
                break;
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
                .batch_processor
                .process_batch(self.chain_id, start_block, current_end)
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
                })?;

            if let Some(batch_result) = result {
                self.handle_batch_result(&batch_result, true).await?;
                self.current_batch += 1;
                info!(
                    progress = format!("{}/{}", self.current_batch, self.total_batches),
                    "Batch processed successfully"
                );
            }

            current_end = start_block.saturating_sub(1);
        }

        info!("MMR build completed successfully");
        Ok(())
    }

    pub async fn build_from_finalized(&mut self) -> Result<()> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        debug!(
            "Building MMR from finalized block {} with batch size {}",
            finalized_block_number,
            self.batch_processor.batch_size()
        );

        let mut current_end = finalized_block_number;

        while current_end > 0 {
            let start_block = self.batch_processor.calculate_start_block(current_end)?;
            let batch_result = self
                .batch_processor
                .process_batch(self.chain_id, start_block, current_end)
                .await?;

            if let Some(result) = batch_result {
                self.handle_batch_result(&result, true).await?;
            }

            current_end = start_block.saturating_sub(1);
        }

        Ok(())
    }

    pub async fn update_mmr_with_new_headers(
        &mut self,
        start_block: u64,
        end_block: u64,
        is_build: bool,
    ) -> Result<()> {
        if end_block < start_block {
            return Err(eyre!(
                "End block cannot be less than start block: {} < {}",
                end_block,
                start_block
            ));
        }

        info!(
            total_blocks = end_block - start_block + 1,
            latest_mmr_block = start_block - 1,
            latest_relayed_block = end_block,
            "Starting MMR update with new headers"
        );

        // Process the batch and ensure we get a result
        let batch_result = self
            .batch_processor
            .process_batch(self.chain_id, start_block, end_block)
            .await?
            .ok_or_else(|| {
                eyre!(
                    "No batch result returned for blocks {}-{}",
                    start_block,
                    end_block
                )
            })?;

        // Always handle the batch result with the is_build flag
        self.handle_batch_result(&batch_result, is_build).await?;

        self.current_batch += 1;
        info!(
            "MMR update completed successfully for blocks {}-{}",
            start_block, end_block
        );

        Ok(())
    }

    async fn handle_batch_result(&self, batch_result: &BatchResult, is_build: bool) -> Result<()> {
        // Always attempt verification if proof is available
        if let Some(proof) = batch_result.proof() {
            self.verify_proof(proof.calldata(), batch_result.ipfs_hash(), is_build)
                .await?;
        } else {
            return Err(eyre!(
                "No proof available for verification for batch: {:?}",
                batch_result
            ));
        }
        Ok(())
    }

    async fn verify_proof(
        &self,
        calldata: Vec<Felt>,
        ipfs_hash: String,
        is_build: bool,
    ) -> Result<()> {
        let starknet_account = self.batch_processor.mmr_state_manager().account();

        info!("Verifying MMR proof (is_build: {})", is_build);
        starknet_account
            .verify_mmr_proof(&self.verifier_address, calldata, ipfs_hash, is_build)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to verify MMR proof");
                e
            })?;

        Ok(())
    }

    pub async fn build_from_block(&mut self, start_block: u64, is_build: bool) -> Result<()> {
        info!("Building MMR from block {}", start_block);
        self.process_blocks_from(start_block, is_build).await
    }

    pub async fn build_from_block_with_batches(
        &mut self,
        start_block: u64,
        num_batches: u64,
        is_build: bool,
    ) -> Result<()> {
        info!(
            "Building MMR from block {} with {} batches",
            start_block, num_batches
        );
        self.process_blocks_from_with_limit(start_block, num_batches, is_build)
            .await
    }

    async fn process_blocks_from(&mut self, start_block: u64, is_build: bool) -> Result<()> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        if start_block > finalized_block_number {
            return Err(eyre!(
                "Start block cannot be greater than finalized block: {} > {}",
                start_block,
                finalized_block_number
            ));
        }

        debug!(
            "Processing blocks from {} with batch size {}",
            start_block,
            self.batch_processor.batch_size()
        );

        let mut current_end = start_block;

        while current_end > 0 {
            let start = self.batch_processor.calculate_start_block(current_end)?;
            let batch_result = self
                .batch_processor
                .process_batch(self.chain_id, start, current_end)
                .await?;

            if let Some(result) = batch_result {
                self.handle_batch_result(&result, is_build).await?;
            }

            current_end = start.saturating_sub(1);
        }

        Ok(())
    }

    async fn process_blocks_from_with_limit(
        &mut self,
        start_block: u64,
        num_batches: u64,
        is_build: bool,
    ) -> Result<()> {
        if num_batches == 0 {
            return Err(eyre!(
                "Number of batches must be greater than 0: {}",
                num_batches
            ));
        }

        let (finalized_block_number, _) = get_finalized_block_hash()
            .await
            .map_err(|e| eyre!("Failed to get finalized block: {}", e))?;

        if start_block > finalized_block_number {
            return Err(eyre!(
                "Start block cannot be greater than finalized block: {} > {}",
                start_block,
                finalized_block_number
            ));
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
                .process_batch(self.chain_id, start, current_end)
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

    pub async fn build_from_latest(&mut self, is_build: bool) -> Result<()> {
        let provider = StarknetProvider::new(&self.starknet_rpc_url)
            .map_err(|e| eyre!("Failed to create Starknet provider: {}", e))?;

        let latest_mmr_block = provider
            .get_min_mmr_block(&self.batch_processor.mmr_state_manager().store_address())
            .await
            .map_err(|e| eyre!("Failed to get latest MMR block: {}", e))?;

        info!(
            "Building MMR from minimum MMR block {} - 1",
            latest_mmr_block
        );
        self.process_blocks_from(latest_mmr_block - 1, is_build)
            .await
    }

    pub async fn build_from_latest_with_batches(
        &mut self,
        num_batches: u64,
        is_build: bool,
    ) -> Result<()> {
        let provider = StarknetProvider::new(&self.starknet_rpc_url)
            .map_err(|e| eyre!("Failed to create Starknet provider: {}", e))?;

        let min_mmr_block = provider
            .get_min_mmr_block(&self.batch_processor.mmr_state_manager().store_address())
            .await
            .map_err(|e| eyre!("Failed to get minimum MMR block: {}", e))?;

        if min_mmr_block == 0 {
            error!("No MMR has been built yet (min_mmr_block = 0)");
            std::process::exit(1);
        }

        info!(
            "Building MMR from latest MMR block {} with {} batches",
            min_mmr_block - 1,
            num_batches
        );
        self.process_blocks_from_with_limit(min_mmr_block - 1, num_batches, is_build)
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{MMRStateManager, ProofGenerator};

    use super::*;
    use mockall::mock;
    use mockall::predicate::*;
    use starknet::providers::jsonrpc::HttpTransport;
    use starknet::providers::JsonRpcClient;
    use starknet::providers::Url;
    use starknet_handler::account::StarknetAccount;
    use std::env;
    use std::sync::Arc;

    // Setup test environment variables
    fn setup_test_env() {
        env::set_var("IPFS_ADD_URL", "http://localhost:5001/api/v0/add");
        env::set_var("IPFS_FETCH_BASE_URL", "http://localhost/ipfs/");
        env::set_var("IPFS_TOKEN", "test_token_placeholder");
    }

    mock! {
        #[derive(Clone)]
        pub StarknetAccount {
            fn verify_mmr_proof(&self, verifier_address: &str, calldata: Vec<Felt>, ipfs_hash: String, is_build: bool) -> Result<()>;
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

        let result = builder.update_mmr_with_new_headers(100, 50, false).await;
        assert!(
            matches!(result, Err(e) if e.to_string().contains("End block cannot be less than start block"))
        );
    }
}
