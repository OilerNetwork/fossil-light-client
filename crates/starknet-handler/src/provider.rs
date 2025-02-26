use starknet::providers::Provider;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use crate::MmrSnapshot;
use eyre::Result;
use starknet::macros::selector;
use starknet::{
    core::{
        codec::Decode,
        types::{BlockId, BlockTag, FunctionCall},
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Url},
};
use starknet_crypto::Felt;
#[derive(Debug)]
pub struct StarknetProvider {
    provider: Arc<JsonRpcClient<HttpTransport>>,
    rpc_url: String,
}

impl StarknetProvider {
    #[instrument(level = "debug", fields(rpc_url = %rpc_url))]
    pub fn new(rpc_url: &str) -> Result<Self> {
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
    pub async fn get_latest_mmr_block(&self, l2_store_address: &str) -> Result<u64> {
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
    pub async fn get_min_mmr_block(&self, l2_store_address: &str) -> Result<u64> {
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
    ) -> Result<MmrSnapshot> {
        debug!(batch_index, "Fetching MMR state");

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
        info!("Retrieved On-chain MMR state");

        Ok(mmr_state)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_latest_relayed_block(&self, l2_store_address: &str) -> Result<u64> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use mockall::predicate;
    use mockall::predicate::*;
    // use std::str::FromStr;

    #[test]
    fn test_provider_new() {
        let rpc_url = "http://localhost:5050";
        let provider = StarknetProvider::new(rpc_url);
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.rpc_url(), rpc_url);
    }

    #[test]
    fn test_provider_new_invalid_url() {
        let rpc_url = "not-a-valid-url";
        let provider = StarknetProvider::new(rpc_url);
        assert!(provider.is_err());
    }

    #[test]
    fn test_provider_getters() {
        let rpc_url = "http://localhost:5050";
        let provider = StarknetProvider::new(rpc_url).unwrap();

        assert_eq!(provider.rpc_url(), rpc_url);
        assert!(Arc::strong_count(&provider.provider()) >= 1);
    }

    mock! {
        Provider {
            fn call(
                &self,
                function_call: FunctionCall,
                block_id: BlockId,
            ) -> Result<Vec<Felt>, starknet::providers::ProviderError>;
        }
    }

    #[tokio::test]
    async fn test_get_latest_mmr_block() {
        let expected_block = 42u64;

        let mut mock_provider = MockProvider::new();
        mock_provider
            .expect_call()
            .with(
                predicate::function(|call: &FunctionCall| {
                    call.entry_point_selector == selector!("get_latest_mmr_block")
                }),
                predicate::eq(BlockId::Tag(BlockTag::Latest)),
            )
            .return_once(move |_, _| Ok(vec![Felt::from(expected_block)]));

        // TODO: Inject mock provider into StarknetProvider
        // let provider = StarknetProvider::with_provider(mock_provider);
        // let result = provider.get_latest_mmr_block(l2_store_address).await;
        // assert!(result.is_ok());
        // assert_eq!(result.unwrap(), expected_block);
    }

    #[tokio::test]
    async fn test_get_min_mmr_block() {
        let expected_block = 10u64;

        let mut mock_provider = MockProvider::new();
        mock_provider
            .expect_call()
            .with(
                predicate::function(|call: &FunctionCall| {
                    call.entry_point_selector == selector!("get_min_mmr_block")
                }),
                predicate::eq(BlockId::Tag(BlockTag::Latest)),
            )
            .return_once(move |_, _| Ok(vec![Felt::from(expected_block)]));

        // TODO: Similar to above test
    }

    #[tokio::test]
    async fn test_get_mmr_state() {
        let batch_index = 5u64;

        let mut mock_provider = MockProvider::new();
        mock_provider
            .expect_call()
            .with(
                predicate::function(move |call: &FunctionCall| {
                    call.entry_point_selector == selector!("get_mmr_state")
                        && call.calldata == vec![Felt::from(batch_index)]
                }),
                predicate::eq(BlockId::Tag(BlockTag::Latest)),
            )
            .return_once(move |_, _| {
                // Create a mock MmrSnapshot response
                Ok(vec![
                    Felt::from(1u64), // root
                    Felt::from(2u64), // size
                ])
            });

        // TODO: Similar to above tests
    }

    #[tokio::test]
    async fn test_get_latest_relayed_block() {
        let expected_block = 100u64;

        let mut mock_provider = MockProvider::new();
        mock_provider
            .expect_call()
            .with(
                predicate::function(|call: &FunctionCall| {
                    call.entry_point_selector == selector!("get_latest_blockhash_from_l1")
                }),
                predicate::eq(BlockId::Tag(BlockTag::Latest)),
            )
            .return_once(move |_, _| {
                Ok(vec![
                    Felt::from_hex(&format!("{:x}", expected_block)).unwrap()
                ])
            });

        // TODO: Similar to above tests
    }
}
