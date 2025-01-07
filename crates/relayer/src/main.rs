#![deny(unused_crate_dependencies)]

mod relayer;

use crate::relayer::Relayer;
use clap::Parser;
use common::initialize_logger_and_env;
use eyre::Result;
use tracing::info;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to environment file (optional)
    #[arg(short = 'e', long, default_value = ".env")]
    env_file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize environment with specified file
    dotenv::from_path(&args.env_file)?;
    initialize_logger_and_env()?;

    info!("Starting the relayer...");

    let relayer = Relayer::new().await?;
    relayer.send_finalized_block_hash_to_l2().await?;

    info!("Relayer finished successfully");

    Ok(())
}
