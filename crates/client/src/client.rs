use std::sync::Arc;
use eyre::Result;
use tracing::{error, info};
use std::time::Duration;
use tokio::time::sleep;

use common::get_env_var;
use host::update_mmr_and_verify_onchain;
use mmr_accumulator::processor_utils::{create_database_file, ensure_directory_exists};
use starknet::{
    core::types::{BlockId, BlockTag, EventFilter, Felt},
    macros::selector,
    providers::Provider as EventProvider,
};
use starknet_handler::{account::StarknetAccount, provider::StarknetProvider};

pub struct LightClient {
    starknet_provider: StarknetProvider,
    l2_store_addr: Felt,
    verifier_addr: String,
    latest_processed_block: u64,
    db_file: String,
    starknet_private_key: String,
    starknet_account_address: String,
}

impl LightClient {
    /// Creates a new instance of the light client.
    pub async fn new() -> Result<Self> {
        // Load environment variables
        let starknet_rpc_url = get_env_var("STARKNET_RPC_URL")?;
        let l2_store_addr = Felt::from_hex(&get_env_var("FOSSIL_STORE")?)?;
        let verifier_addr = get_env_var("STARKNET_VERIFIER")?;
        let starknet_private_key = get_env_var("STARKNET_PRIVATE_KEY")?;
        let starknet_account_address = get_env_var("STARKNET_ACCOUNT_ADDRESS")?;

        // Initialize providers
        let starknet_provider = StarknetProvider::new(&starknet_rpc_url);

        // Set up the database file path
        let current_dir = ensure_directory_exists("db-store")?;
        let db_file = create_database_file(&current_dir, 0)?;

        Ok(Self {
            starknet_provider,
            l2_store_addr,
            verifier_addr,
            latest_processed_block: 0,
            db_file,
            starknet_private_key,
            starknet_account_address,
        })
    }

    /// Runs the light client event loop.
    pub async fn run(&mut self) -> Result<()> {
        loop {
            info!("Listening for new events...");

            match self.process_new_events().await {
                Ok(_) => {
                    // Continue to the next iteration
                }
                Err(e) => {
                    error!("Error processing events: {:?}", e);
                }
            }

            sleep(Duration::from_secs(60)).await; // Check every 60 seconds
        }
    }

    /// Processes new events from the Starknet store contract.
    pub async fn process_new_events(&mut self) -> Result<()> {
        // Poll for new events, starting from the block after the last processed block
        let event_filter = EventFilter {
            from_block: Some(BlockId::Number(self.latest_processed_block + 1)),
            to_block: Some(BlockId::Tag(BlockTag::Latest)),
            address: Some(self.l2_store_addr),
            keys: Some(vec![vec![selector!("LatestBlockhashFromL1Stored")]]),
        };

        let events = self
            .starknet_provider
            .provider
            .get_events(event_filter, None, 1)
            .await?;

        if !events.events.is_empty() {
            info!("Fetched {} new events", events.events.len());

            // Update the latest processed block to the latest block from the new events
            self.latest_processed_block = events
                .events
                .last()
                .and_then(|event| event.block_number)
                .unwrap_or(self.latest_processed_block);

            // Process the events
            self.handle_events().await?;
        } else {
            info!("No new events found.");
        }

        Ok(())
    }

    /// Handles the events by updating the MMR and verifying proofs.
    pub async fn handle_events(&self) -> Result<()> {
        // Fetch the latest stored blockhash from L1
        let latest_relayed_block = self
            .starknet_provider
            .get_latest_relayed_block(&self.l2_store_addr)
            .await?;

        // Fetch latest MMR state from L2
        let (latest_mmr_block, _latest_mmr_root) = self
            .starknet_provider
            .get_latest_mmr_state(&self.l2_store_addr)
            .await?;

        info!(
            "Latest MMR block on Starknet: {}",
            latest_mmr_block
        );
        info!(
            "Latest relayed block number on Starknet: {}",
            latest_relayed_block
        );

        // Call Risc0 prover to verify the block headers, append to MMR, and verify SNARK proof
        self.update_mmr(latest_mmr_block, latest_relayed_block)
            .await?;

        Ok(())
    }

    /// Updates the MMR and verifies the proof on-chain.
    pub async fn update_mmr(&self, latest_mmr_block: u64, latest_relayed_block: u64) -> Result<()> {
        info!(
            "Calling Risc0, proving block headers from {} to {}",
            latest_mmr_block + 1,
            latest_relayed_block
        );

        let (proof_verified, new_mmr_root) = update_mmr_and_verify_onchain(
            &self.db_file,
            latest_mmr_block,
            latest_relayed_block,
            &self.starknet_provider.rpc_url,
            &self.verifier_addr,
        )
        .await?;

        info!("Proof verified: {:?}", proof_verified);
        info!("New MMR root: {:?}", new_mmr_root);

        // If SNARK proof is valid, update the latest stored blockhash and MMR root on L2
        if proof_verified {
            self.update_mmr_state_on_starknet(latest_relayed_block, new_mmr_root)
                .await?;
        } else {
            error!("Proof verification failed.");
        }

        Ok(())
    }

    /// Updates the MMR state on Starknet.
    pub async fn update_mmr_state_on_starknet(
        &self,
        latest_relayed_block: u64,
        new_mmr_root: String,
    ) -> Result<()> {
        info!("Updating MMR state on Starknet...");

        let starknet_account = StarknetAccount::new(
            Arc::clone(&self.starknet_provider.provider),
            &self.starknet_private_key,
            &self.starknet_account_address,
        );

        starknet_account
            .update_mmr_state(
                self.l2_store_addr,
                latest_relayed_block,
                Felt::from_hex(&new_mmr_root)?,
            )
            .await?;

        info!(
            "MMR state updated on Starknet with latest relayed block number: {}, new MMR root: {}",
            latest_relayed_block, new_mmr_root
        );

        Ok(())
    }
}