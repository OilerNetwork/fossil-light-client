#![deny(unused_crate_dependencies)]

mod relayer;

use crate::relayer::Relayer;
use clap::Parser;
use common::initialize_logger_and_env;
use eyre::Result;
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the environment file
    #[arg(short, long)]
    env_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment and logging
    let args = Args::parse();
    let _guard = initialize_logger_and_env(args.env_file.as_deref())?;

    info!("Starting the relayer...");

    let relayer = Relayer::new().await?;
    relayer.send_finalized_block_hash_to_l2().await?;

    info!("Relayer finished successfully");

    Ok(())
}
