use crate::errors::AccumulatorError;
use crate::utils::validate_u256_hex;
use guest_types::GuestOutput;
use mmr::MMR;
use mmr_utils::StoreManager;
use starknet_handler::{u256_from_hex, MmrState};
use store::SqlitePool;

pub struct MMRStateManager;

impl MMRStateManager {
    pub async fn update_state(
        store_manager: StoreManager,
        mmr: &mut MMR,
        pool: &SqlitePool,
        latest_block_number: u64,
        guest_output: &GuestOutput,
        headers: &Vec<String>,
    ) -> Result<MmrState, AccumulatorError> {
        Self::append_headers(store_manager, mmr, pool, headers).await?;
        Self::verify_mmr_state(mmr, guest_output).await?;
        let new_mmr_state = Self::create_new_state(latest_block_number, guest_output).await?;

        Ok(new_mmr_state)
    }

    async fn append_headers(
        store_manager: StoreManager,
        mmr: &mut MMR,
        pool: &SqlitePool,
        headers: &Vec<String>,
    ) -> Result<(), AccumulatorError> {
        for hash in headers {
            let append_result = mmr.append(hash.clone()).await?;
            store_manager
                .insert_value_index_mapping(&pool, &hash, append_result.element_index)
                .await?;
        }
        Ok(())
    }

    async fn verify_mmr_state(
        mmr: &MMR,
        guest_output: &GuestOutput,
    ) -> Result<(), AccumulatorError> {
        if mmr.leaves_count.get().await? != guest_output.leaves_count() as usize {
            return Err(AccumulatorError::InvalidStateTransition);
        }

        let new_element_count = mmr.elements_count.get().await?;
        let bag = mmr.bag_the_peaks(None).await?;
        let new_root_hash = mmr.calculate_root_hash(&bag, new_element_count)?;

        if new_root_hash != guest_output.root_hash() {
            return Err(AccumulatorError::InvalidStateTransition);
        }

        validate_u256_hex(&new_root_hash)?;

        Ok(())
    }

    async fn create_new_state(
        latest_block_number: u64,
        guest_output: &GuestOutput,
    ) -> Result<MmrState, AccumulatorError> {
        Ok(MmrState::new(
            latest_block_number,
            u256_from_hex(guest_output.root_hash().trim_start_matches("0x"))?,
            guest_output.leaves_count() as u64,
        ))
    }
}
