use risc0_zkvm::Receipt;
use starknet_handler::{
    account::StarknetAccount,
    provider::{LatestRelayBlock, StarknetProvider},
};

use crate::core::{AccumulatorBuilder, BatchProcessor, MMRStateManager, ProofGenerator};
use crate::db::DbConnection;
use eyre::Result;
use guest_types::WorldCoinInput;
use methods::{MMR_BUILD_ELF, MMR_BUILD_ID, WORLD_COIN_ELF, WORLD_COIN_ID};

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

/// Verifies a single block header and returns its number, hash, and state root
///
/// This function:
/// 1. Fetches the block header from the database
/// 2. Verifies its validity using the zkVM with a STARK proof
/// 3. Returns the block number, hash, and state root along with the STARK proof
pub async fn verify_single_block_header(block_number: u64, chain_id: u64) -> Result<Receipt> {
    // Connect to the database
    let db_connection = DbConnection::new().await?;

    // Fetch the block header
    let header = db_connection
        .get_block_header_by_number(block_number)
        .await?;

    tracing::info!("Verifying block header for block {}", block_number);

    // Create the zkVM input
    let input = WorldCoinInput::new(header, chain_id);

    // Create the proof generator with WORLD_COIN_ID
    let proof_generator = ProofGenerator::new(WORLD_COIN_ELF, WORLD_COIN_ID)?;

    // Generate a STARK proof
    let stark_proof = proof_generator.generate_stark_proof(input).await?;

    let receipt = stark_proof.receipt();

    tracing::info!(
        "Successfully verified block header: number={}, chain_id={}",
        block_number,
        chain_id
    );

    Ok(receipt)
}
