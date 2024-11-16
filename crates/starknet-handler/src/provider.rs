use std::sync::Arc;

use crate::{error::StarknetHandlerError, get_selector};
use eyre::Result;
use starknet::{
    core::types::{BlockId, BlockTag, FunctionCall},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url},
};
use starknet_crypto::Felt;

pub struct StarknetProvider {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
    pub rpc_url: String,
}

impl StarknetProvider {
    pub fn new(rpc_url: &str) -> Result<Self> {
        let parsed_url = Url::parse(rpc_url)
            .map_err(|_| StarknetHandlerError::ParseError(rpc_url.to_string()))?;
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
            .map_err(|_| StarknetHandlerError::ParseError(verifier_address.to_string()))?;
        tracing::info!("contract_address: {:?}", contract_address);

        let entry_point_selector = get_selector("verify_groth16_proof_bn254")?;

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
            .map_err(|e| StarknetHandlerError::TransactionError(e.to_string()))?;

        Ok(result)
    }

    pub async fn get_latest_mmr_state(&self, l2_store_address: &Felt) -> Result<(u64, Felt)> {
        let entry_point_selector = get_selector("get_mmr_state")?;

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
            .map_err(|e| StarknetHandlerError::TransactionError(e.to_string()))?;

        let from_block = u64::from_str_radix(data[0].to_hex_string().trim_start_matches("0x"), 16)
            .map_err(|_| StarknetHandlerError::ParseError(data[0].to_hex_string()))?;

        Ok((from_block, data[1]))
    }

    pub async fn get_latest_relayed_block(&self, l2_store_address: &Felt) -> Result<u64> {
        let entry_point_selector = get_selector("get_latest_blockhash_from_l1")?;

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
            .map_err(|e| StarknetHandlerError::TransactionError(e.to_string()))?;

        let block_number =
            u64::from_str_radix(data[0].to_hex_string().trim_start_matches("0x"), 16)
                .map_err(|_| StarknetHandlerError::ParseError(data[0].to_hex_string()))?;

        Ok(block_number)
    }
}
