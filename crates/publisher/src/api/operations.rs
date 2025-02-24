use starknet_handler::{account::StarknetAccount, provider::StarknetProvider};

use crate::{
    core::AccumulatorBuilder, errors::PublisherError,
};

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
    skip_proof_verification: bool,
) -> Result<(), PublisherError> {
    let starknet_provider = StarknetProvider::new(rpc_url)?;
    let starknet_account = StarknetAccount::new(
        starknet_provider.provider(),
        account_private_key,
        account_address,
    )?;

    let mut builder = AccumulatorBuilder::new(
        rpc_url,
        chain_id,
        verifier_address,
        store_address,
        starknet_account,
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
        .update_mmr_with_new_headers(start_block, end_block, false)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update MMR with new headers");
            e
        })?;

    tracing::debug!("Successfully generated proof for block range");

    Ok(())
}
