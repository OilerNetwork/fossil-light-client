use common::get_env_var;
use mmr_utils::{create_database_file, ensure_directory_exists};
use starknet::{
    core::types::{BlockId, BlockTag, EventFilter, Felt},
    macros::selector,
    providers::Provider as EventProvider,
};
use starknet_handler::provider::StarknetProvider;
use tokio::time::{self, Duration};
use tracing::{error, info, instrument, warn};

const BATCH_SIZE: u64 = 1024;

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
    #[error("latest_processed_block regression from {0} to {1}")]
    StateError(u64, u64),
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
    latest_processed_block: u64,
    starknet_private_key: String,
    starknet_account_address: String,
    polling_interval: Duration,
}

impl LightClient {
    /// Creates a new instance of the light client.
    pub async fn new(polling_interval: u64) -> Result<Self, LightClientError> {
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
            latest_processed_block: 0,
            starknet_private_key,
            starknet_account_address,
            polling_interval: Duration::from_secs(polling_interval),
        })
    }

    /// Runs the light client event loop.
    pub async fn run(&mut self) -> Result<(), LightClientError> {
        let mut interval = time::interval(self.polling_interval);

        // Create the shutdown signal once
        let mut shutdown = Box::pin(tokio::signal::ctrl_c());

        info!(
            polling_interval_secs = self.polling_interval.as_secs(),
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
        // Poll for new events, starting from the block after the last processed block
        let event_filter = EventFilter {
            from_block: Some(BlockId::Number(self.latest_processed_block + 1)),
            to_block: Some(BlockId::Tag(BlockTag::Latest)),
            address: Some(Felt::from_hex(&self.l2_store_addr)?),
            keys: Some(vec![vec![selector!("LatestBlockhashFromL1Stored")]]),
        };

        let events = self
            .starknet_provider
            .provider()
            .get_events(event_filter, None, 1)
            .await?;

        if !events.events.is_empty() {
            info!(
                event_count = events.events.len(),
                latest_block = self.latest_processed_block,
                "New events processed"
            );

            // Update the latest processed block to the latest block from the new events
            let new_latest_block = events
                .events
                .last()
                .and_then(|event| event.block_number)
                .unwrap_or(self.latest_processed_block);

            // Invariant check: new_latest_block should be greater or equal to the current
            if new_latest_block < self.latest_processed_block {
                error!(
                    "New latest_processed_block ({}) is less than the current ({})",
                    new_latest_block, self.latest_processed_block
                );
                return Err(LightClientError::StateError(
                    self.latest_processed_block,
                    new_latest_block,
                ));
            }

            self.latest_processed_block = new_latest_block;

            // Process the events
            self.handle_events().await?;
        }

        Ok(())
    }

    /// Handles the events by updating the MMR and verifying proofs.
    #[instrument(skip(self))]
    pub async fn handle_events(&self) -> Result<(), LightClientError> {
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
        &self,
        latest_mmr_block: u64,
        latest_relayed_block: u64,
    ) -> Result<(), LightClientError> {
        if latest_mmr_block >= latest_relayed_block {
            warn!(
                latest_mmr_block,
                latest_relayed_block,
                "Latest MMR block is greater than the latest relayed block, skipping proof verification"
            );
            return Err(LightClientError::StateError(
                latest_mmr_block,
                latest_relayed_block,
            ));
        }
        info!("Starting proof verification...");

        publisher::prove_mmr_update(
            &self.starknet_provider.rpc_url().to_string(),
            self.chain_id,
            &self.verifier_addr,
            &self.l2_store_addr,
            &self.starknet_private_key,
            &self.starknet_account_address,
            BATCH_SIZE,
            latest_mmr_block + 1,
            latest_relayed_block,
            false,
        )
        .await?;
        Ok(())
    }
}
