//! # World Coin Block Header Verification Example
//!
//! This example demonstrates how to verify an Ethereum hash inclusion in Fossil MMR using the RISC0 zkVM
//! and extract verified block information.
//!
//! ## Overview
//!
//! This example showcases a complete workflow for verifying an Ethereum block by its hash:
//!
//! 1. **API Call (Outside Guest Program)**: The `get_block_hash_inclusion_proof` API endpoint
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
//! guest-mmr = { git = "https://github.com/OilerNetwork/fossil-light-client.git", package = "guest-mmr" }
//! ```
//!
//! Then import the necessary function in your code:
//!
//! ```rust
//! use publisher::get_block_hash_inclusion_proof;
//! ```
//!
//! This allows you to integrate Ethereum block hash verification with cryptographic
//! guarantees into your own applications.

use dotenv::dotenv;
use eyre::{eyre, Result};
use publisher::get_block_hash_inclusion_proof;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

const BATCH_SIZE: u64 = 1024;
const FOSSIL_STORE: &str = "0x01710d5f515a17943f439c0a5ba4483d44bac0d2b04f5345639c222debc80b2c";
const BLOCK_HASH: &str = "0x18d9e3002e3b190959a0bf0a97b16b282eaa3fd08c0a72e1b932d31e0255f1be";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with info level by default
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    // Load environment variables
    dotenv().ok();

    // Configuration from environment variables
    let rpc_url = std::env::var("STARKNET_RPC_URL")
        .map_err(|_| eyre!("STARKNET_RPC_URL environment variable not set"))?;

    info!("Starting block hash verification example");
    info!("Verifying block with hash: {}", BLOCK_HASH);

    // Get the mmr state and proof outside of the zkVM Guest Program
    let proof_response = get_block_hash_inclusion_proof(
        BLOCK_HASH.to_string(),
        rpc_url.clone(),
        FOSSIL_STORE.to_string(),
        BATCH_SIZE,
    )
    .await?;

    // Pass BlockHashProofResponse to the zkVM Guest Program

    // Verify the proof inside the zkVM Guest Program
    let proof_verified = proof_response.guest_mmr.verify_proof(
        proof_response.proof.clone(),
        BLOCK_HASH.to_string(),
        None,
    )?;

    info!("Proof verified: {}", proof_verified);
    info!("Batch index: {}", proof_response.batch_index);
    info!("Guest MMR: {:?}", proof_response.guest_mmr);
    info!("Proof: {:?}", proof_response.proof);

    Ok(())
}
