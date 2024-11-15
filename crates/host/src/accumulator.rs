// host/src/accumulator.rs
use crate::proof_generator::ProofGenerator;
use crate::types::{BatchResult, ProofType};
use db_access::rpc::get_block_headers_in_range;
use eyre::Result;
use guest_types::{BatchProof, CombinedInput, GuestInput, GuestOutput};
use mmr::{find_peaks, PeaksOptions};
use mmr_accumulator::{
    ethereum::get_finalized_block_hash, processor_utils::*, store::StoreManager, MMR,
};
use starknet_crypto::Felt;
use store::{SqlitePool, SubKey};
use tracing::{debug, info};

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
    ) -> Result<Self> {
        let (store_manager, mmr, pool) = initialize_mmr(store_path).await?;

        // Configure pool with appropriate settings
        sqlx::migrate!("./migrations").run(&pool).await?;

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

    async fn process_batch(&mut self, start_block: u64, end_block: u64) -> Result<BatchResult> {
        // Fetch headers
        let headers = get_block_headers_in_range(start_block, end_block).await?;
        debug!("Fetched {} headers", headers.len());

        // Get and verify current MMR state
        let current_peaks = self.mmr.get_peaks(PeaksOptions::default()).await?;
        let current_elements_count = self.mmr.elements_count.get().await?;
        let current_leaves_count = self.mmr.leaves_count.get().await?;

        debug!("Current MMR state:");
        debug!("  Elements count: {}", current_elements_count);
        debug!("  Leaves count: {}", current_leaves_count);
        debug!("  Peaks: {:?}", current_peaks);

        // Prepare guest input
        let mmr_input = GuestInput {
            initial_peaks: current_peaks.clone(),
            elements_count: current_elements_count,
            leaves_count: current_leaves_count,
            new_elements: headers.iter().map(|h| h.block_hash.clone()).collect(),
            previous_proofs: self.previous_proofs.clone(), // Use the stored proofs
        };

        let combined_input = CombinedInput {
            headers: headers.clone(),
            mmr_input,
        };

        // Generate appropriate proof
        let proof = if self.current_batch == self.total_batches - 1 {
            self.proof_generator
                .generate_groth16_proof(&combined_input)
                .await?
        } else {
            self.proof_generator
                .generate_stark_proof(&combined_input)
                .await?
        };

        // Decode and update state
        let guest_output: GuestOutput = self.proof_generator.decode_journal(&proof)?;
        let new_mmr_root_hash = self.update_mmr_state(&guest_output).await?;

        // If this is a STARK proof, add it to previous_proofs for the next batch
        if let ProofType::Stark {
            ref receipt,
            ref image_id,
            method_id,
        } = proof
        {
            self.previous_proofs.push(BatchProof {
                receipt: receipt.clone(),
                image_id: image_id.clone(),
                method_id,
            });
        }

        // Verify state after update
        let final_peaks = self.mmr.get_peaks(PeaksOptions::default()).await?;
        if final_peaks != guest_output.final_peaks {
            return Err(eyre::eyre!(
                "Final peaks verification failed after batch processing"
            ));
        }

        self.current_batch += 1;

        Ok(BatchResult {
            start_block,
            end_block,
            new_mmr_root_hash,
            proof: Some(proof),
        })
    }

    async fn update_mmr_state(&mut self, guest_output: &GuestOutput) -> Result<String> {
        info!("Guest output: {:?}", guest_output);
        // Verify state transition
        let current_elements_count = self.mmr.elements_count.get().await?;
        if guest_output.elements_count < current_elements_count {
            return Err(eyre::eyre!(
                "Invalid state transition: elements count decreased"
            ));
        }

        info!("Updating MMR state:");
        info!("  Current elements count: {}", current_elements_count);
        info!("  New elements count: {}", guest_output.elements_count);
        info!("  New peaks: {:?}", guest_output.final_peaks);

        // First update the MMR counters
        self.mmr
            .elements_count
            .set(guest_output.elements_count)
            .await?;
        self.mmr.leaves_count.set(guest_output.leaves_count).await?;

        // Update all hashes in the store
        for result in &guest_output.append_results {
            info!(
                "  Storing hash at index {}: {}",
                result.element_index, result.root_hash
            );

            // Store the hash in MMR
            self.mmr
                .hashes
                .set(&result.root_hash, SubKey::Usize(result.element_index))
                .await?;

            // Update the mapping
            self.store_manager
                .insert_value_index_mapping(&self.pool, &result.root_hash, result.element_index)
                .await?;
        }

        // Update peaks
        let peaks_indices = find_peaks(guest_output.elements_count);
        for (peak_hash, &peak_idx) in guest_output.final_peaks.iter().zip(peaks_indices.iter()) {
            info!("  Storing peak at index {}: {}", peak_idx, peak_hash);
            self.mmr
                .hashes
                .set(peak_hash, SubKey::Usize(peak_idx))
                .await?;
        }

        // Verify the state was properly updated
        let stored_peaks = self.mmr.get_peaks(PeaksOptions::default()).await?;
        info!("Verified stored peaks: {:?}", stored_peaks);

        if stored_peaks != guest_output.final_peaks {
            return Err(eyre::eyre!("Failed to verify stored peaks after update"));
        }

        let bag = self.mmr.bag_the_peaks(None).await?;

        let new_mmr_root_hash = self
            .mmr
            .calculate_root_hash(&bag, self.mmr.elements_count.get().await?)?;

        Ok(new_mmr_root_hash)
    }

    /// Build the MMR using a specified number of batches
    pub async fn build_with_num_batches(&mut self, num_batches: u64) -> Result<Vec<BatchResult>> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;
        self.total_batches = num_batches;
        self.current_batch = 0;
        self.previous_proofs.clear();

        let mut batch_results = Vec::new();
        let mut current_end = finalized_block_number;

        for _ in 0..num_batches {
            if current_end == 0 {
                break;
            }

            let start_block = current_end.saturating_sub(self.batch_size as u64 - 1);
            info!(
                "Processing batch {}/{}: {} to {}",
                self.current_batch + 1,
                self.total_batches,
                start_block,
                current_end
            );

            let result = self.process_batch(start_block, current_end).await?;
            batch_results.push(result);
            current_end = start_block.saturating_sub(1);
        }

        Ok(batch_results)
    }

    pub async fn build_from_finalized(&mut self) -> Result<Vec<BatchResult>> {
        let (finalized_block_number, _) = get_finalized_block_hash().await?;

        self.total_batches = (finalized_block_number / self.batch_size as u64) + 1;
        self.current_batch = 0;
        self.previous_proofs.clear(); // Clear any existing proofs

        let mut batch_results = Vec::new();
        let mut current_end = finalized_block_number;

        while current_end > 0 {
            let start_block = current_end.saturating_sub(self.batch_size as u64 - 1);
            info!(
                "Processing batch {}/{}: {} to {}",
                self.current_batch + 1,
                self.total_batches,
                start_block,
                current_end
            );

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
    ) -> Result<(Vec<Felt>, String)> {
        self.total_batches = ((end_block - start_block) / self.batch_size as u64) + 1;

        let result = self.process_batch(start_block, end_block).await?;

        // Extract the `calldata` from the `Groth16` proof
        if let ProofType::Groth16 { calldata, .. } = result.proof.unwrap() {
            Ok((calldata, result.new_mmr_root_hash))
        } else {
            Err(eyre::eyre!(
                "Expected Groth16 proof but got a different proof type"
            ))
        }
    }
}
