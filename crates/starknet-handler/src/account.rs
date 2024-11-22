use crate::MmrState;
use starknet::macros::selector;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::{chain_id, codec::Encode},
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

    pub async fn update_mmr_state(
        &self,
        store_address: Felt,
        latest_mmr_block: u64,
        new_mmr_state: &MmrState,
    ) -> Result<Felt, StarknetHandlerError> {
        let selector = selector!("update_mmr_state");

        let mut calldata = vec![];
        calldata.push(Felt::from(latest_mmr_block + 1));
        new_mmr_state.encode(&mut calldata)?;

        let tx = self
            .account
            .execute_v1(vec![starknet::core::types::Call {
                selector,
                calldata,
                to: store_address,
            }])
            .send()
            .await?;

        Ok(tx.transaction_hash)
    }
}
