// host/src/accumulator.rs
use crate::db_access::{get_block_headers_by_block_range, DbConnection};
use crate::proof_generator::{ProofGenerator, ProofGeneratorError};
use crate::types::BatchResult;
use common::{get_or_create_db_path, UtilsError};
use ethereum::get_finalized_block_hash;
use guest_types::{CombinedInput, GuestOutput, MMRInput};
use mmr::{InStoreTableError, MMRError, PeaksOptions, MMR};
use mmr_utils::{initialize_mmr, MMRUtilsError, StoreManager};
use starknet_crypto::Felt;
use starknet_handler::account::StarknetAccount;
use starknet_handler::provider::StarknetProvider;
use starknet_handler::{u256_from_hex, MmrState};
use store::{SqlitePool, StoreError, SubKey};
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum AccumulatorError {
    #[error("Invalid state transition: elements count decreased")]
    InvalidStateTransition,
    #[error("Failed to verify stored peaks after update")]
    PeaksVerificationError,
    #[error("MMR root is not a valid Starknet field element: {0}")]
    InvalidU256Hex(String),
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Utils error: {0}")]
    Utils(#[from] UtilsError),
    #[error("MMR error: {0}")]
    MMRError(#[from] MMRError),
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    #[error("ProofGenerator error: {0}")]
    ProofGenerator(#[from] ProofGeneratorError),
    #[error("MMRUtils error: {0}")]
    MMRUtils(#[from] MMRUtilsError),
    #[error("InStoreTable error: {0}")]
    InStoreTable(#[from] InStoreTableError),
    #[error("StarknetHandler error: {0}")]
    StarknetHandler(#[from] starknet_handler::StarknetHandlerError),
}

pub struct AccumulatorBuilder<'a> {
    rpc_url: &'a String,
    verifier_address: &'a String,
    account_private_key: &'a String,
    account_address: &'a String,
    batch_size: u64,
    current_batch: u64,
    total_batches: u64,
    proof_generator: ProofGenerator<CombinedInput>,
    skip_proof_verification: bool,
}

impl<'a> AccumulatorBuilder<'a> {
    pub async fn new(
        rpc_url: &'a String,
        verifier_address: &'a String,
        account_private_key: &'a String,
        account_address: &'a String,
        proof_generator: ProofGenerator<CombinedInput>,
        batch_size: u64,
        skip_proof_verification: bool,
    ) -> Result<Self, AccumulatorError> {
        Ok(Self {
            rpc_url,
            verifier_address,
            account_private_key,
            account_address,
            batch_size,
            current_batch: 0,
            total_batches: 0,
            proof_generator,
            skip_proof_verification,
        })
    }

    async fn process_batch(
        &mut self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Option<BatchResult>, AccumulatorError> {
        // Calculate batch index based on the lowest block number of the batch
        let batch_index = start_block / self.batch_size;

        // Determine batch start and end blocks
        let batch_start = batch_index * self.batch_size;
        let batch_end = batch_start + self.batch_size - 1;

        // Adjust end_block if it exceeds batch_end
        let adjusted_end_block = std::cmp::min(end_block, batch_end);

        info!(
            "Processing batch {} (blocks {} to {})",
            batch_index, start_block, adjusted_end_block
        );

        // Determine batch file name
        let batch_file_name = get_or_create_db_path(&format!("batch_{}.db", batch_index))?;
        println!("Batch-file-name: {}", batch_file_name);

        // Initialize MMR
        let (store_manager, mmr, pool) = initialize_mmr(&batch_file_name).await?;

        // Get MMR state
        let current_leaves_count = mmr.leaves_count.get().await?;
        let batch_is_complete = current_leaves_count as u64 >= self.batch_size;

        if batch_is_complete {
            // Batch is complete, no need to process
            debug!("Batch {} is already complete", batch_index);
            return Ok(None);
        }

        // Fetch headers for the block range
        let db_connection = DbConnection::new().await?;
        let headers =
            get_block_headers_by_block_range(&db_connection.pool, start_block, adjusted_end_block)
                .await?;

        // Prepare guest input
        let current_peaks = mmr.get_peaks(PeaksOptions::default()).await?;
        let current_elements_count = mmr.elements_count.get().await?;
        let current_leaves_count = mmr.leaves_count.get().await?;

        let mmr_input = MMRInput::new(
            current_peaks,
            current_elements_count,
            current_leaves_count,
            headers.iter().map(|h| h.block_hash.clone()).collect(),
        );

        let combined_input =
            CombinedInput::new(headers.clone(), mmr_input, self.skip_proof_verification);

        // Generate proof
        let proof = self
            .proof_generator
            .generate_groth16_proof(combined_input)
            .await?;

        // Decode and update state
        let guest_output: GuestOutput = self.proof_generator.decode_journal(&proof)?;

        let new_mmr_state = update_mmr_state(
            store_manager,
            &mmr,
            &pool,
            adjusted_end_block,
            &guest_output,
        )
        .await?;

        // Check if batch is now complete
        let new_leaves_count = mmr.leaves_count.get().await?;
        let batch_is_complete = new_leaves_count as u64 >= self.batch_size;

        if batch_is_complete {
            debug!("Batch {} is now complete", batch_index);
            // Optionally pad the MMR if necessary
            // Finalize batch
        }

        Ok(Some(BatchResult::new(
            start_block,
            adjusted_end_block,
            new_mmr_state,
            proof,
        )))
    }

    /// Build the MMR using a specified number of batches
    pub async fn build_with_num_batches(
        &mut self,
        num_batches: u64,
    ) -> Result<(), AccumulatorError> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        info!("Building MMR...",);

        self.total_batches = num_batches;
        self.current_batch = num_batches;

        let mut current_end = finalized_block_number;

        for _ in 0..num_batches {
            if current_end == 0 {
                break;
            }

            let start_block = current_end.saturating_sub(current_end % self.batch_size);

            let result = self.process_batch(start_block, current_end).await?;

            if let Some(batch_result) = result {
                if !self.skip_proof_verification {
                    self.verify_proof(
                        batch_result.new_mmr_state(),
                        batch_result.proof().calldata(),
                    )
                    .await?;
                }
            }

            current_end = start_block.saturating_sub(1);
        }

        Ok(())
    }

    pub async fn build_from_finalized(&mut self) -> Result<(), AccumulatorError> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        debug!(
            "Building MMR from finalized block {} with batch size {}",
            finalized_block_number, self.batch_size
        );

        let mut current_end = finalized_block_number;

        while current_end > 0 {
            // Calculate batch index
            let batch_index = current_end / self.batch_size;

            // Determine batch start block
            let batch_start = batch_index * self.batch_size;

            // Adjust start_block to batch_start or 0 if negative
            let start_block = if batch_start > 0 { batch_start } else { 0 };

            // Process the batch
            let batch_result = self.process_batch(start_block, current_end).await?;

            if let Some(batch_result) = batch_result {
                if !self.skip_proof_verification {
                    self.verify_proof(
                        batch_result.new_mmr_state(),
                        batch_result.proof().calldata(),
                    )
                    .await?;
                }
            }

            // Move to the previous batch
            current_end = start_block.saturating_sub(1);
        }

        Ok(())
    }

    /// Update the MMR with new block headers
    pub async fn update_mmr_with_new_headers(
        &mut self,
        start_block: u64,
        end_block: u64,
    ) -> Result<(Vec<Felt>, MmrState), AccumulatorError> {
        let mut current_end = end_block;
        let mut final_result = None;

        while current_end >= start_block {
            // Calculate the batch start (nearest lower block number divisible by batch_size)
            let batch_start = current_end - (current_end % self.batch_size);

            // Don't go below the requested start_block
            let effective_start = batch_start.max(start_block);

            let result = self.process_batch(effective_start, current_end).await?;

            // Store the result of the most recent (highest) batch
            if final_result.is_none() {
                final_result = Some(result);
            }

            // Move to the previous batch
            current_end = effective_start.saturating_sub(1);
        }

        // Extract the final result (guaranteed to exist since we process at least one batch)
        let final_result = final_result.unwrap();

        if let Some(batch_result) = final_result {
            Ok((
                batch_result.proof().calldata(),
                batch_result.new_mmr_state(),
            ))
        } else {
            Err(AccumulatorError::InvalidStateTransition.into())
        }
    }

    pub async fn verify_proof(
        &mut self,
        new_mmr_state: MmrState,
        calldata: Vec<Felt>,
    ) -> Result<(), AccumulatorError> {
        let starknet_provider = StarknetProvider::new(&self.rpc_url)?;
        let starknet_account = StarknetAccount::new(
            starknet_provider.provider(),
            &self.account_private_key,
            &self.account_address,
        )?;

        starknet_account
            .verify_mmr_proof(&self.verifier_address, &new_mmr_state, calldata)
            .await?;

        Ok(())
    }
}

async fn update_mmr_state(
    store_manager: StoreManager,
    mmr: &MMR,
    pool: &SqlitePool,
    latest_block_number: u64,
    guest_output: &GuestOutput,
) -> Result<MmrState, AccumulatorError> {
    debug!(
        "Updating MMR state: elements={}, leaves={}",
        guest_output.elements_count(),
        guest_output.leaves_count()
    );

    // Verify state transition
    let current_elements_count = mmr.elements_count.get().await?;
    if guest_output.elements_count() < current_elements_count {
        warn!(
            "Invalid state transition detected: new count {} < current count {}",
            guest_output.elements_count(),
            current_elements_count
        );
        return Err(AccumulatorError::InvalidStateTransition.into());
    }

    // First update the MMR counters
    mmr.elements_count
        .set(guest_output.elements_count())
        .await?;
    mmr.leaves_count.set(guest_output.leaves_count()).await?;

    // Update all hashes in the store
    for (index, hash) in guest_output.all_hashes() {
        // Store the hash in MMR
        mmr.hashes.set(&hash, SubKey::Usize(index)).await?;

        // Update the mapping
        store_manager
            .insert_value_index_mapping(&pool, &hash, index)
            .await?;
    }

    // Verify the state was properly updated

    let bag = mmr.bag_the_peaks(None).await?;

    let new_mmr_root_hash = mmr.calculate_root_hash(&bag, mmr.elements_count.get().await?)?;

    validate_u256_hex(&new_mmr_root_hash)?;

    let new_mmr_state = MmrState::new(
        latest_block_number,
        u256_from_hex(new_mmr_root_hash.trim_start_matches("0x"))?,
        guest_output.elements_count() as u64,
        guest_output.leaves_count() as u64,
    );

    debug!("MMR state updated successfully");
    Ok(new_mmr_state)
}

/// Validates that a hex string represents a valid U256 (256-bit unsigned integer)
fn validate_u256_hex(hex_str: &str) -> Result<(), AccumulatorError> {
    // Check if it's a valid hex string with '0x' prefix
    if !hex_str.starts_with("0x") {
        return Err(AccumulatorError::InvalidU256Hex(hex_str.to_string()).into());
    }

    // Remove '0x' prefix and check if remaining string is valid hex
    let hex_value = &hex_str[2..];
    if !hex_value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AccumulatorError::InvalidU256Hex(hex_str.to_string()).into());
    }

    // Check length - maximum 64 hex chars (256 bits = 64 hex digits)
    // Note: we allow shorter values as they're valid smaller numbers
    if hex_value.len() > 64 {
        return Err(AccumulatorError::InvalidU256Hex(hex_str.to_string()).into());
    }

    Ok(())
}
