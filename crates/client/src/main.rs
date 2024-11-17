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
}

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logger_and_env()?;
    let args = Args::parse();

    tracing::info!("Starting Fossil Light Client...");

    let mut client = LightClient::new(args.polling_interval).await?;
    client.run().await
}
