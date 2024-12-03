use crate::{core::AccumulatorBuilder, errors::PublisherError, validator::ValidatorBuilder};

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
    .await?;

    tracing::debug!(
        start_block,
        end_block,
        "Starting MMR update and proof generation"
    );

    builder
        .update_mmr_with_new_headers(start_block, end_block)
        .await?;
    tracing::debug!(
        start_block,
        end_block,
        "Successfully generated proof for block range"
    );

    Ok(())
}

pub async fn prove_headers_validity_and_inclusion(
    headers: &Vec<eth_rlp_types::BlockHeader>,
    batch_size: u64,
    skip_proof_verification: Option<bool>,
) -> Result<bool, PublisherError> {
    let skip_proof = skip_proof_verification.unwrap_or(false);
    let validator = ValidatorBuilder::new(batch_size, skip_proof).await?;

    let result = validator
        .verify_blocks_validity_and_inclusion(headers)
        .await?;

    Ok(result)
}
