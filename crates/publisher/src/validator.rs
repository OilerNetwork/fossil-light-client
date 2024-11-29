use crate::proof_generator::{ProofGenerator, ProofGeneratorError};
use common::get_db_path;
use guest_types::{BlocksValidityInput, MMRInput};
use methods::{BLOCKS_VALIDITY_ELF, BLOCKS_VALIDITY_ID};
use mmr::{MMRError, PeaksOptions, MMR};
use mmr_utils::{initialize_mmr, StoreManager};
use store::SqlitePool;

#[derive(thiserror::Error, Debug)]
pub enum ValidatorError {
    #[error("Utils error: {0}")]
    Utils(#[from] common::UtilsError),
    #[error("MMR error: {0}")]
    MMRUtils(#[from] mmr_utils::MMRUtilsError),
    #[error("Store error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Store error: {0}")]
    Store(#[from] store::StoreError),
    #[error("MMR error: {0}")]
    MMRError(#[from] MMRError),
    #[error("ProofGenerator error: {0}")]
    ProofGenerator(#[from] ProofGeneratorError),
}

pub struct ValidatorBuilder {
    store: StoreManager,
    mmr: MMR,
    pool: SqlitePool,
    proof_generator: ProofGenerator<BlocksValidityInput>,
}

impl ValidatorBuilder {
    pub async fn new(skip_proof: bool) -> Result<Self, ValidatorError> {
        let proof_generator =
            ProofGenerator::new(BLOCKS_VALIDITY_ELF, BLOCKS_VALIDITY_ID, skip_proof);

        let store_path = get_db_path()?;
        let (store, mmr, pool) = initialize_mmr(&store_path).await?;

        Ok(Self {
            store,
            mmr,
            pool,
            proof_generator,
        })
    }

    pub async fn verify_blocks_validity_and_inclusion(
        &self,
        headers: &Vec<eth_rlp_types::BlockHeader>,
    ) -> Result<bool, ValidatorError> {
        let mut block_indexes = Vec::new();

        for header in headers.iter() {
            let index = self
                .store
                .get_element_index_for_value(&self.pool, &header.block_hash)
                .await?
                .ok_or(ValidatorError::Store(store::StoreError::GetError))?;
            block_indexes.push(index);
        }

        // Get and verify current MMR state
        let current_peaks = self.mmr.get_peaks(PeaksOptions::default()).await?;
        let current_elements_count = self.mmr.elements_count.get().await?;
        let current_leaves_count = self.mmr.leaves_count.get().await?;

        // Prepare guest input
        let mmr_input = MMRInput::new(
            current_peaks.clone(),
            current_elements_count,
            current_leaves_count,
            None,
            None,
        );

        let blocks_validity_input =
            BlocksValidityInput::new(headers.clone(), mmr_input, block_indexes);

        let proof = self
            .proof_generator
            .generate_groth16_proof(blocks_validity_input)
            .await?;

        let guest_output: bool = self.proof_generator.decode_journal(&proof)?;

        Ok(guest_output)
    }
}
