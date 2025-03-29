use std::path::Path;

use eyre::Result;
use guest_mmr::{core::GuestMMR, helper::find_peaks};
use guest_types::GuestMMRProof;
use methods::{MMR_BUILD_ELF, MMR_BUILD_ID};
use mmr;
use serde::{Deserialize, Serialize};
use starknet_handler::{
    account::StarknetAccount,
    provider::{LatestRelayBlock, StarknetProvider},
};

use crate::{
    core::{AccumulatorBuilder, BatchProcessor, MMRStateManager, ProofGenerator},
    db::DbConnection,
};

// Define a serializable proof structure for API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockHashProofResponse {
    pub batch_index: u64,
    pub guest_mmr: GuestMMR,
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
    let starknet_provider = StarknetProvider::new(rpc_url)?;
    let starknet_account = StarknetAccount::new(
        starknet_provider.provider(),
        account_private_key,
        account_address,
    )?;

    // Create components for AccumulatorBuilder
    let proof_generator = ProofGenerator::new(MMR_BUILD_ELF, MMR_BUILD_ID)?;
    let mmr_state_manager = MMRStateManager::new(starknet_account, store_address, rpc_url);
    let batch_processor = BatchProcessor::new(batch_size, proof_generator, mmr_state_manager)?;

    let mut builder = AccumulatorBuilder::new(
        rpc_url,
        chain_id,
        verifier_address,
        batch_processor,
        0, // current_batch
        0, // total_batches
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create AccumulatorBuilder");
        e
    })?;

    tracing::info!("Starting MMR update and proof generation");

    builder
        .update_mmr_with_new_headers(start_block, latest_relayed_block_and_hash, false)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update MMR with new headers");
            e
        })?;

    tracing::debug!("Successfully generated proof for block range");

    Ok(())
}

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
    let starknet_provider = StarknetProvider::new(rpc_url)?;
    let starknet_account = StarknetAccount::new(
        starknet_provider.provider(),
        account_private_key,
        account_address,
    )?;

    // Create components for AccumulatorBuilder
    let proof_generator = ProofGenerator::new(MMR_BUILD_ELF, MMR_BUILD_ID)?;
    let mmr_state_manager = MMRStateManager::new(starknet_account, store_address, rpc_url);
    let batch_processor = BatchProcessor::new(batch_size, proof_generator, mmr_state_manager)?;

    // Use the constructor directly with the correct signature
    let mut builder = AccumulatorBuilder::new(
        rpc_url,
        chain_id,
        verifier_address,
        batch_processor,
        0, // current_batch
        0, // total_batches
    )
    .await?;

    // Always generate and verify proofs (false = don't skip proof verification)
    builder
        .update_mmr_with_new_headers(start_block, latest_relayed_block_and_hash, false)
        .await?;

    Ok(())
}

/// Verifies a single block hash and returns its Merkle proof
///
/// This function:
/// 1. Fetches the MMR state from onchain for the batch containing the block hash
/// 2. Downloads the MMR DB file from IPFS
/// 3. Verifies that the downloaded file matches the onchain MMR root and leaves count
/// 4. Produces a merkle proof for the block hash
/// 5. Returns the proof along with the batch number
pub async fn get_single_block_hash_proof(
    block_hash: String,
    rpc_url: String,
    store_address: String,
    batch_size: u64,
) -> Result<(u64, GuestMMR, mmr::Proof)> {
    tracing::info!("Looking up proof for block hash: {}", block_hash);

    // Connect to Starknet
    let provider = StarknetProvider::new(&rpc_url)?;

    // Get the block header to determine which batch it belongs to
    let db_connection = DbConnection::new().await?;
    let header = db_connection.get_block_header_by_hash(&block_hash).await?;

    // Calculate the batch index based on the block number and batch size
    let batch_index = header.number as u64 / batch_size;
    tracing::info!("Block belongs to batch index: {}", batch_index);

    // Fetch the MMR state from onchain
    let mmr_state = provider.get_mmr_state(&store_address, batch_index).await?;

    // Get the IPFS hash from the MMR state
    let ipfs_hash = mmr_state.ipfs_hash();
    let ipfs_hash_str =
        String::try_from(ipfs_hash).map_err(|_| eyre::eyre!("Invalid IPFS hash format"))?;

    // Set up temporary file path for the downloaded DB
    let batch_file_name = common::get_or_create_db_path(&format!("batch_{}.db", batch_index))?;

    // Initialize IPFS manager and download the DB file
    let ipfs_manager = ipfs_utils::IpfsManager::with_endpoint()?;
    match ipfs_manager
        .fetch_db(&ipfs_hash_str, Path::new(&batch_file_name))
        .await
    {
        Ok(_) => {
            tracing::info!(
                "Successfully downloaded DB from IPFS for batch {}",
                batch_index
            );
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                batch_index = batch_index,
                "Failed to fetch DB from IPFS, falling back to local file"
            );

            if !std::path::Path::new(&batch_file_name).exists() {
                return Err(eyre::eyre!(
                    "Failed to fetch DB from IPFS and no local file exists"
                ));
            }
        }
    }

    // Initialize the MMR from the downloaded DB
    let (store_manager, mmr, pool) = mmr_utils::initialize_mmr(&batch_file_name).await?;

    // Verify that the MMR root in the downloaded DB matches the onchain state
    let mmr_elements_count = mmr.elements_count.get().await?;
    let bag = mmr.bag_the_peaks(Some(mmr_elements_count)).await?;
    let mmr_root_hex = mmr
        .calculate_root_hash(&bag, mmr_elements_count)?
        .to_string();
    let mmr_root = starknet_handler::u256_from_hex(&mmr_root_hex)?;

    if mmr_root != mmr_state.root_hash() {
        return Err(eyre::eyre!(
            "MMR root mismatch: expected {} but got {}",
            mmr_state.root_hash(),
            mmr_root
        ));
    }

    // Verify leaves count
    let mmr_leaves_count = mmr.leaves_count.get().await?;
    if mmr_leaves_count as u64 != mmr_state.leaves_count() {
        return Err(eyre::eyre!(
            "Leaves count mismatch: expected {} but got {}",
            mmr_state.leaves_count(),
            mmr_leaves_count
        ));
    }

    let peaks = mmr
        .retrieve_peaks_hashes(find_peaks(mmr_elements_count), None)
        .await?;

    tracing::info!("MMR state verification successful");

    // Get the element index for the block hash
    let element_index = store_manager
        .get_element_index_for_value(&pool, &block_hash)
        .await?
        .ok_or_else(|| eyre::eyre!("Block hash not found in MMR"))?;

    let guest_mmr = GuestMMR::new(peaks, mmr_elements_count, mmr_leaves_count);

    // Get the Merkle proof for the block hash
    let proof = mmr.get_proof(element_index, None).await?;

    tracing::info!(
        "Successfully generated Merkle proof for block hash: {}",
        block_hash
    );

    Ok((batch_index, guest_mmr, proof))
}

/// Convenience function that returns a serializable proof structure
pub async fn get_block_hash_inclusion_proof(
    block_hash: String,
    rpc_url: String,
    store_address: String,
    batch_size: u64,
) -> Result<BlockHashProofResponse> {
    let (batch_index, guest_mmr, proof) =
        get_single_block_hash_proof(block_hash, rpc_url, store_address, batch_size).await?;

    // Convert mmr::Proof to GuestMMRProof
    let guest_proof = GuestMMRProof {
        element_index: proof.element_index,
        element_hash: proof.element_hash,
        siblings_hashes: proof.siblings_hashes,
        peaks_hashes: proof.peaks_hashes,
        elements_count: proof.elements_count,
    };

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
    use std::sync::Arc;

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
