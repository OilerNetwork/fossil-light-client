#![deny(unused_crate_dependencies)]

mod client;

use clap::Parser;
use client::LightClient;
use common::initialize_logger;
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

    /// Starting block number for indexing (defaults to latest relayed block + 1)
    #[arg(short = 's', long)]
    start_block: Option<u64>,

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
    initialize_logger()?;

    tracing::info!("Starting Fossil Light Client...");

    let mut client = if let Some(start_block) = args.start_block {
        LightClient::new(
            args.polling_interval,
            args.batch_size,
            start_block,
            args.blocks_per_run,
        )
        .await?
    } else {
        LightClient::new_with_default_start(
            args.polling_interval,
            args.batch_size,
            args.blocks_per_run,
        )
        .await?
    };

    client.run().await?;
    Ok(())
}
