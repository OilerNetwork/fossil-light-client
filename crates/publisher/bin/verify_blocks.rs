use clap::Parser;
use common::{get_env_var, initialize_logger_and_env};
use publisher::{db::DbConnection, prove_headers_integrity_and_inclusion};
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

    /// Skip proof generation
    #[arg(long)]
    skip_proof: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_logger_and_env()?;
    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let l2_store_address = get_env_var("FOSSIL_STORE")?;

    let args = Args::parse();

    // Fetch block headers
    let db_connection = DbConnection::new().await?;
    let headers = db_connection
        .get_block_headers_by_block_range(args.start_block, args.end_block)
        .await?;

    // Verify blocks
    match prove_headers_integrity_and_inclusion(
        &rpc_url,
        &l2_store_address,
        &headers,
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
