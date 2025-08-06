//! # API Operations
//!
//! This module provides the public API functions for the Publisher crate.
//! These functions maintain backward compatibility while leveraging the
//! improved internal architecture.
//!
//! ## Functions
//!
//! - [`prove_mmr_update`] - Generate and submit MMR update proofs to Starknet
//! - [`update_mmr`] - Update MMR state without generating proofs
//! - [`get_single_block_hash_proof`] - Get Merkle proof for a specific block hash
//! - [`get_block_hash_inclusion_proof`] - Get serializable proof structure for a block hash
//!
//! ## Examples
//!
//! ### Generating MMR Update Proof
//!
//! ```rust,no_run
//! use publisher::prove_mmr_update;
//! use starknet_handler::provider::LatestRelayBlock;
//!
//! # async fn example() -> Result<(), eyre::Error> {
//! let rpc_url = "http://localhost:8545".to_string();
//! let chain_id = 1;
//! let verifier_address = "0x1234567890123456789012345678901234567890".to_string();
//! let store_address = "0x0987654321098765432109876543210987654321".to_string();
//! let private_key = "0xprivate_key".to_string();
//! let address = "0xaddress".to_string();
//! let batch_size = 100;
//! let start_block = 1;
//! let latest_relay_block = LatestRelayBlock {
//! block_number: 100,
//! block_hash: "0xlatest_hash".to_string(),
//! };
//!
//! prove_mmr_update(
//! &rpc_url,
//! chain_id,
//! &verifier_address,
//! &store_address,
//! &private_key,
//! &address,
//! batch_size,
//! start_block,
//! latest_relay_block,
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Getting Block Hash Proof
//!
//! ```rust,no_run
//! use publisher::get_block_hash_inclusion_proof;
//!
//! # async fn example() -> Result<(), eyre::Error> {
//! let block_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string();
//! let rpc_url = "http://localhost:8545".to_string();
//! let store_address = "0x0987654321098765432109876543210987654321".to_string();
//! let batch_size = 100;
//!
//! let proof = get_block_hash_inclusion_proof(
//! block_hash,
//! rpc_url,
//! store_address,
//! batch_size,
//! ).await?;
//!
//! println!("Proof for batch {}: {:?}", proof.batch_index, proof.proof);
//! # Ok(())
//! # }
//! ```

use eyre::Result;
use guest_mmr::core::GuestMMR;
use guest_types::GuestMMRProof;
use mmr;
use serde::{Deserialize, Serialize};
use starknet_handler::provider::LatestRelayBlock;

use crate::{config::PublisherConfig, service::ProofService};

/// Serializable proof structure for API responses
///
/// This structure contains all the information needed to verify
/// a block hash inclusion in the MMR.
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockHashProofResponse {
    /// The batch index where this block hash is included
    pub batch_index: u64,
    /// The MMR state at the time of this batch
    pub guest_mmr: GuestMMR,
    /// The inclusion proof for the block hash
    pub proof: GuestMMRProof,
}

impl From<(GuestMMR, mmr::Proof, u64)> for BlockHashProofResponse {
    fn from(proof_data: (GuestMMR, mmr::Proof, u64)) -> Self {
        let (guest_mmr, proof, batch_index) = proof_data;
        Self {
            guest_mmr,
            proof: GuestMMRProof {
                element_index: proof.element_index,
                element_hash: proof.element_hash,
                siblings_hashes: proof.siblings_hashes,
                peaks_hashes: proof.peaks_hashes,
                elements_count: proof.elements_count,
            },
            batch_index,
        }
    }
}

/// Generate and submit MMR update proof to Starknet
///
/// This function generates a cryptographic proof for updating the MMR state
/// and submits it to the Starknet verifier contract.
///
/// # Parameters
///
/// * `rpc_url` - RPC endpoint URL for the blockchain network
/// * `chain_id` - Chain ID of the target network
/// * `verifier_address` - Address of the verifier contract on Starknet
/// * `store_address` - Address of the store contract on Starknet
/// * `account_private_key` - Private key for the Starknet account
/// * `account_address` - Address of the Starknet account
/// * `batch_size` - Number of blocks to process in each batch
/// * `start_block` - Starting block number for the update
/// * `latest_relayed_block_and_hash` - Latest block information from the relay
///
/// # Returns
///
/// Returns `Ok(())` if the proof generation and submission succeeds,
/// or an error if any step fails.
///
/// # Examples
///
/// ```rust,no_run
/// use publisher::prove_mmr_update;
/// use starknet_handler::provider::LatestRelayBlock;
///
/// # async fn example() -> Result<(), eyre::Error> {
/// let latest_relay_block = LatestRelayBlock {
///     block_number: 100,
///     block_hash: "0xlatest_hash".to_string(),
/// };
///
/// prove_mmr_update(
///     &"http://localhost:8545".to_string(),
///     1,
///     &"0x1234567890123456789012345678901234567890".to_string(),
///     &"0x0987654321098765432109876543210987654321".to_string(),
///     &"0xprivate_key".to_string(),
///     &"0xaddress".to_string(),
///     100,
///     1,
///     latest_relay_block,
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn prove_mmr_update(
    rpc_url: &String,
    chain_id: u64,
    verifier_address: &String,
    store_address: &String,
    account_private_key: &String,
    account_address: &String,
    batch_size: u64,
    start_block: u64,
    latest_relayed_block_and_hash: LatestRelayBlock,
) -> Result<()> {
    let service = ProofService::new(
        rpc_url.clone(),
        chain_id,
        verifier_address.clone(),
        store_address.clone(),
    );

    service
        .prove_mmr_update(
            account_private_key,
            account_address,
            batch_size,
            start_block,
            latest_relayed_block_and_hash,
        )
        .await
        .map_err(|e| e.into_eyre())
}

/// Update MMR state without generating proofs
///
/// This function updates the MMR state with new block data but does not
/// generate or submit proofs to Starknet. Useful for testing or when
/// proof generation is not required.
///
/// # Parameters
///
/// * `rpc_url` - RPC endpoint URL for the blockchain network
/// * `chain_id` - Chain ID of the target network  
/// * `verifier_address` - Address of the verifier contract on Starknet
/// * `store_address` - Address of the store contract on Starknet
/// * `account_private_key` - Private key for the Starknet account
/// * `account_address` - Address of the Starknet account
/// * `batch_size` - Number of blocks to process in each batch
/// * `start_block` - Starting block number for the update
/// * `latest_relayed_block_and_hash` - Latest block information from the relay
///
/// # Returns
///
/// Returns `Ok(())` if the MMR update succeeds, or an error if any step fails.
///
/// # Examples
///
/// ```rust,no_run
/// use publisher::update_mmr;
/// use starknet_handler::provider::LatestRelayBlock;
///
/// # async fn example() -> Result<(), eyre::Error> {
/// let latest_relay_block = LatestRelayBlock {
///     block_number: 100,
///     block_hash: "0xlatest_hash".to_string(),
/// };
///
/// update_mmr(
///     &"http://localhost:8545".to_string(),
///     1,
///     &"0x1234567890123456789012345678901234567890".to_string(),
///     &"0x0987654321098765432109876543210987654321".to_string(),
///     &"0xprivate_key".to_string(),
///     &"0xaddress".to_string(),
///     100,
///     1,
///     latest_relay_block,
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn update_mmr(
    rpc_url: &String,
    chain_id: u64,
    verifier_address: &String,
    store_address: &String,
    account_private_key: &String,
    account_address: &String,
    batch_size: u64,
    start_block: u64,
    latest_relayed_block_and_hash: LatestRelayBlock,
) -> Result<()> {
    let service = ProofService::new(
        rpc_url.clone(),
        chain_id,
        verifier_address.clone(),
        store_address.clone(),
    );

    service
        .update_mmr(
            account_private_key,
            account_address,
            batch_size,
            start_block,
            latest_relayed_block_and_hash,
        )
        .await
        .map_err(|e| e.into_eyre())
}

/// Get Merkle proof for a single block hash
///
/// This function retrieves and verifies a Merkle proof for a specific block hash
/// by:
/// 1. Fetching the MMR state from onchain for the batch containing the block hash
/// 2. Downloading the MMR database file from IPFS
/// 3. Verifying that the downloaded file matches the onchain MMR root and leaves count
/// 4. Generating a Merkle proof for the block hash
/// 5. Returning the proof along with the batch number and MMR state
///
/// # Parameters
///
/// * `block_hash` - The block hash to generate a proof for (hex-encoded)
/// * `rpc_url` - RPC endpoint URL for the blockchain network
/// * `store_address` - Address of the store contract on Starknet
/// * `batch_size` - Number of blocks processed in each batch
///
/// # Returns
///
/// Returns a tuple containing:
/// - `u64` - The batch index where the block hash is located
/// - `GuestMMR` - The MMR state at the time of the batch
/// - `mmr::Proof` - The Merkle proof for the block hash
///
/// # Errors
///
/// Returns an error if:
/// - The block hash is not found in the database
/// - The IPFS download fails
/// - The MMR state verification fails
/// - The proof generation fails
///
/// # Examples
///
/// ```rust,no_run
/// use publisher::get_single_block_hash_proof;
///
/// # async fn example() -> Result<(), eyre::Error> {
/// let (batch_index, guest_mmr, proof) = get_single_block_hash_proof(
///     "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
///     "http://localhost:8545".to_string(),
///     "0x0987654321098765432109876543210987654321".to_string(),
///     100,
/// ).await?;
///
/// println!("Proof generated for batch {}", batch_index);
/// # Ok(())
/// # }
/// ```
pub async fn get_single_block_hash_proof(
    block_hash: String,
    rpc_url: String,
    store_address: String,
    batch_size: u64,
) -> Result<(u64, GuestMMR, mmr::Proof)> {
    let config = PublisherConfig {
        rpc_url,
        chain_id: 0,                     // chain_id not needed for this operation
        verifier_address: String::new(), // verifier_address not needed for this operation
        store_address,
        batch_size,
    };
    let service = ProofService::with_config(config);

    service
        .get_single_block_hash_proof(&block_hash, batch_size)
        .await
        .map_err(|e| e.into_eyre())
}

/// Get serializable block hash inclusion proof
///
/// This is a convenience function that returns a serializable proof structure
/// suitable for JSON serialization and API responses. It internally calls
/// [`get_single_block_hash_proof`] and wraps the result in a more convenient format.
///
/// # Parameters
///
/// * `block_hash` - The block hash to generate a proof for (hex-encoded)
/// * `rpc_url` - RPC endpoint URL for the blockchain network  
/// * `store_address` - Address of the store contract on Starknet
/// * `batch_size` - Number of blocks processed in each batch
///
/// # Returns
///
/// Returns a [`BlockHashProofResponse`] containing the batch index, MMR state,
/// and serializable proof structure.
///
/// # Errors
///
/// Returns the same errors as [`get_single_block_hash_proof`].
///
/// # Examples
///
/// ```rust,no_run
/// use publisher::get_block_hash_inclusion_proof;
///
/// # async fn example() -> Result<(), eyre::Error> {
/// let proof_response = get_block_hash_inclusion_proof(
///     "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
///     "http://localhost:8545".to_string(),
///     "0x0987654321098765432109876543210987654321".to_string(),
///     100,
/// ).await?;
///
/// println!("Proof for batch {}: {:?}", proof_response.batch_index, proof_response.proof);
///
/// // The response can be serialized to JSON
/// let json = serde_json::to_string(&proof_response)?;
/// # Ok(())
/// # }
/// ```
pub async fn get_block_hash_inclusion_proof(
    block_hash: String,
    rpc_url: String,
    store_address: String,
    batch_size: u64,
) -> Result<BlockHashProofResponse> {
    let config = PublisherConfig {
        rpc_url,
        chain_id: 0,                     // chain_id not needed for this operation
        verifier_address: String::new(), // verifier_address not needed for this operation
        store_address,
        batch_size,
    };
    let service = ProofService::with_config(config);

    let (batch_index, guest_mmr, guest_proof) = service
        .get_block_hash_inclusion_proof(&block_hash, batch_size)
        .await
        .map_err(|e| e.into_eyre())?;

    Ok(BlockHashProofResponse {
        batch_index,
        guest_mmr,
        proof: guest_proof,
    })
}

// /// Verifies a single block header by block number and returns a receipt.
// ///
// /// This function is kept for compatibility with existing code.
// /// It's recommended to use get_block_hash_proof_serializable instead.
// pub async fn verify_single_block_header(block_number: u64, chain_id: u64) -> Result<Receipt> {
//     // For now, we'll just return a simple, empty receipt since we're transitioning to a new approach
//     tracing::warn!(
//         "verify_single_block_header is deprecated. Use get_block_hash_proof_serializable instead."
//     );

//     // Create a receipt - this is just a placeholder for now
//     let receipt = Receipt::default();

//     Ok(receipt)
// }

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc};

    use mockall::mock;

    use super::*;

    // Mock the necessary components for testing
    mock! {
        pub DbConnection {
            async fn new() -> Result<Arc<Self>>;
            async fn get_block_header_by_hash(&self, block_hash: &str) -> Result<eth_rlp_types::BlockHeader>;
        }
    }

    mock! {
        pub StarknetProvider {
            fn new(rpc_url: &str) -> Result<Self>;
            async fn get_mmr_state(&self, address: &str, batch_index: u64) -> Result<starknet_handler::MmrSnapshot>;
        }
    }

    mock! {
        pub IpfsManager {
            fn with_endpoint() -> Result<Self>;
            async fn fetch_db(&self, hash: &str, output_path: &Path) -> Result<()>;
        }
    }

    #[tokio::test]
    #[ignore] // Requires database connection, IPFS integration - use for integration testing
    async fn test_get_single_block_hash_proof() {
        // This is an integration test that would require a real database and IPFS connection
        // We mark it as #[ignore] so it's not run during regular test runs

        let block_hash =
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string();
        let rpc_url = "http://localhost:8545".to_string();
        let store_address = "0x1234567890123456789012345678901234567890".to_string();
        let batch_size = 100;

        match get_single_block_hash_proof(block_hash, rpc_url, store_address, batch_size).await {
            Ok((batch_index, _guest_mmr, proof)) => {
                println!(
                    "Successfully retrieved proof for batch index: {}",
                    batch_index
                );
                println!("Element index: {}", proof.element_index);
                println!("Element hash: {}", proof.element_hash);
                println!("Siblings count: {}", proof.siblings_hashes.len());
                println!("Peaks count: {}", proof.peaks_hashes.len());
            }
            Err(e) => {
                println!("Error retrieving proof: {}", e);
            }
        }
    }
}
