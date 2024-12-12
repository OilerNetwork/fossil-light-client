use clap::Parser;
use common::{get_env_var, initialize_logger_and_env};
use tracing::info;

const BATCH_SIZE: u64 = 1024;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Start block
    #[arg(short = 's', long)]
    start: u64,

    /// End block
    #[arg(short = 'e', long)]
    end: u64,

    /// Skip proof verification
    #[arg(short = 'p', long, default_value_t = false)]
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

    info!("Starting Publisher...");

    // Parse CLI arguments
    let args = Args::parse();

    publisher::prove_mmr_update(
        &rpc_url,
        chain_id,
        &verifier_address,
        &store_address,
        &private_key,
        &account_address,
        BATCH_SIZE,
        args.start,
        args.end,
        args.skip_proof,
    )
    .await?;

    info!("MMR building completed");
    info!("Host finished");

    Ok(())
}
