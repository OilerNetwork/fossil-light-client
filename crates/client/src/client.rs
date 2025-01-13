use common::get_env_var;
use mmr_utils::{create_database_file, ensure_directory_exists};
use starknet::{
    core::types::{BlockId, EventFilter, Felt},
    macros::selector,
    providers::Provider as EventProvider,
};
use starknet_handler::provider::StarknetProvider;
use tokio::time::{self, Duration};
use tracing::{error, info, instrument};

#[derive(thiserror::Error, Debug)]
pub enum LightClientError {
    #[error("Starknet handler error: {0}")]
    StarknetHandler(#[from] starknet_handler::StarknetHandlerError),
    #[error("Utils error: {0}")]
    UtilsError(#[from] common::UtilsError),
    #[error("MMR utils error: {0}")]
    MmrUtilsError(#[from] mmr_utils::MMRUtilsError),
    #[error("Publisher error: {0}")]
    PublisherError(#[from] publisher::PublisherError),
    #[error("Starknet provider error: {0}")]
    StarknetProvider(#[from] starknet::providers::ProviderError),
    #[error("Database file does not exist at path: {0}")]
    ConfigError(String),
    #[error("Polling interval must be greater than zero")]
    PollingIntervalError,
    #[error("Chain ID is not a valid number")]
    ChainIdError(#[from] std::num::ParseIntError),
    #[error("Felt conversion error: {0}")]
    FeltConversion(#[from] starknet::core::types::FromStrError),
}

pub struct LightClient {
    starknet_provider: StarknetProvider,
    l2_store_addr: String,
    verifier_addr: String,
    chain_id: u64,
    latest_processed_events_block: u64,
    latest_processed_mmr_block: u64,
    starknet_private_key: String,
    starknet_account_address: String,
    polling_interval: Duration,
    batch_size: u64,
    blocks_per_run: u64,
}

impl LightClient {
    /// Creates a new instance of the light client.
    pub async fn new(
        polling_interval: u64,
        batch_size: u64,
        start_block: u64,
        blocks_per_run: u64,
    ) -> Result<Self, LightClientError> {
        if polling_interval == 0 {
            error!("Polling interval must be greater than zero");
            return Err(LightClientError::PollingIntervalError);
        }

        // Load environment variables
        let starknet_rpc_url = get_env_var("STARKNET_RPC_URL")?;
        let l2_store_addr = get_env_var("FOSSIL_STORE")?;
        let verifier_addr = get_env_var("FOSSIL_VERIFIER")?;
        let starknet_private_key = get_env_var("STARKNET_PRIVATE_KEY")?;
        let starknet_account_address = get_env_var("STARKNET_ACCOUNT_ADDRESS")?;
        let chain_id = get_env_var("CHAIN_ID")?.parse::<u64>()?;

        // Initialize providers
        let starknet_provider = StarknetProvider::new(&starknet_rpc_url)?;

        // Set up the database file path
        let current_dir = ensure_directory_exists("../../db-instances")?;
        let db_file = create_database_file(&current_dir, 0)?;

        if !std::path::Path::new(&db_file).exists() {
            error!("Database file does not exist at path: {}", db_file);
            return Err(LightClientError::ConfigError(db_file));
        }

        Ok(Self {
            starknet_provider,
            l2_store_addr,
            verifier_addr,
            chain_id,
            latest_processed_events_block: start_block.saturating_sub(1),
            latest_processed_mmr_block: start_block.saturating_sub(1),
            starknet_private_key,
            starknet_account_address,
            polling_interval: Duration::from_secs(polling_interval),
            batch_size,
            blocks_per_run,
        })
    }

    /// Runs the light client event loop.
    pub async fn run(&mut self) -> Result<(), LightClientError> {
        let mut interval = time::interval(self.polling_interval);

        // Create the shutdown signal once
        let mut shutdown = Box::pin(tokio::signal::ctrl_c());

        info!(
            polling_interval_secs = self.polling_interval.as_secs(),
            start_block = self.latest_processed_events_block + 1,
            blocks_per_run = self.blocks_per_run,
            "Light client started"
        );

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.process_new_events().await {
                        error!(error = %e, "Event processing failed");
                    }
                }
                _ = &mut shutdown => {
                    info!("Light client stopped");
                    break Ok(());
                }
            }
        }
    }

    /// Processes new events from the Starknet store contract.
    pub async fn process_new_events(&mut self) -> Result<(), LightClientError> {
        // Get the latest block number
        let latest_block = match self.starknet_provider.provider().block_number().await? {
            number => number.saturating_sub(10), // Stay 10 blocks behind to handle reorgs
        };

        info!(
            latest_block,
            last_processed_events = self.latest_processed_events_block,
            last_processed_mmr = self.latest_processed_mmr_block,
            "Checking for new events"
        );

        // Don't process if we're already caught up with events
        if self.latest_processed_events_block >= latest_block {
            info!(
                latest_block,
                last_processed_events = self.latest_processed_events_block,
                "Already up to date with latest events"
            );
            return Ok(());
        }

        // Calculate the to_block based on blocks_per_run
        let to_block = if self.blocks_per_run > 0 {
            std::cmp::min(
                self.latest_processed_events_block + self.blocks_per_run,
                latest_block,
            )
        } else {
            latest_block
        };

        let from_block = self.latest_processed_events_block + 1;

        // Add validation to prevent block number regression
        if from_block > to_block {
            error!(
                from_block,
                to_block, "Invalid block range: from_block is greater than to_block"
            );
            return Ok(());
        }

        info!(
            from_block,
            to_block,
            blocks_to_process = to_block - from_block + 1,
            "Processing block range for events"
        );

        let event_filter = EventFilter {
            from_block: Some(BlockId::Number(from_block)),
            to_block: Some(BlockId::Number(to_block)),
            address: Some(Felt::from_hex(&self.l2_store_addr)?),
            keys: Some(vec![vec![selector!("LatestBlockhashFromL1Stored")]]),
        };

        let events = self
            .starknet_provider
            .provider()
            .get_events(event_filter, None, 1)
            .await?;

        info!(
            from_block,
            to_block,
            event_count = events.events.len(),
            "Retrieved events from Starknet"
        );

        // Update the latest processed events block
        let old_processed_block = self.latest_processed_events_block;
        self.latest_processed_events_block = to_block;

        info!(
            old_processed = old_processed_block,
            new_processed = self.latest_processed_events_block,
            blocks_advanced = self.latest_processed_events_block - old_processed_block,
            "Updated processed events block"
        );

        if !events.events.is_empty() {
            info!(event_count = events.events.len(), "Processing new events");

            // Process the events and update MMR
            self.handle_events().await?;
        }

        Ok(())
    }

    /// Handles the events by updating the MMR and verifying proofs.
    #[instrument(skip(self))]
    pub async fn handle_events(&mut self) -> Result<(), LightClientError> {
        // Fetch the latest stored blockhash from L1
        let latest_relayed_block = self
            .starknet_provider
            .get_latest_relayed_block(&self.l2_store_addr)
            .await?;

        // Fetch latest MMR state from L2
        let latest_mmr_block = self
            .starknet_provider
            .get_latest_mmr_block(&self.l2_store_addr)
            .await?;

        // Update MMR and verify proofs
        self.update_mmr(latest_mmr_block, latest_relayed_block)
            .await?;

        Ok(())
    }

    /// Updates the MMR and verifies the proof on-chain.
    #[instrument(skip(self))]
    pub async fn update_mmr(
        &mut self,
        latest_mmr_block: u64,
        latest_relayed_block: u64,
    ) -> Result<(), LightClientError> {
        info!(
            latest_mmr_block,
            latest_relayed_block,
            current_processed_mmr = self.latest_processed_mmr_block,
            "Starting MMR update"
        );

        // If MMR is already up to date with the relayed block, nothing to do
        if latest_mmr_block >= latest_relayed_block {
            info!(
                latest_mmr_block,
                latest_relayed_block, "MMR already up to date with latest relayed block"
            );
            return Ok(());
        }

        info!(
            from_block = latest_mmr_block + 1,
            to_block = latest_relayed_block,
            batch_size = self.batch_size,
            "Starting proof verification"
        );

        // Update MMR
        publisher::prove_mmr_update(
            &self.starknet_provider.rpc_url().to_string(),
            self.chain_id,
            &self.verifier_addr,
            &self.l2_store_addr,
            &self.starknet_private_key,
            &self.starknet_account_address,
            self.batch_size,
            latest_mmr_block + 1,
            latest_relayed_block,
            false,
        )
        .await?;

        // Update our tracking of the latest processed MMR block
        self.latest_processed_mmr_block = latest_relayed_block;

        info!("Proof verification completed successfully");
        Ok(())
    }
}
