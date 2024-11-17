use clap::Parser;
use common::{get_env_var, initialize_logger_and_env};
use eyre::{eyre, Result};
use host::{get_store_path, update_mmr_and_verify_onchain};

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
    #[arg(short, long)]
    rpc_url: String,

    /// Verifier address
    #[arg(short, long)]
    verifier: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logger_and_env()?;

    let args = Args::parse();

    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let verifier = get_env_var("STARKNET_VERIFIER")?;

    let store_path = get_store_path(args.db_file).map_err(|e| eyre!(e))?;

    let (proof_verified, new_mmr_root) =
        update_mmr_and_verify_onchain(&store_path, args.start, args.end, &rpc_url, &verifier)
            .await?;

    println!("Proof verified: {:?}", proof_verified);
    println!("New MMR root: {:?}", new_mmr_root);

    Ok(())
}
