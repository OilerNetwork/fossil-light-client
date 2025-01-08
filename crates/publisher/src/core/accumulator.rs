use crate::core::{BatchProcessor, ProofGenerator};
use crate::errors::AccumulatorError;
use crate::utils::BatchResult;
use ethereum::get_finalized_block_hash;
use methods::{MMR_APPEND_ELF, MMR_APPEND_ID};
use starknet_crypto::Felt;
use starknet_handler::account::StarknetAccount;
use starknet_handler::provider::StarknetProvider;
use tracing::{debug, error, info, warn};

use super::MMRStateManager;

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
        store_address: &'a String,
        starknet_account: StarknetAccount,
        batch_size: u64,
        skip_proof_verification: bool,
    ) -> Result<Self, AccumulatorError> {
        let proof_generator = ProofGenerator::new(MMR_APPEND_ELF, MMR_APPEND_ID)?;
        let mmr_state_manager = MMRStateManager::new(starknet_account, store_address);

        if verifier_address.trim().is_empty() {
            return Err(AccumulatorError::InvalidInput(
                "Verifier address cannot be empty",
            ));
        }
        if batch_size == 0 {
            return Err(AccumulatorError::InvalidInput(
                "Batch size must be greater than 0",
            ));
        }

        Ok(Self {
            starknet_rpc_url,
            chain_id,
            verifier_address,
            batch_processor: BatchProcessor::new(
                batch_size,
                proof_generator,
                skip_proof_verification,
                mmr_state_manager,
            )?,
            current_batch: 0,
            total_batches: 0,
        })
    }

    /// Build the MMR using a specified number of batches
    pub async fn build_with_num_batches(
        &mut self,
        num_batches: u64,
    ) -> Result<(), AccumulatorError> {
        if num_batches == 0 {
            return Err(AccumulatorError::InvalidInput(
                "Number of batches must be greater than 0",
            ));
        }

        let (finalized_block_number, _) = get_finalized_block_hash().await.map_err(|e| {
            error!(error = %e, "Failed to get finalized block hash");
            AccumulatorError::BlockchainError(format!("Failed to get finalized block: {}", e))
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
                self.handle_batch_result(&batch_result).await?;
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

    pub async fn build_from_finalized(&mut self) -> Result<(), AccumulatorError> {
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
                self.handle_batch_result(&result).await?;
            }

            current_end = start_block.saturating_sub(1);
        }

        Ok(())
    }

    pub async fn update_mmr_with_new_headers(
        &mut self,
        start_block: u64,
        end_block: u64,
    ) -> Result<(), AccumulatorError> {
        if end_block < start_block {
            return Err(AccumulatorError::InvalidInput(
                "End block cannot be less than start block",
            ));
        }

        let mut current_end = end_block;
        let mut batch_results = Vec::new();

        info!(
            total_blocks = end_block - start_block,
            "Starting MMR update with new headers"
        );

        while current_end >= start_block {
            let batch_range = self
                .batch_processor
                .calculate_batch_range(current_end, start_block)?;

            debug!(
                batch_start = batch_range.start,
                batch_end = batch_range.end,
                "Processing batch range"
            );

            if let Some(result) = self
                .batch_processor
                .process_batch(self.chain_id, batch_range.start, batch_range.end)
                .await
                .map_err(|e| {
                    error!(
                        error = %e,
                        batch_start = batch_range.start,
                        batch_end = batch_range.end,
                        "Failed to process batch"
                    );
                    e
                })?
            {
                self.handle_batch_result(&result).await?;
                let calldata = result
                    .proof()
                    .map(|proof| proof.calldata())
                    .unwrap_or_else(Vec::new);
                batch_results.push((calldata, result.new_mmr_state()));
                debug!(
                    batch_start = batch_range.start,
                    batch_end = batch_range.end,
                    "Batch processed successfully"
                );
            }

            current_end = batch_range.start.saturating_sub(1);
        }

        if batch_results.is_empty() {
            error!(start_block, end_block, "No batch results generated");
            Err(AccumulatorError::InvalidStateTransition)
        } else {
            debug!(
                total_batches = batch_results.len(),
                "MMR update completed successfully"
            );
            Ok(())
        }
    }

    async fn handle_batch_result(
        &self,
        batch_result: &BatchResult,
    ) -> Result<(), AccumulatorError> {
        // Skip verification if explicitly disabled or if no proof is available
        if !self.batch_processor.skip_proof_verification() {
            if let Some(proof) = batch_result.proof() {
                self.verify_proof(proof.calldata(), batch_result.ipfs_hash()).await?;
            } else {
                debug!("Skipping proof verification - no proof available");
            }
        } else {
            debug!("Skipping proof verification - verification disabled");
        }
        Ok(())
    }

    async fn verify_proof(&self, calldata: Vec<Felt>, ipfs_hash: Option<String>) -> Result<(), AccumulatorError> {
        let starknet_account = self.batch_processor.mmr_state_manager().account();

        info!("Verifying MMR proof");
        starknet_account
            .verify_mmr_proof(&self.verifier_address, calldata, ipfs_hash)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to verify MMR proof");
                e
            })?;

        info!("MMR proof verified successfully");
        Ok(())
    }

    pub async fn build_from_block(&mut self, start_block: u64) -> Result<(), AccumulatorError> {
        info!("Building MMR from block {}", start_block);
        self.process_blocks_from(start_block).await
    }

    pub async fn build_from_block_with_batches(
        &mut self,
        start_block: u64,
        num_batches: u64,
    ) -> Result<(), AccumulatorError> {
        info!(
            "Building MMR from block {} with {} batches",
            start_block, num_batches
        );
        self.process_blocks_from_with_limit(start_block, num_batches)
            .await
    }

    async fn process_blocks_from(&mut self, start_block: u64) -> Result<(), AccumulatorError> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        if start_block > finalized_block_number {
            return Err(AccumulatorError::InvalidInput(
                "Start block cannot be greater than finalized block",
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
                self.handle_batch_result(&result).await?;
            }

            current_end = start.saturating_sub(1);
        }

        Ok(())
    }

    async fn process_blocks_from_with_limit(
        &mut self,
        start_block: u64,
        num_batches: u64,
    ) -> Result<(), AccumulatorError> {
        if num_batches == 0 {
            return Err(AccumulatorError::InvalidInput(
                "Number of batches must be greater than 0",
            ));
        }

        let (finalized_block_number, _) = get_finalized_block_hash().await.map_err(|e| {
            error!(error = %e, "Failed to get finalized block hash");
            AccumulatorError::BlockchainError(format!("Failed to get finalized block: {}", e))
        })?;

        if start_block > finalized_block_number {
            return Err(AccumulatorError::InvalidInput(
                "Start block cannot be greater than finalized block",
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
                self.handle_batch_result(&batch_result).await?;
                self.current_batch += 1;
                info!(
                    progress = format!("{}/{}", self.current_batch, self.total_batches),
                    "Batch processed successfully"
                );
            }

            current_end = start.saturating_sub(1);
        }

        info!("MMR build completed successfully");
        Ok(())
    }

    pub async fn build_from_latest(&mut self) -> Result<(), AccumulatorError> {
        let provider = StarknetProvider::new(&self.starknet_rpc_url).map_err(|e| {
            error!(error = %e, "Failed to create Starknet provider");
            AccumulatorError::BlockchainError(format!("Failed to create Starknet provider: {}", e))
        })?;

        let latest_mmr_block = provider
            .get_min_mmr_block(&self.batch_processor.mmr_state_manager().store_address())
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to get latest MMR block");
                AccumulatorError::BlockchainError(format!("Failed to get latest MMR block: {}", e))
            })?;

        info!(
            "Building MMR from minimum MMR block {} - 1",
            latest_mmr_block
        );
        self.process_blocks_from(latest_mmr_block - 1).await
    }

    pub async fn build_from_latest_with_batches(
        &mut self,
        num_batches: u64,
    ) -> Result<(), AccumulatorError> {
        let provider = StarknetProvider::new(&self.starknet_rpc_url).map_err(|e| {
            error!(error = %e, "Failed to create Starknet provider");
            AccumulatorError::BlockchainError(format!("Failed to create Starknet provider: {}", e))
        })?;

        let min_mmr_block = provider
            .get_min_mmr_block(&self.batch_processor.mmr_state_manager().store_address())
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to get minimum MMR block");
                AccumulatorError::BlockchainError(format!("Failed to get minimum MMR block: {}", e))
            })?;

        info!(
            "Building MMR from latest MMR block {} with {} batches",
            min_mmr_block - 1,
            num_batches
        );
        self.process_blocks_from_with_limit(min_mmr_block - 1, num_batches)
            .await
    }
}
