// src/main.rs

mod relayer;

use crate::relayer::Relayer;
use common::initialize_logger_and_env;
use eyre::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment and logging
    initialize_logger_and_env()?;

    info!("Starting the relayer...");

    let relayer = Relayer::new().await?;
    relayer.send_finalized_block_hash_to_l2().await?;

    info!("Relayer finished successfully");

    Ok(())
}
