#![deny(unused_crate_dependencies)]

mod client;

use clap::Parser;
use client::LightClient;
use common::initialize_logger_and_env;
use eyre::Result;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "5")]
    polling_interval: u64,

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

    tracing::info!("Starting Fossil Light Client...");

    let mut client = LightClient::new(args.polling_interval).await?;
    client.run().await?;
    Ok(())
}
