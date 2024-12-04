use crate::core::ProofGenerator;
use crate::errors::ValidatorError;
use common::get_or_create_db_path;
use guest_types::{BlocksValidityInput, GuestProof, MMRInput};
use methods::{BLOCKS_VALIDITY_ELF, BLOCKS_VALIDITY_ID};
use mmr::{PeaksOptions, MMR};
use mmr_utils::{initialize_mmr, StoreManager};
use std::collections::HashMap;
use store::SqlitePool;

pub struct ValidatorBuilder {
    proof_generator: ProofGenerator<BlocksValidityInput>,
    batch_size: u64,
}

impl ValidatorBuilder {
    pub async fn new(batch_size: u64, skip_proof: bool) -> Result<Self, ValidatorError> {
        let proof_generator =
            ProofGenerator::new(BLOCKS_VALIDITY_ELF, BLOCKS_VALIDITY_ID, skip_proof);

        Ok(Self {
            proof_generator,
            batch_size,
        })
    }

    pub async fn verify_blocks_validity_and_inclusion(
        &self,
        headers: &Vec<eth_rlp_types::BlockHeader>,
    ) -> Result<bool, ValidatorError> {
        // Map to store MMRs per batch index
        let mut mmrs: HashMap<u64, (StoreManager, MMR, SqlitePool)> = HashMap::new();
        let mut block_indexes = Vec::new();

        // For each header, determine its batch index and process accordingly
        for header in headers.iter() {
            // Calculate batch index for the block
            let block_number = header.number;
            let batch_index = block_number as u64 / self.batch_size;
            println!("batch index: {}", batch_index);

            // Get or initialize MMR for the batch
            if !mmrs.contains_key(&batch_index) {
                // Determine batch file name
                let batch_file_name = get_or_create_db_path(&format!("batch_{}.db", batch_index))?;
                // Check if batch file exists
                if !std::path::Path::new(&batch_file_name).exists() {
                    return Err(ValidatorError::Store(store::StoreError::GetError));
                }
                // Initialize MMR for the batch
                let (store_manager, mmr, pool) = initialize_mmr(&batch_file_name).await?;
                mmrs.insert(batch_index, (store_manager, mmr, pool));
            }

            // Retrieve the MMR and store manager for the batch
            let (store_manager, _, pool) = mmrs.get(&batch_index).unwrap();

            // Get the index of the block hash in the MMR
            let index = store_manager
                .get_element_index_for_value(pool, &header.block_hash)
                .await?
                .ok_or(ValidatorError::Store(store::StoreError::GetError))?;
            block_indexes.push((index, batch_index));
        }

        // For each batch, prepare MMR inputs and generate proofs
        let mut proofs = Vec::new();
        for (batch_index, (_store_manager, mmr, _pool)) in mmrs.iter() {
            // Get block indexes for this batch
            let batch_block_indexes: Vec<usize> = block_indexes
                .iter()
                .filter(|(_, idx)| idx == batch_index)
                .map(|(index, _)| *index)
                .collect();

            let batch_proofs = mmr.get_proofs(batch_block_indexes.clone(), None).await?;
            // Convert MMR proofs to GuestProofs
            let guest_proofs: Vec<GuestProof> = batch_proofs
                .into_iter()
                .map(|proof| LocalGuestProof::from(proof).into())
                .collect();

            // Get and verify current MMR state
            let current_peaks = mmr.get_peaks(PeaksOptions::default()).await?;
            println!("current peaks: {:?}", current_peaks);
            let current_elements_count = mmr.elements_count.get().await?;
            println!("current elements count: {}", current_elements_count);
            let current_leaves_count = mmr.leaves_count.get().await?;
            println!("current leaves count: {}", current_leaves_count);

            // Prepare MMR input
            let mmr_input = MMRInput::new(
                current_peaks.clone(),
                current_elements_count,
                current_leaves_count,
                vec![], // No new leaves to append
            );

            // Get headers for this batch
            let batch_headers: Vec<eth_rlp_types::BlockHeader> = headers
                .iter()
                .filter(|header| header.number as u64 / self.batch_size == *batch_index)
                .cloned()
                .collect();

            if batch_headers.len() != guest_proofs.len() {
                return Err(ValidatorError::InvalidProofsCount {
                    expected: batch_headers.len(),
                    actual: guest_proofs.len(),
                });
            }
            // Prepare guest input
            let blocks_validity_input =
                BlocksValidityInput::new(batch_headers.clone(), mmr_input, guest_proofs);

            // Generate proof for this batch
            let proof = self
                .proof_generator
                .generate_groth16_proof(blocks_validity_input)
                .await?;

            let guest_output: bool = self.proof_generator.decode_journal(&proof)?;

            // Collect proofs or results
            proofs.push(guest_output);
        }

        // Combine results
        // Assuming we need all proofs to be true
        let all_valid = proofs.iter().all(|&result| result);

        Ok(all_valid)
    }
}

// Add this wrapper struct
pub struct LocalGuestProof {
    pub element_index: usize,
    pub element_hash: String,
    pub siblings_hashes: Vec<String>,
    pub peaks_hashes: Vec<String>,
    pub elements_count: usize,
}

// Implement From for the local wrapper type
impl From<mmr::Proof> for LocalGuestProof {
    fn from(proof: mmr::Proof) -> Self {
        Self {
            element_index: proof.element_index,
            element_hash: proof.element_hash,
            siblings_hashes: proof.siblings_hashes,
            peaks_hashes: proof.peaks_hashes,
            elements_count: proof.elements_count,
        }
    }
}

// Add conversion from LocalGuestProof to GuestProof
impl From<LocalGuestProof> for GuestProof {
    fn from(local: LocalGuestProof) -> Self {
        Self {
            element_index: local.element_index,
            element_hash: local.element_hash,
            siblings_hashes: local.siblings_hashes,
            peaks_hashes: local.peaks_hashes,
            elements_count: local.elements_count,
        }
    }
}
