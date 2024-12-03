use crate::core::{BatchProcessor, ProofGenerator};
use crate::errors::AccumulatorError;
use crate::utils::BatchResult;
use ethereum::get_finalized_block_hash;
use methods::{MMR_APPEND_ELF, MMR_APPEND_ID};
use starknet_crypto::Felt;
use starknet_handler::{account::StarknetAccount, provider::StarknetProvider};
use tracing::{debug, info};

pub struct AccumulatorBuilder<'a> {
    rpc_url: &'a String,
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
        verifier_address: &'a String,
        account_private_key: &'a String,
        account_address: &'a String,
        batch_size: u64,
        skip_proof_verification: bool,
    ) -> Result<Self, AccumulatorError> {
        let proof_generator =
            ProofGenerator::new(MMR_APPEND_ELF, MMR_APPEND_ID, skip_proof_verification);

        Ok(Self {
            rpc_url,
            verifier_address,
            account_private_key,
            account_address,
            batch_processor: BatchProcessor::new(
                batch_size,
                proof_generator,
                skip_proof_verification,
            ),
            current_batch: 0,
            total_batches: 0,
        })
    }

    /// Build the MMR using a specified number of batches
    pub async fn build_with_num_batches(
        &mut self,
        num_batches: u64,
    ) -> Result<(), AccumulatorError> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        info!("Building MMR...");

        self.total_batches = num_batches;
        self.current_batch = 0;

        let mut current_end = finalized_block_number;

        for _ in 0..num_batches {
            if current_end == 0 {
                break;
            }

            let start_block = self.batch_processor.calculate_start_block(current_end);
            let result = self
                .batch_processor
                .process_batch(start_block, current_end)
                .await?;

            if let Some(batch_result) = result {
                self.handle_batch_result(&batch_result).await?;
                self.current_batch += 1;
            }

            current_end = start_block.saturating_sub(1);
        }

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
            let start_block = self.batch_processor.calculate_start_block(current_end);
            let batch_result = self
                .batch_processor
                .process_batch(start_block, current_end)
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
        let mut current_end = end_block;
        let mut batch_results = Vec::new();

        debug!(
            "Updating MMR with blocks from {} to {}",
            start_block, end_block
        );

        while current_end >= start_block {
            let batch_range = self
                .batch_processor
                .calculate_batch_range(current_end, start_block);

            if let Some(result) = self
                .batch_processor
                .process_batch(batch_range.start, batch_range.end)
                .await?
            {
                self.handle_batch_result(&result).await?;
                batch_results.push((result.proof().calldata(), result.new_mmr_state()));
            }

            current_end = batch_range.start.saturating_sub(1);
        }

        if batch_results.is_empty() {
            Err(AccumulatorError::InvalidStateTransition)
        } else {
            Ok(())
        }
    }

    async fn handle_batch_result(
        &self,
        batch_result: &BatchResult,
    ) -> Result<(), AccumulatorError> {
        if !self.batch_processor.skip_proof_verification() {
            self.verify_proof(batch_result.proof().calldata()).await?;
        }
        Ok(())
    }

    async fn verify_proof(&self, calldata: Vec<Felt>) -> Result<(), AccumulatorError> {
        let starknet_provider = StarknetProvider::new(&self.rpc_url)?;
        let starknet_account = StarknetAccount::new(
            starknet_provider.provider(),
            &self.account_private_key,
            &self.account_address,
        )?;

        starknet_account
            .verify_mmr_proof(&self.verifier_address, calldata)
            .await?;

        Ok(())
    }
}
