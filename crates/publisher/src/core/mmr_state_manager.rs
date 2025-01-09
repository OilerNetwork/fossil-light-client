use crate::errors::AccumulatorError;
use crate::utils::validate_u256_hex;
use guest_types::GuestOutput;
use mmr::MMR;
use mmr_utils::StoreManager;
use starknet_handler::{account::StarknetAccount, u256_from_hex, MmrState};
use store::SqlitePool;
use tracing::{debug, error, info};

pub struct MMRStateManager<'a> {
    account: StarknetAccount,
    store_address: &'a str,
}

impl<'a> MMRStateManager<'a> {
    pub fn new(account: StarknetAccount, store_address: &'a str) -> Self {
        Self {
            account,
            store_address,
        }
    }

    pub fn account(&self) -> &StarknetAccount {
        &self.account
    }

    pub fn store_address(&self) -> &'a str {
        self.store_address
    }

    pub async fn update_state(
        &self,
        store_manager: StoreManager,
        mmr: &mut MMR,
        pool: &SqlitePool,
        latest_block_number: u64,
        guest_output: Option<&GuestOutput>,
        headers: &Vec<String>,
    ) -> Result<MmrState, AccumulatorError> {
        if headers.is_empty() {
            return Err(AccumulatorError::InvalidInput(
                "Headers list cannot be empty",
            ));
        }

        info!("Updating MMR state...");

        Self::append_headers(store_manager, mmr, pool, headers)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to append headers");
                e
            })?;

        if let Some(guest_output) = guest_output {
            Self::verify_mmr_state(mmr, guest_output)
                .await
                .map_err(|e| {
                    error!(error = %e, "Failed to verify MMR state");
                    e
                })?;

            let new_mmr_state = Self::create_new_state(latest_block_number, guest_output)
                .await
                .map_err(|e| {
                    error!(error = %e, "Failed to create new MMR state");
                    e
                })?;

            info!("MMR state updated successfully");
            Ok(new_mmr_state)
        } else {
            // When no guest output, create state directly from MMR
            let bag = mmr.bag_the_peaks(None).await.map_err(|e| {
                error!(error = %e, "Failed to bag the peaks");
                e
            })?;
            let elements_count = mmr.elements_count.get().await.map_err(|e| {
                error!(error = %e, "Failed to get elements count");
                e
            })?;
            let root_hash = mmr.calculate_root_hash(&bag, elements_count).map_err(|e| {
                error!(error = %e, "Failed to calculate root hash");
                e
            })?;
            let leaves_count = mmr.leaves_count.get().await.map_err(|e| {
                error!(error = %e, "Failed to get leaves count");
                e
            })?;
            let latest_mmr_block_hash = u256_from_hex(
                headers
                    .last()
                    .ok_or(AccumulatorError::InvalidInput("Headers list is empty"))?,
            )
            .map_err(|e| {
                error!(error = %e, "Failed to convert root hash from hex");
                e
            })?;

            let new_mmr_state = MmrState::new(
                latest_block_number,
                latest_mmr_block_hash,
                u256_from_hex(&root_hash.trim_start_matches("0x")).map_err(|e| {
                    error!(error = %e, "Failed to convert root hash from hex");
                    e
                })?,
                leaves_count as u64,
                None,
            );

            info!("No verification option selected, MMR state not updated onchain");
            Ok(new_mmr_state)
        }
    }

    async fn append_headers(
        store_manager: StoreManager,
        mmr: &mut MMR,
        pool: &SqlitePool,
        headers: &Vec<String>,
    ) -> Result<(), AccumulatorError> {
        debug!("Appending headers to MMR");

        for hash in headers {
            if hash.trim().is_empty() {
                return Err(AccumulatorError::InvalidInput(
                    "Header hash cannot be empty",
                ));
            }

            let append_result = mmr.append(hash.clone()).await.map_err(|e| {
                error!(error = %e, "Failed to append hash to MMR");
                e
            })?;
            store_manager
                .insert_value_index_mapping(&pool, &hash, append_result.element_index)
                .await
                .map_err(|e| {
                    error!(error = %e, "Failed to insert value index mapping");
                    e
                })?;
        }
        debug!("Headers appended successfully");
        Ok(())
    }

    async fn verify_mmr_state(
        mmr: &MMR,
        guest_output: &GuestOutput,
    ) -> Result<(), AccumulatorError> {
        debug!("Verifying MMR state");

        let leaves_count = mmr.leaves_count.get().await.map_err(|e| {
            error!(error = %e, "Failed to get leaves count");
            e
        })?;
        if leaves_count != guest_output.leaves_count() as usize {
            error!("Leaves count mismatch");
            return Err(AccumulatorError::InvalidStateTransition);
        }

        let new_element_count = mmr.elements_count.get().await.map_err(|e| {
            error!(error = %e, "Failed to get elements count");
            e
        })?;
        let bag = mmr.bag_the_peaks(None).await.map_err(|e| {
            error!(error = %e, "Failed to bag the peaks");
            e
        })?;
        let new_root_hash = mmr
            .calculate_root_hash(&bag, new_element_count)
            .map_err(|e| {
                error!(error = %e, "Failed to calculate root hash");
                e
            })?;

        if new_root_hash != guest_output.root_hash() {
            error!("Root hash mismatch");
            return Err(AccumulatorError::InvalidStateTransition);
        }

        validate_u256_hex(&new_root_hash).map_err(|e| {
            error!(error = %e, "Invalid root hash format");
            e
        })?;

        debug!("MMR state verified successfully");
        Ok(())
    }

    async fn create_new_state(
        latest_block_number: u64,
        guest_output: &GuestOutput,
    ) -> Result<MmrState, AccumulatorError> {
        debug!("Creating new MMR state");

        let root_hash = guest_output.root_hash().trim_start_matches("0x");
        if root_hash.is_empty() {
            return Err(AccumulatorError::InvalidInput("Root hash cannot be empty"));
        }

        let latest_mmr_block_hash =
            u256_from_hex(guest_output.latest_mmr_block_hash()).map_err(|e| {
                error!(error = %e, "Failed to convert latest mmr block hash from hex");
                e
            })?;
        let new_state = MmrState::new(
            latest_block_number,
            latest_mmr_block_hash,
            latest_mmr_block_hash,
            guest_output.leaves_count() as u64,
            None,
        );

        debug!("New MMR state created successfully");
        Ok(new_state)
    }
}
