use clap::Parser;
use common::{get_env_var, initialize_logger_and_env};
use publisher::extract_fees;
use tokio;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start block number
    #[arg(long, short)]
    start_block: u64,

    /// End block number
    #[arg(long, short)]
    end_block: u64,

    /// Batch size
    #[arg(long, short, default_value_t = 1024)]
    batch_size: u64,

    /// Skip proof generation
    #[arg(long)]
    skip_proof: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_logger_and_env()?;
    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let l2_store_address = get_env_var("FOSSIL_STORE")?;
    let chain_id = get_env_var("CHAIN_ID")?.parse::<u64>()?;

    let args = Args::parse();

    // Verify blocks
    match extract_fees(
        &rpc_url,
        &l2_store_address,
        chain_id,
        args.batch_size,
        args.start_block,
        args.end_block,
        Some(args.skip_proof),
    )
    .await
    {
        Ok(result) => {
            for proof in result {
                proof.receipt().verify(proof.image_id()?)?;
                let result = proof.journal().decode::<bool>()?;
                tracing::info!("result: {}", result);
            }
        }
        Err(e) => {
            tracing::error!("Error during verification: {:?}", e);
        }
    }

    tracing::info!("All blocks are valid!");

    Ok(())
}
