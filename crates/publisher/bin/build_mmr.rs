use clap::Parser;
use common::{get_env_var, initialize_logger_and_env};
use publisher::core::AccumulatorBuilder;
use starknet_handler::{account::StarknetAccount, provider::StarknetProvider};
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Batch size for processing blocks
    #[arg(short, long, default_value_t = 1024)]
    batch_size: u64,

    /// Number of batches to process. If not specified, processes until block #0.
    #[arg(short, long)]
    num_batches: Option<u64>,

    /// Skip proof verification
    #[arg(short, long, default_value_t = false)]
    skip_proof: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_logger_and_env()?;

    let chain_id = get_env_var("CHAIN_ID")?.parse::<u64>()?;
    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let verifier_address = get_env_var("FOSSIL_VERIFIER")?;
    let store_address = get_env_var("FOSSIL_STORE")?;
    let private_key = get_env_var("STARKNET_PRIVATE_KEY")?;
    let account_address = get_env_var("STARKNET_ACCOUNT_ADDRESS")?;

    // Parse CLI arguments
    let args = Args::parse();

    let starknet_provider = StarknetProvider::new(&rpc_url)?;
    let starknet_account =
        StarknetAccount::new(starknet_provider.provider(), &private_key, &account_address)?;

    let mut builder = AccumulatorBuilder::new(
        chain_id,
        &verifier_address,
        &store_address,
        starknet_account,
        args.batch_size,
        args.skip_proof,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create AccumulatorBuilder");
        e
    })?;

    // Build MMR from finalized block to block #0 or up to the specified number of batches
    if let Some(num_batches) = args.num_batches {
        builder.build_with_num_batches(num_batches).await?;
    } else {
        builder.build_from_finalized().await?;
    }

    info!("Host finished");

    Ok(())
}
