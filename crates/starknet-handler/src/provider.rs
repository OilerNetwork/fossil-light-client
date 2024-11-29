use starknet::providers::Provider;
use std::sync::Arc;

use crate::{MmrState, StarknetHandlerError};
use starknet::macros::selector;
use starknet::{
    core::{
        codec::Decode,
        types::{BlockId, BlockTag, FunctionCall},
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Url},
};
use starknet_crypto::Felt;

pub struct StarknetProvider {
    provider: Arc<JsonRpcClient<HttpTransport>>,
    rpc_url: String,
}

impl StarknetProvider {
    pub fn new(rpc_url: &str) -> Result<Self, StarknetHandlerError> {
        let parsed_url = Url::parse(rpc_url)?;
        Ok(Self {
            provider: Arc::new(JsonRpcClient::new(HttpTransport::new(parsed_url))),
            rpc_url: rpc_url.to_string(),
        })
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    pub fn provider(&self) -> Arc<JsonRpcClient<HttpTransport>> {
        self.provider.clone()
    }

    // pub async fn verify_groth16_proof_onchain(
    //     &self,
    //     verifier_address: &str,
    //     calldata: &[Felt],
    // ) -> Result<Vec<Felt>, StarknetHandlerError> {
    //     tracing::info!("Verifying Groth16 proof onchain...");
    //     let contract_address = felt(verifier_address)?;

    //     let entry_point_selector = selector!("verify_groth16_proof_bn254");

    //     let result = self
    //         .provider
    //         .call(
    //             FunctionCall {
    //                 contract_address,
    //                 entry_point_selector,
    //                 calldata: calldata.to_vec(),
    //             },
    //             BlockId::Tag(BlockTag::Latest),
    //         )
    //         .await
    //         .map_err(|e| StarknetHandlerError::TransactionError(e.to_string()))?;

    //     Ok(result)
    // }

    pub async fn get_latest_mmr_state(
        &self,
        l2_store_address: &Felt,
    ) -> Result<MmrState, StarknetHandlerError> {
        let entry_point_selector = selector!("get_mmr_state");

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
            .await?;

        let mmr_state = MmrState::decode(&data)?;

        Ok(mmr_state)
    }

    pub async fn get_latest_relayed_block(
        &self,
        l2_store_address: &Felt,
    ) -> Result<u64, StarknetHandlerError> {
        let entry_point_selector = selector!("get_latest_blockhash_from_l1");

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
            .await?;

        let block_number =
            u64::from_str_radix(data[0].to_hex_string().trim_start_matches("0x"), 16)?;

        Ok(block_number)
    }
}
