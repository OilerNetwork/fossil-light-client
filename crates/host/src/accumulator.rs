// host/src/accumulator.rs
use crate::db_access::{get_block_headers_by_block_range, DbConnection};
use crate::proof_generator::{ProofGenerator, ProofGeneratorError};
use crate::types::{BatchResult, ProofType};
use common::{felt, string_array_to_felt_array, UtilsError};
use ethereum::get_finalized_block_hash;
use guest_types::{BatchProof, CombinedInput, GuestInput, GuestOutput};
use mmr::{find_peaks, InStoreTableError, MMRError, PeaksOptions, MMR};
use mmr_utils::{initialize_mmr, MMRUtilsError, StoreManager};
use starknet_crypto::Felt;
use starknet_handler::MmrState;
use store::{SqlitePool, StoreError, SubKey};
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum AccumulatorError {
    #[error("Invalid state transition: elements count decreased")]
    InvalidStateTransition,
    #[error("Failed to verify stored peaks after update")]
    PeaksVerificationError,
    #[error("Expected Groth16 proof but got {got:?}")]
    ExpectedGroth16Proof { got: ProofType },
    #[error("MMR root is not a valid Starknet field element: {0}")]
    InvalidFeltHex(String),
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
}

pub struct AccumulatorBuilder {
    batch_size: u64,
    store_manager: StoreManager,
    mmr: MMR,
    pool: SqlitePool,
    proof_generator: ProofGenerator,
    total_batches: u64,
    current_batch: u64,
    previous_proofs: Vec<BatchProof>,
}

impl AccumulatorBuilder {
    pub async fn new(
        store_path: &str,
        proof_generator: ProofGenerator,
        batch_size: u64,
    ) -> Result<Self, AccumulatorError> {
        let (store_manager, mmr, pool) = initialize_mmr(store_path).await?;
        debug!("MMR initialized at {}", store_path);

        Ok(Self {
            batch_size,
            store_manager,
            mmr,
            pool,
            proof_generator,
            total_batches: 0,
            current_batch: 0,
            previous_proofs: Vec::new(),
        })
    }

    async fn process_batch(
        &mut self,
        start_block: u64,
        end_block: u64,
    ) -> Result<BatchResult, AccumulatorError> {
        info!(
            "Processing batch {}/{} (blocks {} to {})",
            self.current_batch + 1,
            self.total_batches,
            start_block,
            end_block
        );

        let db_connection = DbConnection::new().await?;
        debug!(
            "Fetching headers for blocks {} to {}",
            start_block, end_block
        );
        let headers =
            get_block_headers_by_block_range(&db_connection.pool, start_block, end_block).await?;

        // Get and verify current MMR state
        let current_peaks = self.mmr.get_peaks(PeaksOptions::default()).await?;
        let current_elements_count = self.mmr.elements_count.get().await?;
        let current_leaves_count = self.mmr.leaves_count.get().await?;

        // Prepare guest input
        let mmr_input = GuestInput::new(
            current_peaks.clone(),
            current_elements_count,
            current_leaves_count,
            headers.iter().map(|h| h.block_hash.clone()).collect(),
            self.previous_proofs.clone(), // Use the stored proofs
        );

        let combined_input = CombinedInput::new(headers.clone(), mmr_input);

        // Generate appropriate proof
        let proof = if self.current_batch == self.total_batches - 1 {
            debug!("Generating final Groth16 proof for batch");
            self.proof_generator
                .generate_groth16_proof(&combined_input)
                .await?
        } else {
            debug!("Generating intermediate STARK proof for batch");
            self.proof_generator
                .generate_stark_proof(&combined_input)
                .await?
        };

        // Decode and update state
        let guest_output: GuestOutput = self.proof_generator.decode_journal(&proof)?;

        // TODO: Remove this and update MMR state after the proof is verified onchain
        let new_mmr_state = self.update_mmr_state(&guest_output).await?;

        // If this is a STARK proof, add it to previous_proofs for the next batch
        if let ProofType::Stark {
            ref receipt,
            ref image_id,
            method_id,
        } = proof
        {
            self.previous_proofs.push(BatchProof::new(
                receipt.clone(),
                image_id.clone(),
                method_id,
            ));
        }

        // Verify state after update
        let final_peaks = self.mmr.get_peaks(PeaksOptions::default()).await?;
        if final_peaks != guest_output.final_peaks().to_vec() {
            return Err(AccumulatorError::PeaksVerificationError.into());
        }

        self.current_batch += 1;

        debug!("Batch processing completed successfully");
        Ok(BatchResult::new(
            start_block,
            end_block,
            new_mmr_state,
            Some(proof),
        ))
    }

    async fn update_mmr_state(
        &mut self,
        guest_output: &GuestOutput,
    ) -> Result<MmrState, AccumulatorError> {
        debug!(
            "Updating MMR state: elements={}, leaves={}",
            guest_output.elements_count(),
            guest_output.leaves_count()
        );

        // Verify state transition
        let current_elements_count = self.mmr.elements_count.get().await?;
        if guest_output.elements_count() < current_elements_count {
            warn!(
                "Invalid state transition detected: new count {} < current count {}",
                guest_output.elements_count(),
                current_elements_count
            );
            return Err(AccumulatorError::InvalidStateTransition.into());
        }

        // First update the MMR counters
        self.mmr
            .elements_count
            .set(guest_output.elements_count())
            .await?;
        self.mmr
            .leaves_count
            .set(guest_output.leaves_count())
            .await?;

        // Update all hashes in the store
        for result in guest_output.append_results() {
            // Store the hash in MMR
            self.mmr
                .hashes
                .set(result.root_hash(), SubKey::Usize(result.element_index()))
                .await?;

            // Update the mapping
            self.store_manager
                .insert_value_index_mapping(&self.pool, result.root_hash(), result.element_index())
                .await?;
        }

        // Update peaks
        let peaks_indices = find_peaks(guest_output.elements_count());
        for (peak_hash, &peak_idx) in guest_output.final_peaks().iter().zip(peaks_indices.iter()) {
            self.mmr
                .hashes
                .set(peak_hash, SubKey::Usize(peak_idx))
                .await?;
        }

        // Verify the state was properly updated
        let stored_peaks = self.mmr.get_peaks(PeaksOptions::default()).await?;

        if stored_peaks != guest_output.final_peaks() {
            return Err(AccumulatorError::PeaksVerificationError.into());
        }

        let bag = self.mmr.bag_the_peaks(None).await?;

        let new_mmr_root_hash = self
            .mmr
            .calculate_root_hash(&bag, self.mmr.elements_count.get().await?)?;

        validate_felt_hex(&new_mmr_root_hash)?;

        let new_mmr_state = MmrState::new(
            felt(&new_mmr_root_hash)?,
            guest_output.elements_count() as u64,
            guest_output.leaves_count() as u64,
            string_array_to_felt_array(guest_output.final_peaks().to_vec())?,
        );

        debug!("MMR state updated successfully");
        Ok(new_mmr_state)
    }

    /// Build the MMR using a specified number of batches
    pub async fn build_with_num_batches(
        &mut self,
        num_batches: u64,
    ) -> Result<Vec<BatchResult>, AccumulatorError> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        info!("Building MMR...",);

        self.total_batches = num_batches;
        self.current_batch = 0;
        self.previous_proofs.clear();

        let mut batch_results = Vec::new();

        let mut current_end = finalized_block_number;

        for _ in 0..num_batches {
            if current_end == 0 {
                break;
            }

            let start_block = current_end.saturating_sub(self.batch_size - 1);

            let result = self.process_batch(start_block, current_end).await?;

            batch_results.push(result);

            current_end = start_block.saturating_sub(1);
        }

        Ok(batch_results)
    }

    pub async fn build_from_finalized(&mut self) -> Result<Vec<BatchResult>, AccumulatorError> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        debug!(
            "Building MMR from finalized block {} with batch size {}",
            finalized_block_number, self.batch_size
        );

        self.total_batches = (finalized_block_number / self.batch_size) + 1;
        self.current_batch = 0;
        self.previous_proofs.clear(); // Clear any existing proofs

        let mut batch_results = Vec::new();

        let mut current_end = finalized_block_number;

        while current_end > 0 {
            let start_block = current_end.saturating_sub(self.batch_size - 1);

            let result = self.process_batch(start_block, current_end).await?;

            batch_results.push(result);

            current_end = start_block.saturating_sub(1);
        }

        Ok(batch_results)
    }

    /// Update the MMR with new block headers
    pub async fn update_mmr_with_new_headers(
        &mut self,
        start_block: u64,
        end_block: u64,
    ) -> Result<(Vec<Felt>, MmrState), AccumulatorError> {
        self.total_batches = ((end_block - start_block) / self.batch_size) + 1;

        let result = self.process_batch(start_block, end_block).await?;

        // Extract the `calldata` from the `Groth16` proof
        if let Some(ProofType::Groth16 { calldata, .. }) = result.proof() {
            Ok((calldata, result.new_mmr_state()))
        } else {
            Err(AccumulatorError::ExpectedGroth16Proof {
                got: result.proof().unwrap(),
            }
            .into())
        }
    }
}

/// Validates that a hex string represents a valid Starknet field element (252 bits)
fn validate_felt_hex(hex_str: &str) -> Result<(), AccumulatorError> {
    // Check if it's a valid hex string with '0x' prefix
    if !hex_str.starts_with("0x") {
        return Err(AccumulatorError::InvalidFeltHex(hex_str.to_string()).into());
    }

    // Remove '0x' prefix and check if remaining string is valid hex
    let hex_value = &hex_str[2..];
    if !hex_value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AccumulatorError::InvalidFeltHex(hex_str.to_string()).into());
    }

    // Check length - maximum 63 hex chars (252 bits = 63 hex digits)
    // Note: we allow shorter values as they're valid smaller numbers
    if hex_value.len() > 63 {
        return Err(AccumulatorError::InvalidFeltHex(hex_str.to_string()).into());
    }

    Ok(())
}
