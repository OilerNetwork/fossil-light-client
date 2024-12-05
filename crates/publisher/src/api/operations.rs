use crate::{
    core::AccumulatorBuilder, errors::PublisherError, utils::Stark, validator::ValidatorBuilder,
};

const DEFAULT_BATCH_SIZE: u64 = 1024;

pub async fn prove_mmr_update(
    rpc_url: &String,
    verifier_address: &String,
    account_private_key: &String,
    account_address: &String,
    batch_size: u64,
    start_block: u64,
    end_block: u64,
    skip_proof_verification: bool,
) -> Result<(), PublisherError> {
    let mut builder = AccumulatorBuilder::new(
        rpc_url,
        verifier_address,
        account_private_key,
        account_address,
        batch_size,
        skip_proof_verification,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create AccumulatorBuilder");
        e
    })?;

    tracing::info!("Starting MMR update and proof generation");

    builder
        .update_mmr_with_new_headers(start_block, end_block)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update MMR with new headers");
            e
        })?;

    tracing::debug!("Successfully generated proof for block range");

    Ok(())
}

pub async fn prove_headers_integrity_and_inclusion(
    rpc_url: &String,
    l2_store_address: &String,
    headers: &Vec<eth_rlp_types::BlockHeader>,
    skip_proof_verification: Option<bool>,
) -> Result<Vec<Stark>, PublisherError> {
    let skip_proof = skip_proof_verification.unwrap_or(false);

    let validator =
        ValidatorBuilder::new(rpc_url, l2_store_address, DEFAULT_BATCH_SIZE, skip_proof)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create ValidatorBuilder");
                e
            })?;

    let result = validator
        .verify_blocks_integrity_and_inclusion(headers)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to verify blocks validity and inclusion");
            e
        })?;

    tracing::info!("Successfully verified blocks validity and inclusion");

    Ok(result)
}
