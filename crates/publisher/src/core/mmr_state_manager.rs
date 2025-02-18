use crate::errors::AccumulatorError;
use crate::utils::validate_u256_hex;
use guest_types::GuestOutput;
use mmr::MMR;
use mmr_utils::StoreManager;
use starknet_handler::{account::StarknetAccount, u256_from_hex, MmrState};
use store::SqlitePool;
use tracing::{debug, error, info};
// use jsonrpc_client::{JsonRpcClient, HttpTransport, Url};
// use std::sync::Arc;

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

        info!("Updating MMR state with {} headers...", headers.len());
        debug!("Headers: {:?}", headers);

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
            debug!("No guest output provided, creating state from MMR directly");
            let bag = mmr.bag_the_peaks(None).await.map_err(|e| {
                error!(error = %e, "Failed to bag the peaks");
                e
            })?;

            let elements_count = mmr.elements_count.get().await.map_err(|e| {
                error!(error = %e, "Failed to get elements count");
                e
            })?;
            debug!("Elements count: {}", elements_count);

            let root_hash = mmr.calculate_root_hash(&bag, elements_count).map_err(|e| {
                error!(error = %e, "Failed to calculate root hash");
                e
            })?;
            debug!("Raw root hash: {}", root_hash);

            let root_hash_hex = if !root_hash.starts_with("0x") {
                format!("0x{}", root_hash)
            } else {
                root_hash
            };
            debug!("Formatted root hash: {}", root_hash_hex);

            let root_hash_u256 = u256_from_hex(&root_hash_hex).map_err(|e| {
                error!(error = %e, "Failed to convert root hash to U256");
                e
            })?;

            let latest_header = headers.last().unwrap();
            debug!("Latest header: {}", latest_header);

            let latest_mmr_block_hash = u256_from_hex(latest_header).map_err(|e| {
                error!(error = %e, "Failed to convert latest header to U256");
                e
            })?;

            let leaves_count = mmr.leaves_count.get().await.map_err(|e| {
                error!(error = %e, "Failed to get leaves count");
                e
            })?;
            debug!("Leaves count: {}", leaves_count);

            let new_mmr_state = MmrState::new(
                latest_block_number,
                latest_mmr_block_hash,
                root_hash_u256,
                leaves_count as u64,
                None,
            );

            info!(
                "Created MMR state: latest_block={}, leaves={}",
                latest_block_number, leaves_count
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use mmr_utils::StoreManager;
    use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient, Url};
    use starknet_handler::account::StarknetAccount;
    use std::sync::Arc;
    use store::memory::InMemoryStore;

    // Helper function to create test dependencies
    async fn setup_test() -> (MMRStateManager<'static>, StoreManager, MMR, SqlitePool) {
        let account = StarknetAccount::new(
            Arc::new(JsonRpcClient::new(HttpTransport::new(
                Url::parse("http://localhost:5050").expect("Invalid URL"),
            ))),
            "0x1234567890abcdef", // Valid hex address
            "0x1234567890abcdef", // Valid hex private key
        )
        .expect("Failed to create StarknetAccount");

        let store_address = "0x1234567890abcdef"; // Valid hex store address
        let mmr_state_manager = MMRStateManager::new(account, store_address);

        let memory_store = Arc::new(InMemoryStore::new(None));
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory SQLite database");

        // Create the required table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS value_index_map (
                value TEXT PRIMARY KEY,
                element_index INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create value_index_map table");

        let store_manager = StoreManager::new("sqlite::memory:")
            .await
            .expect("Failed to create StoreManager");

        let mmr = MMR::new(
            memory_store.clone(),
            Arc::new(hasher::hashers::sha2::Sha2Hasher::new()),
            None,
        );

        debug!("Test dependencies created successfully");
        (mmr_state_manager, store_manager, mmr, pool)
    }

    #[tokio::test]
    async fn test_update_state_without_guest_output() {
        let (manager, store_manager, mut mmr, pool) = setup_test().await;

        let headers = vec![
            "0x0000000000000000000000000000000000000000000000001234567890abcdef".to_string(),
            "0x0000000000000000000000000000000000000000000000000deadbeefcafe000".to_string(),
        ];

        // MMR is already initialized by MMR::new()
        let result = manager
            .update_state(store_manager, &mut mmr, &pool, 100, None, &headers)
            .await;

        match &result {
            Ok(_) => debug!("Update state succeeded"),
            Err(e) => error!("Update state failed: {:?}", e),
        }

        assert!(
            result.is_ok(),
            "Update state failed: {:?}",
            result.err().unwrap()
        );
        let state = result.unwrap();
        assert_eq!(state.latest_mmr_block(), 100);
        assert_eq!(state.leaves_count(), 2);
    }

    #[tokio::test]
    async fn test_update_state_with_empty_headers() {
        let (manager, store_manager, mut mmr, pool) = setup_test().await;

        let result = manager
            .update_state(store_manager, &mut mmr, &pool, 100, None, &vec![])
            .await;

        assert!(matches!(result, Err(AccumulatorError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_append_headers_with_empty_hash() {
        let (_, store_manager, mut mmr, pool) = setup_test().await;

        let headers = vec!["".to_string()];

        let result =
            MMRStateManager::append_headers(store_manager, &mut mmr, &pool, &headers).await;

        assert!(matches!(result, Err(AccumulatorError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_create_new_state() {
        let guest_output = GuestOutput::new(
            1,                                                                                // batch_index
            100, // latest_mmr_block
            "0x0000000000000000000000000000000000000000000000001234567890abcdef".to_string(), // 64 chars hex
            "0x0000000000000000000000000000000000000000000000001234567890abcdef".to_string(), // 64 chars hex
            10, // leaves_count
            "0x0000000000000000000000000000000000000000000000001234567890abcdef".to_string(), // 64 chars hex
            [(0, 100), (1, 200), (2, 300), (3, 400)],
        );

        let result = MMRStateManager::create_new_state(100, &guest_output).await;

        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state.latest_mmr_block(), 100);
        assert_eq!(state.leaves_count(), 10);
    }
}
