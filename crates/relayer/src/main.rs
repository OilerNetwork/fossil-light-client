#![deny(unused_crate_dependencies)]

mod relayer;

use crate::relayer::Relayer;
use clap::Parser;
use common::initialize_logger;
use eyre::Result;
use std::time::Duration;
use tokio::time;
use tracing::info;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to environment file (optional)
    #[arg(short = 'e', long, default_value = ".env")]
    env_file: String,

    /// Relay interval in minutes (0 for single run)
    #[arg(short = 't', long, default_value = "0")]
    relay_time_minutes: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize environment with specified file
    dotenv::from_path(&args.env_file)?;
    initialize_logger()?;

    info!("Starting the relayer...");

    // Create the relayer instance
    let relayer = Relayer::new().await?;

    if args.relay_time_minutes == 0 {
        // Single run mode
        info!("Running in single execution mode");
        relayer.send_finalized_block_hash_to_l2().await?;
        info!("Relayer finished successfully");
    } else {
        // Continuous mode with specified interval
        info!(
            "Running in continuous mode with {} minute interval",
            args.relay_time_minutes
        );

        let interval = Duration::from_secs(args.relay_time_minutes * 60);

        loop {
            let start_time = std::time::Instant::now();

            info!("Sending finalized block hash to L2...");
            match relayer.send_finalized_block_hash_to_l2().await {
                Ok(_) => info!("Successfully relayed block hash to L2"),
                Err(e) => info!("Failed to relay block hash: {}", e),
            }

            let elapsed = start_time.elapsed();
            if elapsed < interval {
                let sleep_time = interval - elapsed;
                info!(
                    "Waiting for {} minutes before next relay...",
                    sleep_time.as_secs() / 60
                );
                time::sleep(sleep_time).await;
            }
        }
    }

    Ok(())
}
