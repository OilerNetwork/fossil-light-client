use starknet_handler::{account::StarknetAccount, provider::StarknetProvider};

use crate::{
    core::{AccumulatorBuilder, BatchProcessor, MMRStateManager, ProofGenerator},
    errors::PublisherError,
};
use methods::{MMR_BUILD_ELF, MMR_BUILD_ID};
use tracing::info;

pub async fn prove_mmr_update(
    rpc_url: &String,
    chain_id: u64,
    verifier_address: &String,
    store_address: &String,
    account_private_key: &String,
    account_address: &String,
    batch_size: u64,
    start_block: u64,
    end_block: u64,
) -> Result<(), PublisherError> {
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
        .update_mmr_with_new_headers(start_block, end_block, false)
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
    end_block: u64,
) -> Result<Option<String>, PublisherError> {
    println!("OPERATION.rs");
    info!(
        latest_mmr_block = start_block - 1,
        latest_relayed_block = end_block,
        "Starting MMR update and proof generation"
    );

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
        .update_mmr_with_new_headers(start_block, end_block, false)
        .await?;

    // For now, return None as we don't have a way to capture the tx hash
    Ok(None)
}
