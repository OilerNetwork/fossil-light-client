use eyre::Result;
use guest_mmr::core::GuestMMR;
use guest_types::GuestMMRProof;
use mmr;
use serde::{Deserialize, Serialize};
use starknet_handler::provider::LatestRelayBlock;

use crate::service::ProofService;

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
    let service = ProofService::new(
        rpc_url,
        0,             // chain_id not needed for this operation
        String::new(), // verifier_address not needed for this operation
        store_address,
    );

    service
        .get_single_block_hash_proof(&block_hash, batch_size)
        .await
        .map_err(|e| e.into_eyre())
}

/// Convenience function that returns a serializable proof structure
pub async fn get_block_hash_inclusion_proof(
    block_hash: String,
    rpc_url: String,
    store_address: String,
    batch_size: u64,
) -> Result<BlockHashProofResponse> {
    let service = ProofService::new(
        rpc_url,
        0,             // chain_id not needed for this operation
        String::new(), // verifier_address not needed for this operation
        store_address,
    );

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
