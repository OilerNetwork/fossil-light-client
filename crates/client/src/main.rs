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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize environment with specified file
    dotenv::from_path(&args.env_file)?;
    initialize_logger_and_env()?;

    tracing::info!("Starting Fossil Light Client...");

    let mut client = LightClient::new(args.polling_interval, args.batch_size).await?;
    client.run().await?;
    Ok(())
}
