use starknet::macros::selector;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::{chain_id, codec::Encode},
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};
use starknet_crypto::Felt;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use common::felt;

use crate::{MmrState, StarknetHandlerError};

pub struct StarknetAccount {
    account: SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>,
}

impl StarknetAccount {
    #[instrument(skip(provider, account_private_key), fields(address = %account_address), level = "debug")]
    pub fn new(
        provider: Arc<JsonRpcClient<HttpTransport>>,
        account_private_key: &str,
        account_address: &str,
    ) -> Result<Self, StarknetHandlerError> {
        debug!("Creating new Starknet account");

        let private_key = felt(account_private_key)?;
        debug!("Private key converted to felt");

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));
        let address = felt(account_address)?;

        debug!(
            chain_id = ?chain_id::SEPOLIA,
            encoding = ?ExecutionEncoding::New,
            "Initializing SingleOwnerAccount"
        );

        let account = SingleOwnerAccount::new(
            provider,
            signer,
            address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        );

        debug!("Starknet account successfully created");
        Ok(Self { account })
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn verify_mmr_proof(
        &self,
        verifier_address: &str,
        proof: Vec<Felt>,
    ) -> Result<Felt, StarknetHandlerError> {
        debug!(
            verifier_address = %verifier_address,
            proof_length = proof.len(),
            "Verifying MMR proof"
        );

        let selector = selector!("verify_mmr_proof");

        debug!("Executing verification transaction");
        let tx = self
            .account
            .execute_v1(vec![starknet::core::types::Call {
                selector,
                calldata: proof,
                to: felt(verifier_address)?,
            }])
            .send()
            .await?;

        info!(
            tx_hash = ?tx.transaction_hash,
            "MMR proof onchain verification successful."
        );
        Ok(tx.transaction_hash)
    }

    pub async fn update_mmr_state(
        &self,
        store_address: &str,
        batch_index: u64,
        mmr_state: &MmrState,
    ) -> Result<Felt, StarknetHandlerError> {
        let selector = selector!("update_mmr_state");

        let mut calldata = vec![];
        calldata.push(Felt::from(batch_index));
        mmr_state.encode(&mut calldata)?;

        let tx = self
            .account
            .execute_v1(vec![starknet::core::types::Call {
                selector,
                calldata,
                to: felt(store_address)?,
            }])
            .send()
            .await?;

        Ok(tx.transaction_hash)
    }
}
