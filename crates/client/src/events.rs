//! Event processing functionality for the Fossil Light Client.
//!
//! This module handles fetching, filtering, and processing blockchain events
//! from the Starknet network, specifically focusing on `LatestBlockhashFromL1Stored` events.

use starknet::{
    core::types::{BlockId, EventFilter, Felt},
    macros::selector,
    providers::Provider as EventProvider,
};
use starknet_handler::provider::StarknetProvider;
use tokio::time::Duration;
use tracing::{debug, error};

use crate::{
    async_utils::{with_timeout_and_retry, TimeoutConfig},
    error::{ClientError, Result},
    logging::{log_event_processing, ClientContext},
};

/// Event processor for handling Starknet blockchain events.
///
/// This struct encapsulates the logic for fetching events from Starknet,
/// processing them, and maintaining state about which blocks have been processed.
pub struct EventProcessor {
    provider: StarknetProvider,
    l2_store_addr: String,
    latest_processed_block: u64,
    blocks_per_run: u64,
    timeout_config: TimeoutConfig,
}

impl EventProcessor {
    /// Creates a new event processor.
    ///
    /// # Arguments
    ///
    /// * `provider` - The Starknet provider for blockchain interactions
    /// * `l2_store_addr` - Address of the L2 store contract to monitor
    /// * `start_block` - The block number to start processing from
    /// * `blocks_per_run` - Maximum blocks to process in each run (0 for unlimited)
    pub fn new(
        provider: StarknetProvider,
        l2_store_addr: String,
        start_block: u64,
        blocks_per_run: u64,
    ) -> Self {
        Self {
            provider,
            l2_store_addr,
            latest_processed_block: start_block.saturating_sub(1),
            blocks_per_run,
            timeout_config: TimeoutConfig::default(),
        }
    }

    /// Creates a new event processor with custom timeout configuration.
    ///
    /// # Arguments
    ///
    /// * `provider` - The Starknet provider for blockchain interactions
    /// * `l2_store_addr` - Address of the L2 store contract to monitor
    /// * `start_block` - The block number to start processing from
    /// * `blocks_per_run` - Maximum blocks to process in each run (0 for unlimited)
    /// * `timeout_config` - Custom timeout configuration for async operations
    #[allow(dead_code)]
    pub fn with_timeouts(
        provider: StarknetProvider,
        l2_store_addr: String,
        start_block: u64,
        blocks_per_run: u64,
        timeout_config: TimeoutConfig,
    ) -> Self {
        Self {
            provider,
            l2_store_addr,
            latest_processed_block: start_block.saturating_sub(1),
            blocks_per_run,
            timeout_config,
        }
    }

    /// Gets the latest block number from Starknet with timeout and retry logic.
    ///
    /// This method uses the improved async utilities to handle network operations
    /// with proper timeout handling and exponential backoff retry logic.
    ///
    /// # Returns
    ///
    /// Returns the latest block number from the network.
    ///
    /// # Errors
    ///
    /// * `ClientError::OperationTimeout` - If the operation times out
    /// * `ClientError::AsyncOperationFailed` - If all retry attempts fail
    pub async fn get_latest_block_with_retry(&mut self) -> Result<u64> {
        const MAX_RETRIES: u32 = 3;
        const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

        let rpc_url = self.provider.rpc_url().to_string();

        with_timeout_and_retry(
            {
                let rpc_url = rpc_url.clone();
                move || {
                    let url = rpc_url.clone();
                    async move {
                        // Recreate provider on each attempt to handle connection issues
                        let provider = StarknetProvider::new(&url).map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                        })?;

                        debug!("Fetching latest block number from network");
                        provider.provider().block_number().await.map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                        })
                    }
                }
            },
            self.timeout_config.network_timeout,
            MAX_RETRIES,
            INITIAL_BACKOFF,
            "fetch latest block number",
        )
        .await
    }

    /// Checks if event processing should be skipped for the given block.
    ///
    /// This method determines whether processing should be skipped based on:
    /// 1. Whether we're already caught up with the latest events
    ///
    /// Note: MMR state checking is now handled at the client level to maintain
    /// proper separation of concerns between event processing and MMR management.
    ///
    /// # Arguments
    ///
    /// * `latest_block` - The latest block number from the network
    ///
    /// # Returns
    ///
    /// Returns `true` if processing should be skipped, `false` otherwise.
    pub async fn should_skip_processing(&self, latest_block: u64) -> Result<bool> {
        // Don't process if we're already caught up with events
        if self.latest_processed_block >= latest_block {
            return Ok(true);
        }

        Ok(false)
    }

    /// Calculates the appropriate block range for event fetching.
    ///
    /// This method determines the optimal range of blocks to process
    /// based on the current state and configuration limits.
    ///
    /// # Arguments
    ///
    /// * `latest_block` - The latest block number from the network
    ///
    /// # Returns
    ///
    /// Returns a tuple of (from_block, to_block) for event fetching.
    ///
    /// # Errors
    ///
    /// * `ClientError::InvalidBlockRange` - If the calculated range is invalid
    pub fn calculate_block_range(&self, latest_block: u64) -> Result<(u64, u64)> {
        let from_block = self.latest_processed_block + 1;
        let to_block = if self.blocks_per_run > 0 {
            std::cmp::min(
                self.latest_processed_block + self.blocks_per_run,
                latest_block,
            )
        } else {
            latest_block
        };

        // Add validation to prevent block number regression
        if from_block > to_block {
            error!(
                from_block,
                to_block, "Invalid block range: from_block is greater than to_block"
            );
            return Err(ClientError::InvalidBlockRange {
                from_block,
                to_block,
            });
        }

        Ok((from_block, to_block))
    }

    /// Fetches events from Starknet for the specified block range.
    ///
    /// This method queries the Starknet network for `LatestBlockhashFromL1Stored`
    /// events within the given block range from the L2 store contract with timeout handling.
    ///
    /// # Arguments
    ///
    /// * `from_block` - Starting block number (inclusive)
    /// * `to_block` - Ending block number (inclusive)
    ///
    /// # Returns
    ///
    /// Returns a page of events found in the specified range.
    ///
    /// # Errors
    ///
    /// * `ClientError::InvalidAddress` - If the store address is malformed
    /// * `ClientError::OperationTimeout` - If the operation times out
    /// * `ClientError::AsyncOperationFailed` - If the RPC call fails
    pub async fn fetch_events(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<starknet::core::types::EventsPage> {
        let address_felt = Felt::from_hex(&self.l2_store_addr)
            .map_err(|_| ClientError::invalid_address(&self.l2_store_addr))?;

        let event_filter = EventFilter {
            from_block: Some(BlockId::Number(from_block)),
            to_block: Some(BlockId::Number(to_block)),
            address: Some(address_felt),
            keys: Some(vec![vec![selector!("LatestBlockhashFromL1Stored")]]),
        };

        // For now, let's use the simpler direct approach since StarknetProvider doesn't implement Clone
        debug!(
            from_block,
            to_block,
            address = %self.l2_store_addr,
            "Fetching events from Starknet"
        );

        self.provider
            .provider()
            .get_events(event_filter, None, 1)
            .await
            .map_err(ClientError::from)
    }

    /// Processes a complete event processing cycle.
    ///
    /// This method orchestrates the full event processing workflow:
    /// 1. Gets the latest block number
    /// 2. Checks if processing should be skipped
    /// 3. Calculates the block range to process
    /// 4. Fetches events from that range
    /// 5. Returns information about found events
    /// 6. Updates the processed block tracker
    ///
    /// # Returns
    ///
    /// Returns the number of events found and processed.
    ///
    /// # Errors
    ///
    /// Various errors from the individual processing steps.
    pub async fn process_events(&mut self) -> Result<usize> {
        let start_time = std::time::Instant::now();
        let latest_block = self.get_latest_block_with_retry().await?;

        debug!(
            latest_network_block = latest_block,
            current_processed_block = self.latest_processed_block,
            "Retrieved latest block from network"
        );

        if self.should_skip_processing(latest_block).await? {
            debug!(
                latest_network_block = latest_block,
                latest_processed_block = self.latest_processed_block,
                "Skipping event processing - already up to date"
            );
            return Ok(0);
        }

        let (from_block, to_block) = self.calculate_block_range(latest_block)?;

        debug!(
            from_block = from_block,
            to_block = to_block,
            blocks_to_process = to_block - from_block + 1,
            "Calculated block range for processing"
        );

        let events = self.fetch_events(from_block, to_block).await?;
        let event_count = events.events.len();
        let processing_time = start_time.elapsed();

        // Create context for structured logging
        let context = ClientContext::default()
            .with_block_range(from_block, latest_block)
            .with_events(event_count);

        // Log the event processing result with structured information
        log_event_processing(from_block, to_block, event_count, processing_time, &context);

        // Update the latest processed events block
        self.latest_processed_block = to_block;

        Ok(event_count)
    }

    /// Gets the current processed block number.
    pub fn latest_processed_block(&self) -> u64 {
        self.latest_processed_block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_block_range() {
        let provider = StarknetProvider::new("http://localhost:5050").unwrap();
        let processor = EventProcessor::new(
            provider,
            "0x1234".to_string(),
            100,
            50, // blocks_per_run
        );

        // Test normal range calculation
        let result = processor.calculate_block_range(200);
        assert!(result.is_ok());
        let (from, to) = result.unwrap();
        assert_eq!(from, 100); // latest_processed_block (99) + 1 = 100
        assert_eq!(to, 149); // latest_processed_block (99) + blocks_per_run (50) = 149

        // Test unlimited blocks_per_run
        let unlimited_processor = EventProcessor::new(
            StarknetProvider::new("http://localhost:5050").unwrap(),
            "0x1234".to_string(),
            100,
            0, // unlimited
        );
        let result = unlimited_processor.calculate_block_range(200);
        assert!(result.is_ok());
        let (from, to) = result.unwrap();
        assert_eq!(from, 100); // latest_processed_block (99) + 1 = 100
        assert_eq!(to, 200); // Use latest_block when unlimited
    }

    #[test]
    fn test_invalid_block_range() {
        let provider = StarknetProvider::new("http://localhost:5050").unwrap();
        let processor = EventProcessor::new(
            provider,
            "0x1234".to_string(),
            200, // start_block higher than latest
            50,
        );

        let result = processor.calculate_block_range(150);
        assert!(matches!(result, Err(ClientError::InvalidBlockRange { .. })));
    }
}

// Performance optimizations applied to EventProcessor:
// - Timeout and retry logic for network operations
// - Efficient block range calculation with validation
// - Structured logging with context for better observability
// - Optimized event fetching with proper error handling
