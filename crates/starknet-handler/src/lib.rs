use std::sync::Arc;

use eyre::Result;
use starknet::{
    accounts::Account,
    core::{
        types::{BlockId, BlockTag, FunctionCall},
        utils::get_selector_from_name,
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url},
};
use starknet::{
    accounts::{ExecutionEncoding, SingleOwnerAccount},
    core::chain_id,
    signers::{LocalWallet, SigningKey},
};
use starknet_crypto::Felt;
use tracing::info;

pub struct StarknetProvider {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
}

impl StarknetProvider {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            provider: Arc::new(JsonRpcClient::new(HttpTransport::new(
                Url::parse(rpc_url).expect("Invalid RPC URL provided"),
            ))),
        }
    }

    pub async fn verify_groth16_proof_onchain(
        &self,
        verifier_address: &str,
        calldata: &Vec<Felt>,
    ) -> Result<Vec<Felt>> {
        let contract_address =
            Felt::from_hex(verifier_address).expect("Invalid verifier address provided");
        info!("contract_address: {:?}", contract_address);

        let result = self
            .provider
            .call(
                FunctionCall {
                    contract_address,
                    entry_point_selector: get_selector_from_name("verify_groth16_proof_bn254")
                        .unwrap(),
                    calldata: calldata.clone(),
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect("Failed to call contract");

        Ok(result)
    }

    pub async fn get_latest_mmr_state(
        &self,
        l2_store_address: &Felt,
    ) -> Result<(u64, Felt)> {
        let data = self
            .provider
            .call(
                FunctionCall {
                    contract_address: l2_store_address.clone(),
                    entry_point_selector: get_selector_from_name("get_mmr_state")
                        .unwrap(),
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect("Failed to call contract");

        let from_block = u64::from_str_radix(data[0].to_hex_string().trim_start_matches("0x"), 16)
            .expect("Failed to convert hex string to u64");

        Ok((from_block, data[1]))
    }

    pub async fn get_latest_relayed_block(
        &self,
        l2_store_address: &Felt,
    ) -> Result<u64> {
        let data = self
            .provider
            .call(
                FunctionCall {
                    contract_address: l2_store_address.clone(),
                    entry_point_selector: get_selector_from_name("get_latest_blockhash_from_l1")
                        .unwrap(),
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect("Failed to call contract");

        let block_number = u64::from_str_radix(data[0].to_hex_string().trim_start_matches("0x"), 16)
            .expect("Failed to convert hex string to u64");

        Ok(block_number)
    }
}

#[derive(Debug)]
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
                selector: starknet::core::utils::get_selector_from_name("update_mmr_state").unwrap(),
                calldata: vec![Felt::from(latest_block_number), new_mmr_root],
                to: store_address,
            }])
            .send()
            .await?;

        Ok(tx.transaction_hash)
    }
}
