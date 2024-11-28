use clap::Parser;
use common::{get_env_var, initialize_logger_and_env};
use eyre::Result;
use host::{db_access::get_store_path, AccumulatorBuilder, ProofGenerator, ProofType};
use methods::{MMR_GUEST_ELF, MMR_GUEST_ID};
use starknet_handler::{account::StarknetAccount, provider::StarknetProvider};
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Batch size for processing blocks
    #[arg(short, long, default_value_t = 1024)]
    batch_size: u64,

    /// Path to the SQLite database file. If not specified, a new one will be created.
    #[arg(short, long)]
    db_file: Option<String>,

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

    let store_path = get_store_path(args.db_file)?;

    info!("Initializing proof generator...");
    // Initialize proof generator
    let proof_generator = ProofGenerator::new(MMR_GUEST_ELF, MMR_GUEST_ID, args.skip_proof);

    info!("Initializing accumulator builder...");
    // Initialize accumulator builder with the batch size
    let mut builder = AccumulatorBuilder::new(
        &store_path,
        proof_generator,
        args.batch_size,
        args.skip_proof,
    )
    .await?;

    info!("Building MMR...");
    // Build MMR from finalized block to block #0 or up to the specified number of batches
    let results = if let Some(num_batches) = args.num_batches {
        builder.build_with_num_batches(num_batches).await?
    } else {
        builder.build_from_finalized().await?
    };

    info!("Processing results...");
    // Print results
    for result in &results {
        info!(
            "Processed blocks {} to {}",
            result.start_block(),
            result.end_block()
        );

        let new_mmr_state = result.new_mmr_state();

        match result.proof() {
            Some(ProofType::Stark { .. }) => info!("Generated STARK proof"),
            Some(ProofType::Groth16 { calldata, .. }) => {
                info!("Generated Groth16 proof");
                let provider = StarknetProvider::new(&rpc_url)?;
                let account =
                    StarknetAccount::new(provider.provider(), &private_key, &account_address)?;
                let (tx_hash, new_mmr_state) = account
                    .verify_mmr_proof(&verifier_address, &new_mmr_state, calldata)
                    .await?;
                info!("Final proof verified on Starknet, tx hash: {:?}", tx_hash);
                info!("New MMR state: {:?}", new_mmr_state);
            }
            None => info!("No proof generated"),
        }
    }
    info!("Host finished");

    Ok(())
}
