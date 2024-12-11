use crate::core::{BatchProcessor, ProofGenerator};
use crate::errors::AccumulatorError;
use crate::utils::BatchResult;
use ethereum::get_finalized_block_hash;
use methods::{MMR_APPEND_ELF, MMR_APPEND_ID};
use starknet_crypto::Felt;
use starknet_handler::{account::StarknetAccount, provider::StarknetProvider};
use tracing::{debug, error, info, warn};

pub struct AccumulatorBuilder<'a> {
    rpc_url: &'a String,
    chain_id: u64,
    verifier_address: &'a String,
    account_private_key: &'a String,
    account_address: &'a String,
    batch_processor: BatchProcessor,
    current_batch: u64,
    total_batches: u64,
}

impl<'a> AccumulatorBuilder<'a> {
    pub async fn new(
        rpc_url: &'a String,
        chain_id: u64,
        verifier_address: &'a String,
        account_private_key: &'a String,
        account_address: &'a String,
        batch_size: u64,
        skip_proof_verification: bool,
    ) -> Result<Self, AccumulatorError> {
        let proof_generator =
            ProofGenerator::new(MMR_APPEND_ELF, MMR_APPEND_ID, skip_proof_verification)?;

        if rpc_url.trim().is_empty() {
            return Err(AccumulatorError::InvalidInput("RPC URL cannot be empty"));
        }
        if verifier_address.trim().is_empty() {
            return Err(AccumulatorError::InvalidInput(
                "Verifier address cannot be empty",
            ));
        }
        if account_private_key.trim().is_empty() {
            return Err(AccumulatorError::InvalidInput(
                "Account private key cannot be empty",
            ));
        }
        if account_address.trim().is_empty() {
            return Err(AccumulatorError::InvalidInput(
                "Account address cannot be empty",
            ));
        }
        if batch_size == 0 {
            return Err(AccumulatorError::InvalidInput(
                "Batch size must be greater than 0",
            ));
        }

        Ok(Self {
            rpc_url,
            chain_id,
            verifier_address,
            account_private_key,
            account_address,
            batch_processor: BatchProcessor::new(
                batch_size,
                proof_generator,
                skip_proof_verification,
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
                self.verify_proof(proof.calldata()).await?;
            } else {
                debug!("Skipping proof verification - no proof available");
            }
        } else {
            debug!("Skipping proof verification - verification disabled");
        }
        Ok(())
    }

    async fn verify_proof(&self, calldata: Vec<Felt>) -> Result<(), AccumulatorError> {
        debug!("Initializing Starknet provider");
        let starknet_provider = StarknetProvider::new(&self.rpc_url).map_err(|e| {
            error!(error = %e, "Failed to initialize Starknet provider");
            e
        })?;

        debug!("Creating Starknet account");
        let starknet_account = StarknetAccount::new(
            starknet_provider.provider(),
            &self.account_private_key,
            &self.account_address,
        )
        .map_err(|e| {
            error!(error = %e, "Failed to create Starknet account");
            e
        })?;

        debug!("Verifying MMR proof");
        starknet_account
            .verify_mmr_proof(&self.verifier_address, calldata)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to verify MMR proof");
                e
            })?;

        debug!("MMR proof verified successfully");
        Ok(())
    }
}
