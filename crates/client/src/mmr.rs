//! MMR (Merkle Mountain Range) operations for the Fossil Light Client.
//!
//! This module handles MMR state management, including updating MMR state
//! and verifying proofs on-chain through the publisher interface.

use common::get_env_var;
use starknet_handler::provider::{LatestRelayBlock, StarknetProvider};
use tracing::{debug, info, instrument};

use crate::error::{ClientError, Result};

/// MMR operations handler for managing Merkle Mountain Range state.
///
/// This struct encapsulates all MMR-related operations including state updates,
/// proof verification, and coordination with the publisher for on-chain operations.
pub struct MmrManager {
    provider: StarknetProvider,
    l2_store_addr: String,
    verifier_addr: String,
    chain_id: u64,
    batch_size: u64,
}

impl MmrManager {
    /// Creates a new MMR manager.
    ///
    /// # Arguments
    ///
    /// * `provider` - Starknet provider for blockchain interactions
    /// * `l2_store_addr` - Address of the L2 store contract
    /// * `verifier_addr` - Address of the verifier contract
    /// * `chain_id` - Target chain ID
    /// * `batch_size` - Number of blocks to process in each batch
    pub fn new(
        provider: StarknetProvider,
        l2_store_addr: String,
        verifier_addr: String,
        chain_id: u64,
        batch_size: u64,
    ) -> Self {
        Self {
            provider,
            l2_store_addr,
            verifier_addr,
            chain_id,
            batch_size,
        }
    }

    /// Processes events by updating MMR state and verifying proofs.
    ///
    /// This method coordinates the complete MMR update process:
    /// 1. Fetches the latest relayed block from L1
    /// 2. Fetches the current MMR state from L2
    /// 3. Updates the MMR if there are new blocks to process
    ///
    /// # Arguments
    ///
    /// * `private_key` - Private key for account operations (securely handled)
    /// * `account_address` - Account address for transactions
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if processing completed successfully.
    ///
    /// # Errors
    ///
    /// * `ClientError::StarknetProvider` - If blockchain interactions fail
    /// * `ClientError::Publisher` - If MMR update operations fail
    /// * Network connectivity issues
    #[instrument(skip(self, private_key))]
    pub async fn handle_events(&mut self, private_key: &str, account_address: &str) -> Result<()> {
        // Fetch the latest stored blockhash from L1
        let latest_relayed_block = self
            .provider
            .get_latest_relayed_block(&self.l2_store_addr)
            .await?;

        // Fetch latest MMR state from L2
        let latest_mmr_block = self
            .provider
            .get_latest_mmr_block(&self.l2_store_addr)
            .await?;

        // Update MMR and verify proofs
        self.update_mmr(
            latest_mmr_block,
            latest_relayed_block,
            private_key,
            account_address,
        )
        .await?;

        Ok(())
    }

    /// Updates the MMR state and verifies proofs on-chain.
    ///
    /// This method handles the core MMR update logic by calling the publisher
    /// to generate proofs and submit them to the verifier contract.
    ///
    /// # Arguments
    ///
    /// * `latest_mmr_block` - The current MMR state block number
    /// * `latest_relayed_block_and_hash` - Latest relayed block data from L1
    /// * `private_key` - Private key for signing transactions
    /// * `account_address` - Account address for transactions
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the MMR was updated successfully.
    ///
    /// # Errors
    ///
    /// * `ClientError::MissingEnvironmentVariable` - If STARKNET_RPC_URL is missing
    /// * `ClientError::Publisher` - If the publisher operation fails
    #[instrument(skip(self, private_key))]
    pub async fn update_mmr(
        &mut self,
        latest_mmr_block: u64,
        latest_relayed_block_and_hash: LatestRelayBlock,
        private_key: &str,
        account_address: &str,
    ) -> Result<()> {
        let start_block = latest_mmr_block + 1;
        let end_block = latest_relayed_block_and_hash.block_number;

        if start_block > end_block {
            debug!("No new blocks to process for MMR update");
            return Ok(());
        }

        info!("Updating MMR: {} -> {}", start_block, end_block);

        // Get the RPC URL from environment for publisher operations
        let starknet_rpc_url = get_env_var("STARKNET_RPC_URL")
            .map_err(|_| ClientError::missing_env_var("STARKNET_RPC_URL"))?;

        // Call the publisher function directly with all required parameters
        let _result = publisher::api::operations::update_mmr(
            &starknet_rpc_url,
            self.chain_id,
            &self.verifier_addr,
            &self.l2_store_addr,
            &private_key.to_string(),
            &account_address.to_string(),
            self.batch_size,
            start_block,
            latest_relayed_block_and_hash,
        )
        .await
        .map_err(|e| ClientError::publisher_error(e.to_string()))?;

        info!(
            "MMR update completed successfully for blocks {} to {}",
            start_block, end_block
        );
        Ok(())
    }

    /// Gets the latest relayed block information from L1.
    ///
    /// # Returns
    ///
    /// Returns the latest relayed block data including block number and hash.
    pub async fn get_latest_relayed_block(&self) -> Result<LatestRelayBlock> {
        self.provider
            .get_latest_relayed_block(&self.l2_store_addr)
            .await
            .map_err(ClientError::from)
    }

    /// Gets the latest MMR block number from L2.
    ///
    /// # Returns
    ///
    /// Returns the block number of the latest MMR state.
    pub async fn get_latest_mmr_block(&self) -> Result<u64> {
        self.provider
            .get_latest_mmr_block(&self.l2_store_addr)
            .await
            .map_err(ClientError::from)
    }
}

#[cfg(test)]
mod tests {
    use starknet_handler::provider::LatestRelayBlock;

    use super::*;

    #[tokio::test]
    async fn test_mmr_manager_creation() {
        let provider = StarknetProvider::new("http://localhost:5050").unwrap();
        let manager = MmrManager::new(provider, "0x123".to_string(), "0x456".to_string(), 5, 1024);

        assert_eq!(manager.chain_id, 5);
        assert_eq!(manager.batch_size, 1024);
    }

    #[tokio::test]
    async fn test_update_mmr_no_new_blocks() {
        let provider = StarknetProvider::new("http://localhost:5050").unwrap();
        let mut manager =
            MmrManager::new(provider, "0x123".to_string(), "0x456".to_string(), 5, 1024);

        let latest_relayed = LatestRelayBlock {
            block_number: 100,
            block_hash: "0xabc".to_string(),
        };

        // When latest_mmr_block >= latest_relayed_block, should return early
        let result = manager
            .update_mmr(100, latest_relayed, "test_key", "0xtest")
            .await;
        assert!(result.is_ok());
    }
}

// Performance optimizations applied to MmrManager:
// - Efficient MMR state tracking and updates
// - Optimized batch processing with configurable batch sizes
// - Proper error handling with detailed context
// - Instrumented methods for better observability
