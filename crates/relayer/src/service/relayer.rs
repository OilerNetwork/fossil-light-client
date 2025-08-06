use std::sync::Arc;

use crate::{
    config::RelayerConfig,
    error::Result,
    observability::{HealthChecker, RelayerMetrics, SystemHealth},
    service::RelayerService,
};

/// Legacy Relayer struct to maintain backward compatibility
/// This is a thin wrapper around `RelayerService`
#[derive(Debug)]
pub struct Relayer {
    service: RelayerService,
}

impl Relayer {
    /// Create a new relayer instance from environment variables
    pub async fn new() -> Result<Self> {
        let config = RelayerConfig::from_env()?;
        let service = RelayerService::new(config).await?;

        Ok(Self { service })
    }

    /// Create a new relayer instance from configuration
    pub async fn from_config(config: RelayerConfig) -> Result<Self> {
        let service = RelayerService::new(config).await?;
        Ok(Self { service })
    }

    /// Send finalized block hash to L2
    pub async fn send_finalized_block_hash_to_l2(&self) -> Result<()> {
        self.service.send_finalized_block_hash_to_l2().await
    }

    /// Get metrics snapshot
    pub fn get_metrics(&self) -> Arc<RelayerMetrics> {
        self.service.get_metrics()
    }

    /// Get health checker
    pub fn get_health_checker(&self) -> Arc<HealthChecker> {
        self.service.get_health_checker()
    }

    /// Get current system health status
    pub fn get_health_status(&self) -> SystemHealth {
        self.service.get_health_status()
    }
}
