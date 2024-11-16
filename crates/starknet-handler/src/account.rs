use std::sync::Arc;

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
    ) -> Self {
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex(account_private_key).expect("Invalid private key provided"),
        ));

        Self {
            account: SingleOwnerAccount::new(
                provider, // Use `Arc` directly
                signer,
                Felt::from_hex(account_address).expect("Invalid address provided"),
                chain_id::SEPOLIA,
                ExecutionEncoding::New,
            ),
        }
    }

    pub async fn update_mmr_state(
        &self,
        store_address: Felt,
        latest_block_number: u64,
        new_mmr_root: Felt,
    ) -> eyre::Result<Felt> {
        let tx = self
            .account
            .execute_v1(vec![starknet::core::types::Call {
                selector: starknet::core::utils::get_selector_from_name("update_mmr_state")
                    .unwrap(),
                calldata: vec![Felt::from(latest_block_number), new_mmr_root],
                to: store_address,
            }])
            .send()
            .await?;

        Ok(tx.transaction_hash)
    }
}
