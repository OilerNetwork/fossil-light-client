use std::sync::Arc;

use eyre::Result;
use starknet::{
    core::{
        types::{BlockId, BlockTag, FunctionCall},
        utils::get_selector_from_name,
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url},
};
use starknet_crypto::Felt;

pub struct StarknetProvider {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
    pub rpc_url: String,
}

impl StarknetProvider {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            provider: Arc::new(JsonRpcClient::new(HttpTransport::new(
                Url::parse(rpc_url).expect("Invalid RPC URL provided"),
            ))),
            rpc_url: rpc_url.to_string(),
        }
    }

    pub async fn verify_groth16_proof_onchain(
        &self,
        verifier_address: &str,
        calldata: &Vec<Felt>,
    ) -> Result<Vec<Felt>> {
        let contract_address =
            Felt::from_hex(verifier_address).expect("Invalid verifier address provided");
        tracing::info!("contract_address: {:?}", contract_address);

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

    pub async fn get_latest_mmr_state(&self, l2_store_address: &Felt) -> Result<(u64, Felt)> {
        let data = self
            .provider
            .call(
                FunctionCall {
                    contract_address: l2_store_address.clone(),
                    entry_point_selector: get_selector_from_name("get_mmr_state").unwrap(),
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

    pub async fn get_latest_relayed_block(&self, l2_store_address: &Felt) -> Result<u64> {
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

        let block_number =
            u64::from_str_radix(data[0].to_hex_string().trim_start_matches("0x"), 16)
                .expect("Failed to convert hex string to u64");

        Ok(block_number)
    }
}
