use starknet::providers::Provider;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use crate::{MmrSnapshot, StarknetHandlerError};
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
    #[instrument(level = "debug", fields(rpc_url = %rpc_url))]
    pub fn new(rpc_url: &str) -> Result<Self, StarknetHandlerError> {
        debug!("Initializing StarknetProvider");

        let parsed_url = Url::parse(rpc_url)?;
        debug!("Parsed RPC URL successfully");

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

    #[instrument(skip(self), level = "debug")]
    pub async fn get_latest_mmr_block(
        &self,
        l2_store_address: &str,
    ) -> Result<u64, StarknetHandlerError> {
        debug!("Fetching latest MMR block");

        let entry_point_selector = selector!("get_latest_mmr_block");

        let data = self
            .provider
            .call(
                FunctionCall {
                    contract_address: Felt::from_hex(l2_store_address)?,
                    entry_point_selector,
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;

        let mmr_block = u64::decode(&data)?;
        info!(mmr_block, "Retrieved latest MMR block");

        Ok(mmr_block)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_min_mmr_block(
        &self,
        l2_store_address: &str,
    ) -> Result<u64, StarknetHandlerError> {
        debug!("Fetching min MMR block");

        let entry_point_selector = selector!("get_min_mmr_block");

        let data = self
            .provider
            .call(
                FunctionCall {
                    contract_address: Felt::from_hex(l2_store_address)?,
                    entry_point_selector,
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;

        let min_mmr_block = u64::decode(&data)?;
        info!(min_mmr_block, "Retrieved minimum MMR block");

        Ok(min_mmr_block)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_mmr_state(
        &self,
        l2_store_address: &str,
        batch_index: u64,
    ) -> Result<MmrSnapshot, StarknetHandlerError> {
        debug!(batch_index, "Fetching MMR state");
        println!("batch_index: {:?}", batch_index);

        let entry_point_selector = selector!("get_mmr_state");

        let data = self
            .provider
            .call(
                FunctionCall {
                    contract_address: Felt::from_hex(l2_store_address)?,
                    entry_point_selector,
                    calldata: vec![Felt::from(batch_index)],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;

        let mmr_state = MmrSnapshot::decode(&data)?;
        info!("Retrieved MMR state");

        Ok(mmr_state)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_latest_relayed_block(
        &self,
        l2_store_address: &str,
    ) -> Result<u64, StarknetHandlerError> {
        debug!("Fetching latest relayed block");

        let entry_point_selector = selector!("get_latest_blockhash_from_l1");

        let data = self
            .provider
            .call(
                FunctionCall {
                    contract_address: Felt::from_hex(l2_store_address)?,
                    entry_point_selector,
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;

        let block_number =
            u64::from_str_radix(data[0].to_hex_string().trim_start_matches("0x"), 16)?;
        info!(block_number, "Retrieved latest relayed block");

        Ok(block_number)
    }
}
