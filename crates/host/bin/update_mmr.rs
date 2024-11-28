use clap::Parser;
use common::get_env_var;
use eyre::{eyre, Result};
use host::{db_access::get_store_path, update_mmr_and_verify_onchain};

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let private_key = get_env_var("STARKNET_PRIVATE_KEY")?;
    let account_address = get_env_var("STARKNET_ACCOUNT_ADDRESS")?;
    let verifier = get_env_var("FOSSIL_VERIFIER")?;

    let store_path = get_store_path(args.db_file)?;

    let (new_mmr_state, proof) = prove_mmr_update(&store_path, args.start, args.end).await?;

    let provider = StarknetProvider::new(&rpc_url)?;

    let account = StarknetAccount::new(provider.provider(), &private_key, &account_address)?;

    account
        .verify_mmr_proof(&verifier, &new_mmr_state, proof)
        .await?;

    Ok(())
}
