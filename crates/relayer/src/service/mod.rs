use std::sync::Arc;

use alloy::network::EthereumWallet;
use tracing::{info, info_span, Instrument};

use crate::{
    config::RelayerConfig,
    error::Result,
    network::{
        AlloyEthereumProvider, EthereumProviderConfig, L1MessageSender, L1MessageSenderContract,
    },
    observability::{checks, HealthChecker, MetricsTimer, RelayerMetrics},
};

/// Legacy relayer wrapper for backward compatibility
pub mod relayer;

pub use relayer::*;

/// Core relayer service that orchestrates block hash relaying
#[derive(Debug)]
pub struct RelayerService {
    config: RelayerConfig,
    contract: Box<dyn L1MessageSender>,
    metrics: Arc<RelayerMetrics>,
    health_checker: Arc<HealthChecker>,
}

impl RelayerService {
    /// Create a new relayer service from configuration
    pub async fn new(config: RelayerConfig) -> Result<Self> {
        Self::new_with_provider_config(config, EthereumProviderConfig::default()).await
    }

    /// Create a new relayer service with custom provider configuration
    pub async fn new_with_provider_config(
        config: RelayerConfig,
        provider_config: EthereumProviderConfig,
    ) -> Result<Self> {
        let span = info_span!("relayer_service_initialization");
        async move {
            // Initialize observability components
            let metrics = Arc::new(RelayerMetrics::new());
            let health_checker = Arc::new(HealthChecker::new());

            // Initialize metrics descriptions
            RelayerMetrics::init_descriptions();

            // Create wallet from config
            let wallet = EthereumWallet::from(config.private_key.clone());

            // Create Ethereum provider with reliability features
            let provider = AlloyEthereumProvider::new_with_config(
                wallet,
                &config.eth_rpc_url,
                provider_config,
            )
            .await?;

            // Create contract instance
            let contract = Box::new(L1MessageSenderContract::new(
                Box::new(provider),
                config.l1_message_sender,
            ));

            // Initial health checks
            let config_health = checks::configuration_health();
            health_checker.update_component_health(config_health);

            let network_health = checks::network_connectivity_health(&config.eth_rpc_url).await;
            health_checker.update_component_health(network_health);

            info!(
                l2_recipient_addr = %config.l2_recipient_addr,
                eth_rpc_url = %config.eth_rpc_url,
                l1_message_sender = %config.l1_message_sender,
                "Relayer service initialized successfully with observability features"
            );

            Ok(Self {
                config,
                contract,
                metrics,
                health_checker,
            })
        }
        .instrument(span)
        .await
    }

    /// Send finalized block hash to L2
    #[tracing::instrument(skip(self), fields(
        l2_recipient = %self.config.l2_recipient_addr,
        transaction_value = %self.config.transaction_value,
        confirmations = self.config.required_confirmations
    ))]
    pub async fn send_finalized_block_hash_to_l2(&self) -> Result<()> {
        let timer = MetricsTimer::start("relayer_transaction");

        info!(
            l2_recipient_addr = %self.config.l2_recipient_addr,
            transaction_value = %self.config.transaction_value,
            confirmation_timeout = ?self.config.confirmation_timeout,
            required_confirmations = self.config.required_confirmations,
            "Starting block hash relay to L2"
        );

        // Update uptime metric
        self.metrics.update_uptime();

        let result = self
            .contract
            .send_finalized_block_hash(
                self.config.l2_recipient_addr,
                self.config.transaction_value,
                self.config.confirmation_timeout,
                self.config.required_confirmations,
            )
            .await;

        let duration = timer.finish();

        match &result {
            Ok(tx_hash) => {
                self.metrics.record_transaction_success();
                self.metrics.record_transaction_duration(duration);

                info!(
                    tx_hash = ?tx_hash,
                    duration_ms = duration.as_millis(),
                    "Successfully relayed block hash to L2"
                );
            }
            Err(error) => {
                let error_type = match error {
                    crate::error::RelayerError::Network(_) => "network",
                    crate::error::RelayerError::Config(_) => "config",
                    crate::error::RelayerError::Contract(_) => "contract",
                    crate::error::RelayerError::Environment(_) => "environment",
                    crate::error::RelayerError::Io(_) => "io",
                };

                self.metrics.record_transaction_failure(error_type);

                tracing::error!(
                    error = %error,
                    error_type = error_type,
                    duration_ms = duration.as_millis(),
                    "Failed to relay block hash to L2"
                );
            }
        }

        result.map(|_| ())
    }

    /// Get metrics snapshot
    pub fn get_metrics(&self) -> Arc<RelayerMetrics> {
        self.metrics.clone()
    }

    /// Get health checker
    pub fn get_health_checker(&self) -> Arc<HealthChecker> {
        self.health_checker.clone()
    }

    /// Get current system health status
    pub fn get_health_status(&self) -> crate::observability::SystemHealth {
        self.health_checker.get_system_health()
    }
}
