use std::{sync::Arc, time::Duration};

use eyre::Result;
use num_traits::ToPrimitive;
use starknet::{
    core::{
        codec::Decode,
        types::{BlockId, BlockTag, FunctionCall, U256},
    },
    macros::selector,
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url},
};
use tracing::{debug, error, info, instrument, warn};

use crate::MmrSnapshot;

// Default retry configuration
const DEFAULT_MAX_RETRIES: u32 = 5;
const DEFAULT_INITIAL_BACKOFF_MS: u64 = 100;
const DEFAULT_MAX_BACKOFF_MS: u64 = 10000; // 10 seconds

#[derive(Clone, Debug)]
pub struct LatestRelayBlock {
    pub block_number: u64,
    pub block_hash: String,
}

use starknet_crypto::Felt;
#[derive(Debug)]
pub struct StarknetProvider {
    provider: Arc<JsonRpcClient<HttpTransport>>,
    rpc_url: String,
    max_retries: u32,
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
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
            max_retries: DEFAULT_MAX_RETRIES,
            initial_backoff_ms: DEFAULT_INITIAL_BACKOFF_MS,
            max_backoff_ms: DEFAULT_MAX_BACKOFF_MS,
        })
    }

    #[instrument(level = "debug", fields(rpc_url = %rpc_url, max_retries, initial_backoff_ms, max_backoff_ms))]
    pub fn new_with_retry_config(
        rpc_url: &str,
        max_retries: u32,
        initial_backoff_ms: u64,
        max_backoff_ms: u64,
    ) -> Result<Self> {
        debug!("Initializing StarknetProvider with custom retry config");

        let parsed_url = Url::parse(rpc_url)?;
        debug!("Parsed RPC URL successfully");

        Ok(Self {
            provider: Arc::new(JsonRpcClient::new(HttpTransport::new(parsed_url))),
            rpc_url: rpc_url.to_string(),
            max_retries,
            initial_backoff_ms,
            max_backoff_ms,
        })
    }

    /// Returns a reference to the provider's RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Returns a clone of the provider's `JsonRpcClient`
    pub fn provider(&self) -> Arc<JsonRpcClient<HttpTransport>> {
        self.provider.clone()
    }

    /// Generic retry mechanism for any async operation with exponential backoff
    async fn with_retry<F, Fut, T>(&self, operation_name: &str, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut backoff_ms = self.initial_backoff_ms;
        let mut attempt = 0;

        loop {
            attempt += 1;

            match f().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!(
                            operation = operation_name,
                            attempt, "Operation succeeded after retry"
                        );
                    }
                    return Ok(result);
                }
                Err(err) => {
                    if attempt >= self.max_retries {
                        error!(
                            operation = operation_name,
                            attempt,
                            error = %err,
                            "Operation failed after maximum retries"
                        );
                        return Err(err);
                    }

                    // Calculate next backoff with exponential increase, capped at max_backoff_ms
                    backoff_ms = std::cmp::min(backoff_ms * 2, self.max_backoff_ms);

                    warn!(
                        operation = operation_name,
                        attempt,
                        next_attempt = attempt + 1,
                        backoff_ms,
                        error = %err,
                        "Operation failed, retrying after backoff"
                    );

                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_latest_mmr_block(&self, l2_store_address: &str) -> Result<u64> {
        debug!("Fetching latest MMR block");

        self.with_retry("get_latest_mmr_block", || async {
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
            debug!(mmr_block, "Retrieved latest MMR block");

            Ok(mmr_block)
        })
        .await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_min_mmr_block(&self, l2_store_address: &str) -> Result<u64> {
        debug!("Fetching min MMR block");

        self.with_retry("get_min_mmr_block", || async {
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
            debug!(min_mmr_block, "Retrieved minimum MMR block");

            Ok(min_mmr_block)
        })
        .await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_mmr_state(
        &self,
        l2_store_address: &str,
        batch_index: u64,
    ) -> Result<MmrSnapshot> {
        debug!(batch_index, "Fetching MMR state");

        self.with_retry("get_mmr_state", || async {
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
            debug!("Retrieved On-chain MMR state");

            Ok(mmr_state)
        })
        .await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_latest_relayed_block(
        &self,
        l2_store_address: &str,
    ) -> Result<LatestRelayBlock> {
        debug!("Fetching latest relayed block");

        self.with_retry("get_latest_relayed_block", || async {
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
            let block_hash_u256 = U256::from_words(
                data[1]
                    .to_u128()
                    .ok_or_else(|| eyre::eyre!("Failed to convert Felt to u128"))?,
                data[2]
                    .to_u128()
                    .ok_or_else(|| eyre::eyre!("Failed to convert Felt to u128"))?,
            );
            let block_hash = format!("{block_hash_u256:#x}");
            debug!(block_number, "Retrieved latest relayed block");

            Ok(LatestRelayBlock {
                block_number,
                block_hash,
            })
        })
        .await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_total_batches(&self, l2_store_address: &str) -> Result<u64> {
        debug!("Fetching total batches");

        self.with_retry("get_total_batches", || async {
            let entry_point_selector = selector!("get_total_batches");

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

            let total_batches = u64::decode(&data)?;
            debug!(total_batches, "Retrieved total batches");

            Ok(total_batches)
        })
        .await
    }

    pub async fn get_avg_fees_in_range(
        &self,
        l2_store_address: &str,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<Felt>> {
        self.with_retry("get_avg_fees_in_range", || async {
            let entry_point_selector = selector!("get_avg_fees_in_range");

            let data = self
                .provider
                .call(
                    FunctionCall {
                        contract_address: Felt::from_hex(l2_store_address)?,
                        entry_point_selector,
                        calldata: vec![Felt::from(start_timestamp), Felt::from(end_timestamp)],
                    },
                    BlockId::Tag(BlockTag::Latest),
                )
                .await?;

            Ok(data)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    // use std::str::FromStr;
    use super::*;

    #[test]
    fn test_provider_new() {
        let rpc_url = "http://localhost:5050";
        let provider = StarknetProvider::new(rpc_url);
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.rpc_url(), rpc_url);
    }

    #[test]
    fn test_provider_new_with_retry_config() {
        let rpc_url = "http://localhost:5050";
        let max_retries = 3;
        let initial_backoff_ms = 200;
        let max_backoff_ms = 5000;

        let provider = StarknetProvider::new_with_retry_config(
            rpc_url,
            max_retries,
            initial_backoff_ms,
            max_backoff_ms,
        );

        assert!(provider.is_ok());
        let provider = provider.unwrap();

        assert_eq!(provider.rpc_url(), rpc_url);
        assert_eq!(provider.max_retries, max_retries);
        assert_eq!(provider.initial_backoff_ms, initial_backoff_ms);
        assert_eq!(provider.max_backoff_ms, max_backoff_ms);
    }

    #[tokio::test]
    async fn test_with_retry_success_first_attempt() {
        let rpc_url = "http://localhost:5050";
        let provider = StarknetProvider::new(rpc_url).unwrap();

        let result = provider
            .with_retry("test_operation", || async { Ok::<i32, eyre::Report>(42) })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_retry_success_after_retries() {
        let rpc_url = "http://localhost:5050";
        let provider = StarknetProvider::new_with_retry_config(
            rpc_url, 5,  // max retries
            10, // very short backoff for testing
            50, // max backoff
        )
        .unwrap();

        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = provider
            .with_retry("test_operation", || {
                let attempts = attempts_clone.clone();
                async move {
                    let current = attempts.fetch_add(1, Ordering::SeqCst);

                    // Fail on first two attempts, succeed on third
                    if current < 2 {
                        Err(eyre::eyre!("Simulated failure"))
                    } else {
                        Ok(current as i32)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2); // Third attempt (0-indexed)
        assert_eq!(attempts.load(Ordering::SeqCst), 3); // Three attempts total
    }

    #[tokio::test]
    async fn test_with_retry_max_retries_exceeded() {
        let rpc_url = "http://localhost:5050";
        let max_retries = 3;
        let provider = StarknetProvider::new_with_retry_config(
            rpc_url,
            max_retries,
            10, // very short backoff for testing
            50, // max backoff
        )
        .unwrap();

        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: Result<i32, _> = provider
            .with_retry("test_operation", || {
                let attempts = attempts_clone.clone();
                async move {
                    let _ = attempts.fetch_add(1, Ordering::SeqCst);
                    // Always fail
                    Err(eyre::eyre!("Simulated failure"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), max_retries as u32);
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

    // TODO: To properly test the provider methods like get_latest_mmr_block,
    // we would need to either use mocking frameworks that can mock HTTP responses
    // or create a test server that returns expected responses.
    // For now, we're testing the retry mechanism separately.
}
