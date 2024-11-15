use alloy::{
    eips::BlockNumberOrTag,
    network::primitives::BlockTransactionsKind,
    providers::{Provider as EthereumProvider, ProviderBuilder},
};
use dotenv::dotenv;
use eyre::Result;
use mmr_accumulator::processor_utils::{create_database_file, ensure_directory_exists};
use host::update_mmr_and_verify_onchain;
use starknet::{
    core::types::{BlockId, BlockTag, EventFilter, Felt, FunctionCall},
    macros::selector,
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider as StarknetProvider, Url,
    },
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Fossil Light Client...");

    let starknet_rpc_url = dotenv::var("KATANA_RPC_URL").expect("KATANA_RPC_URL not set");

    let l2_store_addr =
        Felt::from_hex(&dotenv::var("FOSSIL_STORE").expect("FOSSIL_STORE not set")).unwrap();

    let verifier_addr = dotenv::var("STARKNET_VERIFIER").expect("STARKNET_VERIFIER not set");

    let starknet_provider =
        JsonRpcClient::new(HttpTransport::new(Url::parse(&starknet_rpc_url).unwrap()));

    // Poll for events from the latest stored blockhash to the latest block
    let event_filter = EventFilter {
        from_block: Some(BlockId::Number(0)),
        to_block: Some(BlockId::Tag(BlockTag::Latest)),
        address: Some(l2_store_addr),
        keys: Some(vec![vec![selector!("LatestBlockhashFromL1Stored")]]),
    };

    let events = starknet_provider.get_events(event_filter, None, 1).await?;

    info!("Fetched {} events", events.events.len());

    // Fetch the latest stored blockhash from L1
    let latest_updated_block = starknet_provider
        .call(
            FunctionCall {
                contract_address: l2_store_addr,
                entry_point_selector: selector!("get_latest_blockhash_from_l1"),
                calldata: vec![],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .expect("failed to call contract");

    info!(
        "Latest updated block on Starknet: {:?}",
        latest_updated_block
    );

    let ethereum_provider_url = dotenv::var("ANVIL_URL")
        .expect("ANVIL_RPC_URL not set")
        .parse()?;
    let ethereum_provider = ProviderBuilder::new().on_http(ethereum_provider_url);

    // if new events are found:
    // 1. fetch the Ethereum latest finalized block number
    let latest_finalized_block = ethereum_provider
        .get_block_by_number(BlockNumberOrTag::Finalized, BlockTransactionsKind::Hashes)
        .await
        .expect("failed to get latest finalized block");
    let latest_finalized_block_number: u64 = latest_finalized_block
        .expect("failed to get latest finalized block")
        .header
        .inner
        .number;
    info!(
        "Latest Ethereum finalized block: {:?}",
        latest_finalized_block_number
    );

    // Convert the block number from the call response
    let from_block = u64::from_str_radix(
        latest_updated_block[0]
            .to_hex_string()
            .trim_start_matches("0x"),
        16, // Base 16 for hexadecimal
    )
    .expect("Failed to convert hex string to u64");

    // call risc0 prover to verify the blockheaders, append to MMR and verify SNARK proof.
    // Set up the database file path
    let current_dir = ensure_directory_exists("db-store")?;
    let db_file = create_database_file(&current_dir, 0)?;

    info!(
        "Calling Risc0, proving blockheaders from {:?} to {:?}",
        from_block, latest_finalized_block_number
    );
    let (proof_verified, new_mmr_root) = update_mmr_and_verify_onchain(
        &db_file,
        from_block,
        latest_finalized_block_number,
        &starknet_rpc_url,
        &verifier_addr,
    )
    .await?;
    info!("Proof verified: {:?}", proof_verified);
    info!("New MMR root: {:?}", new_mmr_root);
    // if SNARK proof is valid, update the latest stored blockhash and MMR root on L2

    // repeat having a way to check if new events are found

    Ok(())
}
