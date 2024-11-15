use std::sync::Arc;

use dotenv::dotenv;
use eyre::Result;
use host::update_mmr_and_verify_onchain;
use mmr_accumulator::processor_utils::{create_database_file, ensure_directory_exists};
use starknet::{
    core::types::{BlockId, BlockTag, EventFilter, Felt},
    macros::selector,
    providers::Provider as EventProvider,
};
use starknet_handler::{StarknetAccount, StarknetProvider};
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Fossil Light Client...");

    let starknet_rpc_url = dotenv::var("STARKNET_RPC_URL").expect("STARKNET_RPC_URL not set");
    let l2_store_addr =
        Felt::from_hex(&dotenv::var("FOSSIL_STORE").expect("FOSSIL_STORE not set")).unwrap();
    let verifier_addr = dotenv::var("STARKNET_VERIFIER").expect("STARKNET_VERIFIER not set");

    let starknet_provider = StarknetProvider::new(&starknet_rpc_url);

    // Variable to track the latest processed block number
    let mut latest_processed_block: u64 = 0;

    loop {
        info!("Listening for new events...");
        // Poll for new events, starting from the block after the last processed block
        let event_filter = EventFilter {
            from_block: Some(BlockId::Number(latest_processed_block + 1)),
            to_block: Some(BlockId::Tag(BlockTag::Latest)),
            address: Some(l2_store_addr),
            keys: Some(vec![vec![selector!("LatestBlockhashFromL1Stored")]]),
        };

        let events = starknet_provider
            .provider
            .get_events(event_filter, None, 1)
            .await?;

        if !events.events.is_empty() {
            info!("Fetched {} new events", events.events.len());

            // Update the latest processed block to the latest block from the new events
            latest_processed_block = events
                .events
                .last()
                .unwrap()
                .block_number
                .expect("Block number not found");

            // Fetch the latest stored blockhash from L1
            let latest_relayed_block =
                starknet_provider.get_latest_relayed_block(&l2_store_addr).await?;

            // Fetch latest MMR state from L2
            let (latest_mmr_block, latest_mmr_root) =
                starknet_provider.get_latest_mmr_state(&l2_store_addr).await?;

            info!(
                "Latest MMR state on Starknet: block number: {:?}, root: {:?}",
                latest_mmr_block, latest_mmr_root
            );
            info!(
                "Latest relayed block number on Starknet: {}",
                latest_relayed_block
            );

            // Call Risc0 prover to verify the blockheaders, append to MMR, and verify SNARK proof
            let current_dir = ensure_directory_exists("db-store")?;
            let db_file = create_database_file(&current_dir, 0)?;

            info!(
                "Calling Risc0, proving blockheaders from {:?} to {:?}",
                latest_mmr_block + 1, latest_relayed_block
            );

            let (proof_verified, new_mmr_root) = update_mmr_and_verify_onchain(
                &db_file,
                latest_mmr_block, latest_relayed_block, &starknet_rpc_url, &verifier_addr
            )
            .await?;

            info!("Proof verified: {:?}", proof_verified);
            info!("New MMR root: {:?}", new_mmr_root);

            // If SNARK proof is valid, update the latest stored blockhash and MMR root on L2
            if proof_verified {
                info!("Updating MMR state on Starknet...");
                let starknet_account = StarknetAccount::new(
                    Arc::clone(&starknet_provider.provider), // Clone the `Arc` to pass it
                    &dotenv::var("STARKNET_PRIVATE_KEY").expect("STARKNET_PRIVATE_KEY not set"),
                    &dotenv::var("STARKNET_ACCOUNT_ADDRESS")
                        .expect("STARKNET_ACCOUNT_ADDRESS not set"),
                );

                starknet_account
                    .update_mmr_state(
                        l2_store_addr,
                        latest_relayed_block,
                        Felt::from_hex(&new_mmr_root).unwrap(),
                    )
                    .await?;
                info!(
                    "MMR state updated on Starknet with latest relayed block number: {:?}, new MMR root: {:?}",
                    latest_relayed_block, new_mmr_root
                );
            }
        } else {
            info!("No new events found.");
        }

        // Wait for a specified interval before checking for new events again
        sleep(Duration::from_secs(60)).await; // Check every 60 seconds
    }
}
