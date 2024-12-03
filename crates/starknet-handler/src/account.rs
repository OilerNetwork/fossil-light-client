use crate::MmrState;
use starknet::macros::selector;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::chain_id,
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};
use starknet_crypto::Felt;
use std::sync::Arc;

use common::felt;

use crate::StarknetHandlerError;

pub struct StarknetAccount {
    account: SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>,
}

impl StarknetAccount {
    pub fn new(
        provider: Arc<JsonRpcClient<HttpTransport>>,
        account_private_key: &str,
        account_address: &str,
    ) -> Result<Self, StarknetHandlerError> {
        let private_key = felt(account_private_key)?;
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));

        let address = felt(account_address)?;

        let account = SingleOwnerAccount::new(
            provider, // Use `Arc` directly
            signer,
            address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        );

        Ok(Self { account })
    }

    pub fn account(&self) -> SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet> {
        self.account.clone()
    }

    pub async fn verify_mmr_proof(
        &self,
        verifier_address: &str,
        new_mmr_state: &MmrState,
        proof: Vec<Felt>,
    ) -> Result<Felt, StarknetHandlerError> {
        let selector = selector!("verify_mmr_proof");

        println!("new_mmr_state: {:?}", new_mmr_state);

        let batch_index = new_mmr_state.latest_block_number() / 1024;
        let latest_mmr_block = new_mmr_state.latest_block_number();

        let mut calldata = vec![Felt::from(batch_index), Felt::from(latest_mmr_block)];

        calldata.extend(proof.iter().cloned());

        let tx = self
            .account
            .execute_v1(vec![starknet::core::types::Call {
                selector,
                calldata,
                to: felt(verifier_address)?,
            }])
            .send()
            .await?;

        Ok(tx.transaction_hash)
    }
}
