use std::sync::Arc;

use eyre::{eyre, Result};
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::chain_id,
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};
use starknet_crypto::Felt;

pub struct StarknetAccount {
    pub account: SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>,
}

impl StarknetAccount {
    pub fn new(
        provider: Arc<JsonRpcClient<HttpTransport>>,
        account_private_key: &str,
        account_address: &str,
    ) -> Result<Self> {
        let private_key = Felt::from_hex(account_private_key)
            .map_err(|_| eyre!("Invalid private key provided"))?;
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));

        let address =
            Felt::from_hex(account_address).map_err(|_| eyre!("Invalid address provided"))?;

        let account = SingleOwnerAccount::new(
            provider, // Use `Arc` directly
            signer,
            address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        );

        Ok(Self { account })
    }

    pub async fn update_mmr_state(
        &self,
        store_address: Felt,
        latest_block_number: u64,
        new_mmr_root: Felt,
    ) -> Result<Felt> {
        let selector = starknet::core::utils::get_selector_from_name("update_mmr_state")
            .map_err(|_| eyre!("Failed to get selector for update_mmr_state"))?;

        let tx = self
            .account
            .execute_v1(vec![starknet::core::types::Call {
                selector,
                calldata: vec![Felt::from(latest_block_number), new_mmr_root],
                to: store_address,
            }])
            .send()
            .await?;

        Ok(tx.transaction_hash)
    }
}
