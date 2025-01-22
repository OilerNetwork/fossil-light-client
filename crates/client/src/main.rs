#![deny(unused_crate_dependencies)]

mod client;

use clap::Parser;
use client::LightClient;
use common::initialize_logger_and_env;
use eyre::Result;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "5")]
    polling_interval: u64,

    /// Path to environment file (optional)
    #[arg(short = 'e', long, default_value = ".env")]
    env_file: String,

    /// Number of blocks to process in each batch
    #[arg(short, long, default_value = "1024")]
    batch_size: u64,

    /// Starting block number for indexing
    #[arg(short = 's', long, default_value = "0")]
    start_block: u64,

    /// Maximum number of blocks to process in each loop run (0 for unlimited)
    #[arg(short = 'n', long, default_value = "100")]
    blocks_per_run: u64,

    /// Blocks buffer size
    #[arg(long, default_value = "50")]
    blocks_buffer_size: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize environment with specified file
    dotenv::from_path(&args.env_file)?;
    initialize_logger_and_env()?;

    tracing::info!("Starting Fossil Light Client...");

    let mut client = LightClient::new(
        args.polling_interval,
        args.batch_size,
        args.start_block,
        args.blocks_per_run,
        args.blocks_buffer_size,
    )
    .await?;
    client.run().await?;
    Ok(())
}
