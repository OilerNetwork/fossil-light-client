use std::sync::Arc;

use eyre::{eyre, Result};
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
    pub fn new(rpc_url: &str) -> Result<Self> {
        let parsed_url = Url::parse(rpc_url).map_err(|_| eyre!("Invalid RPC URL provided"))?;
        Ok(Self {
            provider: Arc::new(JsonRpcClient::new(HttpTransport::new(parsed_url))),
            rpc_url: rpc_url.to_string(),
        })
    }

    pub async fn verify_groth16_proof_onchain(
        &self,
        verifier_address: &str,
        calldata: &[Felt],
    ) -> Result<Vec<Felt>> {
        let contract_address = Felt::from_hex(verifier_address)
            .map_err(|_| eyre!("Invalid verifier address provided"))?;
        tracing::info!("contract_address: {:?}", contract_address);

        let entry_point_selector = get_selector_from_name("verify_groth16_proof_bn254")
            .map_err(|_| eyre!("Failed to get selector for verify_groth16_proof_bn254"))?;

        let result = self
            .provider
            .call(
                FunctionCall {
                    contract_address,
                    entry_point_selector,
                    calldata: calldata.to_vec(),
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .map_err(|e| eyre!("Failed to call contract: {}", e))?;

        Ok(result)
    }

    pub async fn get_latest_mmr_state(&self, l2_store_address: &Felt) -> Result<(u64, Felt)> {
        let entry_point_selector = get_selector_from_name("get_mmr_state")
            .map_err(|_| eyre!("Failed to get selector for get_mmr_state"))?;

        let data = self
            .provider
            .call(
                FunctionCall {
                    contract_address: *l2_store_address,
                    entry_point_selector,
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .map_err(|e| eyre!("Failed to call contract: {}", e))?;

        let from_block = u64::from_str_radix(data[0].to_hex_string().trim_start_matches("0x"), 16)
            .map_err(|_| eyre!("Failed to convert hex string to u64"))?;

        Ok((from_block, data[1]))
    }

    pub async fn get_latest_relayed_block(&self, l2_store_address: &Felt) -> Result<u64> {
        let entry_point_selector = get_selector_from_name("get_latest_blockhash_from_l1")
            .map_err(|_| eyre!("Failed to get selector for get_latest_blockhash_from_l1"))?;

        let data = self
            .provider
            .call(
                FunctionCall {
                    contract_address: *l2_store_address,
                    entry_point_selector,
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .map_err(|e| eyre!("Failed to call contract: {}", e))?;

        let block_number =
            u64::from_str_radix(data[0].to_hex_string().trim_start_matches("0x"), 16)
                .map_err(|_| eyre!("Failed to convert hex string to u64"))?;

        Ok(block_number)
    }
}
