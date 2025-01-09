use clap::Parser;
use common::{get_env_var, initialize_logger_and_env};
use publisher::core::AccumulatorBuilder;
use starknet_handler::{account::StarknetAccount, provider::StarknetProvider};

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
    #[arg(short = 'p', long, default_value_t = false)]
    skip_proof: bool,

    /// Path to environment file (optional)
    #[arg(short = 'e', long, default_value = ".env")]
    env_file: String,

    /// Start building from this block number. If not specified, starts from the latest finalized block.
    #[arg(short = 's', long)]
    start_block: Option<u64>,

    /// Start building from the latest MMR block
    #[arg(short = 'l', long, default_value_t = false)]
    from_latest: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments first
    let args = Args::parse();

    // Initialize environment with specified file
    dotenv::from_path(&args.env_file)?;
    initialize_logger_and_env()?;

    let chain_id = get_env_var("CHAIN_ID")?.parse::<u64>()?;
    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let verifier_address = get_env_var("FOSSIL_VERIFIER")?;
    let store_address = get_env_var("FOSSIL_STORE")?;
    let private_key = get_env_var("STARKNET_PRIVATE_KEY")?;
    let account_address = get_env_var("STARKNET_ACCOUNT_ADDRESS")?;

    let starknet_provider = StarknetProvider::new(&rpc_url)?;
    let starknet_account =
        StarknetAccount::new(starknet_provider.provider(), &private_key, &account_address)?;

    let mut builder = AccumulatorBuilder::new(
        &rpc_url,
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

    // Build MMR from specified start block or finalized block
    match (args.from_latest, args.start_block, args.num_batches) {
        (true, Some(_), _) => {
            return Err("Cannot specify both --from-latest and --start-block".into());
        }
        (true, None, Some(num_batches)) => {
            builder.build_from_latest_with_batches(num_batches).await?
        }
        (true, None, None) => builder.build_from_latest().await?,
        (false, Some(start_block), Some(num_batches)) => {
            builder
                .build_from_block_with_batches(start_block, num_batches)
                .await?
        }
        (false, Some(start_block), None) => builder.build_from_block(start_block).await?,
        (false, None, Some(num_batches)) => builder.build_with_num_batches(num_batches).await?,
        (false, None, None) => builder.build_from_finalized().await?,
    }

    Ok(())
}
