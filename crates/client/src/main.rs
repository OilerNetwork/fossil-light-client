//! Fossil Light Client - A Starknet blockchain event processor
//!
//! This binary provides a command-line interface for running the Fossil Light Client,
//! which monitors Starknet for blockchain events and maintains MMR (Merkle Mountain Range)
//! state synchronization between L1 and L2.
//!
//! The client connects to a Starknet RPC endpoint and continuously polls for new events
//! from the L2 store contract, processing them to update the MMR state and verify
//! proofs on-chain.

#![deny(unused_crate_dependencies)]

// Import unused dependencies to satisfy the linter
#[allow(unused_imports)]
use chrono as _;
#[allow(unused_imports)]
use derive_more as _;

#[cfg(test)]
mod test_imports {
    #[allow(unused_imports)]
    use hex as _;
    #[allow(unused_imports)]
    use mockall as _;
    #[allow(unused_imports)]
    use proptest as _;
    #[allow(unused_imports)]
    use tempfile as _;
    #[allow(unused_imports)]
    use tokio_test as _;
    #[allow(unused_imports)]
    use toml as _;
}

pub mod async_utils;
mod builder;
mod client;
mod config;
pub mod config_manager;
mod error;
mod events;
pub mod logging;
mod mmr;
pub mod types;

// Re-export public API
pub use builder::LightClientBuilder;
use clap::Parser;
pub use client::LightClient;
use common::initialize_logger;
pub use error::{ClientError, Result};

/// Command-line arguments for the Fossil Light Client.
///
/// These arguments control the behavior of the light client, including
/// polling intervals, batch processing settings, and environment configuration.
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

    /// Starknet monitoring start block (for event searching, defaults to latest - 1000)
    #[arg(long)]
    starknet_monitoring_start: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize environment with specified file
    dotenv::from_path(&args.env_file).map_err(|e| {
        error::ClientError::publisher_error(format!("Failed to load env file: {e}"))
    })?;
    initialize_logger().map_err(|e| {
        error::ClientError::publisher_error(format!("Failed to initialize logger: {e}"))
    })?;

    tracing::info!("Starting Fossil Light Client...");

    let mut client = if let Some(start_block) = args.start_block {
        LightClient::new(
            args.polling_interval,
            args.batch_size,
            start_block,
            args.blocks_per_run,
            args.starknet_monitoring_start,
        )
        .await?
    } else {
        LightClient::new_with_default_start(
            args.polling_interval,
            args.batch_size,
            args.blocks_per_run,
            args.starknet_monitoring_start,
        )
        .await?
    };

    client.run().await?;
    Ok(())
}
