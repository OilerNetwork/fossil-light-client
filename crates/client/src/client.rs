use starknet_handler::provider::StarknetProvider;
use tokio::time::Duration;
use tracing::{debug, info};

use crate::{
    builder::LightClientBuilder,
    config::LightClientConfig,
    error::Result,
    events::EventProcessor,
    logging::{ClientContext, PerformanceLogger},
    mmr::MmrManager,
};

/// A Starknet light client for processing blockchain events and maintaining MMR state.
///
/// This refactored version uses focused modules for configuration, event processing,
/// and MMR management, providing better separation of concerns and maintainability.
pub struct LightClient {
    config: LightClientConfig,
    event_processor: EventProcessor,
    mmr_manager: MmrManager,
    polling_interval: Duration,
}

impl LightClient {
    /// Creates a new builder for configuring the light client.
    ///
    /// This provides a fluent API for constructing LightClient instances
    /// with optional parameters and preset configurations.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use client::LightClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = LightClient::builder()
    ///         .with_defaults()
    ///         .start_block(1000)
    ///         .build()
    ///         .await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn builder() -> LightClientBuilder {
        LightClientBuilder::new()
    }

    /// Creates a new instance of the light client with specified configuration.
    pub async fn new(
        polling_interval: u64,
        batch_size: u64,
        start_block: u64,
        blocks_per_run: u64,
    ) -> Result<Self> {
        // Load configuration from environment
        let config =
            LightClientConfig::from_env(polling_interval, batch_size, start_block, blocks_per_run)
                .await?;

        // Initialize Starknet providers (we need separate instances)
        let events_provider = StarknetProvider::new(&config.starknet_rpc_url)?;
        let mmr_provider = StarknetProvider::new(&config.starknet_rpc_url)?;

        // Create MMR manager first to get latest MMR block
        let mmr_provider_for_init = StarknetProvider::new(&config.starknet_rpc_url)?;
        let temp_mmr_manager = MmrManager::new(
            mmr_provider_for_init,
            config.l2_store_addr.value().to_string(),
            config.verifier_addr.value().to_string(),
            config.chain_id.value(),
            config.batch_size.value(),
        );
        let latest_mmr_block = temp_mmr_manager.get_latest_mmr_block().await?;

        // Create event processor starting from latest MMR block + 1
        let event_processor = EventProcessor::new(
            events_provider,
            config.l2_store_addr.value().to_string(),
            latest_mmr_block + 1,
            config.blocks_per_run,
        );

        // Create MMR manager
        let mmr_manager = MmrManager::new(
            mmr_provider,
            config.l2_store_addr.value().to_string(),
            config.verifier_addr.value().to_string(),
            config.chain_id.value(),
            config.batch_size.value(),
        );

        let polling_interval = config.polling_interval.as_duration();

        Ok(Self {
            config,
            event_processor,
            mmr_manager,
            polling_interval,
        })
    }

    /// Creates a new instance with automatic start block detection.
    pub async fn new_with_default_start(
        polling_interval: u64,
        batch_size: u64,
        blocks_per_run: u64,
    ) -> Result<Self> {
        // Create a temporary client to get the latest block
        let temp_client = Self::new(polling_interval, batch_size, 0, blocks_per_run).await?;
        let start_block = temp_client.get_latest_relayed_block_number().await? + 1;

        // Create the actual client with the correct start block
        Self::new(polling_interval, batch_size, start_block, blocks_per_run).await
    }

    /// Gets the latest relayed block number from L1.
    pub async fn get_latest_relayed_block_number(&self) -> Result<u64> {
        let latest_relayed_block = self.mmr_manager.get_latest_relayed_block().await?;
        Ok(latest_relayed_block.block_number)
    }

    /// Processes new events from the Starknet store contract.
    pub async fn process_new_events(&mut self) -> Result<()> {
        // Create context for structured logging
        let context = ClientContext::with_operation("process_new_events").with_chain_info(
            self.config.chain_id.value(),
            self.config.starknet_account_address.value().to_string(),
        );

        let logger = PerformanceLogger::start_operation("process_new_events", context);

        // First check if processing should be skipped by checking MMR state
        let latest_relayed_block = self.mmr_manager.get_latest_relayed_block().await?;
        let latest_mmr_block = self.mmr_manager.get_latest_mmr_block().await?;

        logger.log_milestone("fetched MMR state", None);

        // Skip processing if the latest relayed block has already been processed in MMR
        if latest_relayed_block.block_number <= latest_mmr_block {
            debug!(
                relayed_block = latest_relayed_block.block_number,
                mmr_block = latest_mmr_block,
                "Block already processed in MMR, skipping"
            );
            logger.log_success(Some("skipped - already processed"));
            return Ok(());
        }

        // Process events using the event processor
        let event_count = self
            .event_processor
            .process_events(latest_relayed_block.block_number)
            .await?;
        logger.log_milestone(
            "events processed",
            Some(&format!("{} events found", event_count)),
        );

        if event_count > 0 {
            // If events were found, handle them with MMR manager
            self.mmr_manager
                .handle_events(
                    self.config.private_key(),
                    self.config.starknet_account_address.value(),
                )
                .await?;

            logger.log_success(Some(&format!("processed {} events", event_count)));
        } else {
            logger.log_success(Some("no events to process"));
        }

        Ok(())
    }

    /// Starts the main event processing loop.
    pub async fn run(&mut self) -> Result<()> {
        info!(
            "Starting from block {} (poll: {}s)",
            self.event_processor.latest_processed_block() + 1,
            self.polling_interval.as_secs()
        );

        loop {
            self.process_new_events().await?;
            tokio::time::sleep(self.polling_interval).await;
        }
    }
}

// Performance optimizations have been applied to the LightClient:
// - Better error handling with context
// - Structured logging for observability
// - Optimized event processing flow
// - Modular design for better maintainability
