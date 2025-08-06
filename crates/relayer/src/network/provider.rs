use std::{sync::Arc, time::Duration};

use alloy::{
    network::EthereumWallet,
    primitives::{Address, TxHash, U256},
    providers::ProviderBuilder,
};
use async_trait::async_trait;
use tracing::{debug, info};

use super::{EthereumProvider, L1MessagesSender};
use crate::{
    error::{NetworkError, Result},
    reliability::{retry_with_backoff, CircuitBreaker, CircuitBreakerConfig, RetryConfig},
};

/// Configuration for the Ethereum provider
#[derive(Debug, Clone)]
pub struct EthereumProviderConfig {
    /// Retry configuration for operations
    pub retry_config: RetryConfig,
    /// Circuit breaker configuration
    pub circuit_breaker_config: CircuitBreakerConfig,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Connection timeout
    pub connect_timeout: Duration,
}

impl Default for EthereumProviderConfig {
    fn default() -> Self {
        Self {
            retry_config: RetryConfig::new()
                .with_max_attempts(3)
                .with_base_delay(Duration::from_millis(500))
                .with_max_delay(Duration::from_secs(5)),
            circuit_breaker_config: CircuitBreakerConfig::new()
                .with_failure_threshold(5)
                .with_recovery_timeout(Duration::from_secs(30))
                .with_request_timeout(Duration::from_secs(30)),
            max_connections: 10,
            connect_timeout: Duration::from_secs(10),
        }
    }
}

impl EthereumProviderConfig {
    /// Create new provider configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set retry configuration
    pub const fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Set circuit breaker configuration
    pub const fn with_circuit_breaker_config(mut self, config: CircuitBreakerConfig) -> Self {
        self.circuit_breaker_config = config;
        self
    }

    /// Set maximum connections
    pub const fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// Set connection timeout
    pub const fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }
}

/// Implementation of `EthereumProvider` using Alloy with reliability features
pub struct AlloyEthereumProvider {
    wallet: EthereumWallet,
    rpc_url: String,
    config: EthereumProviderConfig,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl AlloyEthereumProvider {
    /// Create a new Ethereum provider with default configuration
    pub async fn new(wallet: EthereumWallet, rpc_url: &str) -> Result<Self> {
        Self::new_with_config(wallet, rpc_url, EthereumProviderConfig::default()).await
    }

    /// Create a new Ethereum provider with custom configuration
    pub async fn new_with_config(
        wallet: EthereumWallet,
        rpc_url: &str,
        config: EthereumProviderConfig,
    ) -> Result<Self> {
        info!(
            rpc_url = %rpc_url,
            max_connections = config.max_connections,
            connect_timeout_ms = config.connect_timeout.as_millis(),
            "Configuring Ethereum provider"
        );

        let circuit_breaker = Arc::new(CircuitBreaker::new(
            config.circuit_breaker_config.clone(),
            format!("ethereum_provider_{rpc_url}"),
        ));

        Ok(Self {
            wallet,
            rpc_url: rpc_url.to_string(),
            config,
            circuit_breaker,
        })
    }
}

#[async_trait]
impl EthereumProvider for AlloyEthereumProvider {
    async fn send_transaction(
        &self,
        contract_address: Address,
        recipient: U256,
        value: U256,
        timeout: Duration,
        confirmations: u64,
    ) -> Result<TxHash> {
        let circuit_breaker = self.circuit_breaker.clone();
        let provider_config = self.config.clone();
        let wallet = self.wallet.clone();
        let rpc_url = self.rpc_url.clone();

        // Execute transaction through circuit breaker
        circuit_breaker
            .call(|| async {
                let operation = || async {
                    // Create provider connection
                    let provider = ProviderBuilder::new()
                        .wallet(wallet.clone())
                        .connect(&rpc_url)
                        .await
                        .map_err(|e| {
                            NetworkError::ConnectionFailed(format!(
                                "Failed to connect to {rpc_url}: {e}"
                            ))
                        })?;

                    let contract = L1MessagesSender::new(contract_address, &provider);

                    info!(
                        contract_address = %contract_address,
                        recipient = %recipient,
                        value = %value,
                        "Sending transaction to L1MessagesSender contract"
                    );

                    let call_builder = contract.sendFinalizedBlockHashToL2(recipient).value(value);

                    // Send transaction with retry logic
                    let pending_tx = call_builder.send().await.map_err(|e| {
                        NetworkError::TransactionFailed(format!("Failed to send transaction: {e}"))
                    })?;

                    debug!(
                        tx_hash = ?pending_tx.tx_hash(),
                        confirmations = confirmations,
                        timeout_ms = timeout.as_millis(),
                        "Waiting for transaction confirmation"
                    );

                    // Wait for confirmations with timeout
                    let tx_hash = pending_tx
                        .with_required_confirmations(confirmations)
                        .with_timeout(Some(timeout))
                        .watch()
                        .await
                        .map_err(|e| {
                            if e.to_string().contains("timeout") {
                                NetworkError::Timeout {
                                    timeout,
                                    operation: format!("Transaction confirmation: {e}"),
                                }
                            } else {
                                NetworkError::TransactionFailed(format!(
                                    "Transaction failed during confirmation: {e}"
                                ))
                            }
                        })?;

                    info!(
                        tx_hash = ?tx_hash,
                        confirmations = confirmations,
                        "Transaction confirmed successfully"
                    );

                    Ok(tx_hash)
                };

                retry_with_backoff(
                    operation,
                    &provider_config.retry_config,
                    "send_ethereum_transaction",
                )
                .await
            })
            .await
    }
}

// We need to implement Debug manually since the circuit breaker contains complex types
impl std::fmt::Debug for AlloyEthereumProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlloyEthereumProvider")
            .field("rpc_url", &self.rpc_url)
            .field("config", &self.config)
            .field("circuit_breaker_state", &self.circuit_breaker.get_state())
            .finish()
    }
}
