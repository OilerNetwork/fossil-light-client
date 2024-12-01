use clap::Parser;
use common::{get_env_var, initialize_logger_and_env};
use eyre::Result;
use methods::{MMR_APPEND_ELF, MMR_APPEND_ID};
use publisher::{AccumulatorBuilder, ProofGenerator};
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
async fn main() -> Result<()> {
    initialize_logger_and_env()?;

    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let verifier_address = get_env_var("FOSSIL_VERIFIER")?;
    let private_key = get_env_var("STARKNET_PRIVATE_KEY")?;
    let account_address = get_env_var("STARKNET_ACCOUNT_ADDRESS")?;

    info!("Starting Publisher...");

    // Parse CLI arguments
    let args = Args::parse();

    info!("Initializing proof generator...");
    // Initialize proof generator
    let proof_generator = ProofGenerator::new(MMR_APPEND_ELF, MMR_APPEND_ID, args.skip_proof);

    info!("Initializing accumulator builder...");
    // Initialize accumulator builder with the batch size
    let mut builder = AccumulatorBuilder::new(
        &rpc_url,
        &verifier_address,
        &private_key,
        &account_address,
        proof_generator,
        args.batch_size,
        args.skip_proof,
    )
    .await?;

    info!("Building MMR...");
    // Build MMR from finalized block to block #0 or up to the specified number of batches
    if let Some(num_batches) = args.num_batches {
        builder.build_with_num_batches(num_batches).await?;
    } else {
        builder.build_from_finalized().await?;
    }

    info!("MMR building completed");
    info!("Host finished");

    Ok(())
}
