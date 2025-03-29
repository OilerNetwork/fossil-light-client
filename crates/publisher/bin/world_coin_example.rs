//! # World Coin Block Header Verification Example
//!
//! This example demonstrates how to verify an Ethereum block by its hash using the RISC0 zkVM
//! and extract verified block information.
//!
//! ## Overview
//!
//! This example showcases a complete workflow for verifying an Ethereum block by its hash:
//!
//! 1. **API Call (Outside Guest Program)**: The `get_block_hash_proof_serializable` API endpoint
//!    is called from outside the guest program. This function:
//!    - Accepts a block hash as input
//!    - Fetches the MMR state from onchain
//!    - Downloads the MMR database from IPFS
//!    - Verifies the MMR state matches the onchain state
//!    - Generates a Merkle proof for the block hash
//!    - Returns a proof and batch index
//!
//! 2. **Client Usage (In Application)**: The returned proof can be:
//!    - Used to verify the block's inclusion in the MMR
//!    - Passed to Smart Contracts or other verification systems
//!    - Used to establish trust in the block data
//!
//! 3. **Verification**: The proof verifies that:
//!    - The block with the given hash exists in the MMR
//!    - The MMR root matches the one stored onchain
//!    - The block is part of the canonical chain
//!
//! ## Security Considerations
//!
//! This workflow ensures:
//! - The block's existence is cryptographically verified against the MMR
//! - The MMR state is verified against the onchain state
//! - The proof can be used for trustless verification
//!
//! ## Usage
//!
//! This example is intended to demonstrate the pattern. In a real application:
//! - The API would be called by an external service
//! - The proof would be passed to a verification system
//! - The verification system would verify the block's inclusion
//!
//! ## Importing as a Dependency
//!
//! To use this functionality in your project, add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! publisher = { git = "https://github.com/OilerNetwork/fossil-light-client.git", package = "publisher" }
//! guest-types = { git = "https://github.com/OilerNetwork/fossil-light-client.git", package = "guest-types" }
//! methods = { git = "https://github.com/OilerNetwork/fossil-light-client.git", package = "methods" }
//! ```
//!
//! This allows you to integrate Ethereum block hash verification with cryptographic
//! guarantees into your own applications.

use dotenv::dotenv;
use eyre::{eyre, Result};
use publisher::api::operations::get_block_hash_proof_serializable;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with info level by default
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    // Load environment variables
    dotenv().ok();

    // The block hash to verify
    let block_hash =
        "0x18d9e3002e3b190959a0bf0a97b16b282eaa3fd08c0a72e1b932d31e0255f1be".to_string();

    // Configuration from environment variables
    let rpc_url = std::env::var("STARKNET_RPC_URL")
        .map_err(|_| eyre!("STARKNET_RPC_URL environment variable not set"))?;
    let store_address = std::env::var("STARKNET_STORE_ADDRESS")
        .map_err(|_| eyre!("STARKNET_STORE_ADDRESS environment variable not set"))?;
    let batch_size = std::env::var("BATCH_SIZE")
        .map_err(|_| eyre!("BATCH_SIZE environment variable not set"))?
        .parse::<u64>()
        .map_err(|_| eyre!("BATCH_SIZE must be a valid integer"))?;

    info!("Starting block hash verification example");
    info!("Verifying block with hash: {}", block_hash);

    // Get the proof for the block hash
    match get_block_hash_proof_serializable(block_hash.clone(), rpc_url, store_address, batch_size)
        .await
    {
        Ok(proof_response) => {
            // Successfully retrieved the proof
            info!("Block verification successful!");
            info!("Batch Index: {}", proof_response.batch_index);

            let proof = proof_response.proof;
            info!("Element Index: {}", proof.element_index);
            info!("Element Hash: {}", proof.element_hash);
            info!("Number of Siblings: {}", proof.siblings_hashes.len());
            info!("Number of Peaks: {}", proof.peaks_hashes.len());

            info!("This proof can be used to verify the block's inclusion in the MMR");

            // Example: Verify the proof (this would typically be done in another system)
            info!("To verify this proof, you would typically:");
            info!("1. Use the proof to verify against the MMR root");
            info!("2. Check that the MMR root matches the one stored onchain");
            info!("3. Confirm the element hash matches the expected block hash");
        }
        Err(e) => {
            error!("Failed to verify block hash: {}", e);
            return Err(eyre!("Block verification failed: {}", e));
        }
    }

    Ok(())
}
