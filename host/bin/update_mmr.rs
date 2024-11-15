use std::env;

use clap::Parser;
use dotenv::dotenv;
use eyre::Result;
use host::{get_store_path, update_mmr_and_verify_onchain}; // Import the function from your library
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the SQLite database file.
    #[arg(short, long)]
    db_file: Option<String>,

    /// Start block
    #[arg(short, long)]
    start: u64,

    /// End block
    #[arg(short, long)]
    end: u64,

    /// RPC URL
    #[arg(short, long, default_value_t = String::from(env::var("STARKNET_RPC_URL").expect("STARKNET_RPC_URL must be set")))]
    rpc_url: String,

    /// Verifier address
    #[arg(short, long)]
    verifier: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let store_path = get_store_path(args.db_file).expect("Failed to get store path");

    let (proof_verified, new_mmr_root) = update_mmr_and_verify_onchain(
        &store_path,
        args.start,
        args.end,
        &args.rpc_url,
        &args.verifier,
    )
    .await?;

    println!("Proof verified: {:?}", proof_verified);
    println!("New MMR root: {:?}", new_mmr_root);

    Ok(())
}
