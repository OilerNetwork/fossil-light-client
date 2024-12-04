use crate::errors::ValidatorError;
use crate::{core::ProofGenerator, utils::Stark};
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
    ) -> Result<Vec<Stark>, ValidatorError> {
        let mmrs = self.initialize_mmrs_for_headers(headers).await?;
        let block_indexes = self.collect_block_indexes(headers, &mmrs).await?;
        self.generate_proofs_for_batches(headers, &mmrs, &block_indexes)
            .await
    }

    async fn initialize_mmrs_for_headers(
        &self,
        headers: &[eth_rlp_types::BlockHeader],
    ) -> Result<HashMap<u64, (StoreManager, MMR, SqlitePool)>, ValidatorError> {
        let mut mmrs = HashMap::new();

        for header in headers {
            let batch_index = header.number as u64 / self.batch_size;

            if !mmrs.contains_key(&batch_index) {
                let batch_file_name = get_or_create_db_path(&format!("batch_{}.db", batch_index))?;
                if !std::path::Path::new(&batch_file_name).exists() {
                    return Err(ValidatorError::Store(store::StoreError::GetError));
                }
                let mmr_components = initialize_mmr(&batch_file_name).await?;
                mmrs.insert(batch_index, mmr_components);
            }
        }

        Ok(mmrs)
    }

    async fn collect_block_indexes(
        &self,
        headers: &[eth_rlp_types::BlockHeader],
        mmrs: &HashMap<u64, (StoreManager, MMR, SqlitePool)>,
    ) -> Result<Vec<(usize, u64)>, ValidatorError> {
        let mut block_indexes = Vec::new();

        for header in headers {
            let batch_index = header.number as u64 / self.batch_size;
            let (store_manager, _, pool) = mmrs.get(&batch_index).unwrap();

            let index = store_manager
                .get_element_index_for_value(pool, &header.block_hash)
                .await?
                .ok_or(ValidatorError::Store(store::StoreError::GetError))?;

            block_indexes.push((index, batch_index));
        }

        Ok(block_indexes)
    }

    async fn generate_proofs_for_batches(
        &self,
        headers: &[eth_rlp_types::BlockHeader],
        mmrs: &HashMap<u64, (StoreManager, MMR, SqlitePool)>,
        block_indexes: &[(usize, u64)],
    ) -> Result<Vec<Stark>, ValidatorError> {
        let mut proofs = Vec::new();

        for (batch_index, (_, mmr, _)) in mmrs {
            let batch_block_indexes = self.get_batch_block_indexes(block_indexes, *batch_index);
            let batch_headers = self.get_batch_headers(headers, *batch_index);

            let batch_proofs = mmr.get_proofs(&batch_block_indexes, None).await?;
            let guest_proofs = self.convert_to_guest_proofs(batch_proofs);

            if batch_headers.len() != guest_proofs.len() {
                return Err(ValidatorError::InvalidProofsCount {
                    expected: batch_headers.len(),
                    actual: guest_proofs.len(),
                });
            }

            let mmr_input = self.prepare_mmr_input(mmr).await?;
            let blocks_validity_input =
                BlocksValidityInput::new(batch_headers, mmr_input, guest_proofs);

            let proof = self
                .proof_generator
                .generate_stark_proof(blocks_validity_input)
                .await?;

            proofs.push(proof);
        }

        Ok(proofs)
    }

    fn get_batch_block_indexes(
        &self,
        block_indexes: &[(usize, u64)],
        batch_index: u64,
    ) -> Vec<usize> {
        block_indexes
            .iter()
            .filter(|(_, idx)| *idx == batch_index)
            .map(|(index, _)| *index)
            .collect()
    }

    fn get_batch_headers(
        &self,
        headers: &[eth_rlp_types::BlockHeader],
        batch_index: u64,
    ) -> Vec<eth_rlp_types::BlockHeader> {
        headers
            .iter()
            .filter(|header| header.number as u64 / self.batch_size == batch_index)
            .cloned()
            .collect()
    }

    fn convert_to_guest_proofs(&self, batch_proofs: Vec<mmr::Proof>) -> Vec<GuestProof> {
        batch_proofs
            .into_iter()
            .map(|proof| LocalGuestProof::from(proof).into())
            .collect()
    }

    async fn prepare_mmr_input(&self, mmr: &MMR) -> Result<MMRInput, ValidatorError> {
        let current_peaks = mmr.get_peaks(PeaksOptions::default()).await?;
        let current_elements_count = mmr.elements_count.get().await?;
        let current_leaves_count = mmr.leaves_count.get().await?;

        Ok(MMRInput::new(
            current_peaks,
            current_elements_count,
            current_leaves_count,
            vec![],
        ))
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
