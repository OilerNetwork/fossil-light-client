use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Health status of a component
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Component is healthy and operating normally
    Healthy,
    /// Component is degraded but still functional
    Degraded,
    /// Component is unhealthy and not functioning properly
    Unhealthy,
    /// Component status is unknown or not checked recently
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Health check result for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Name of the component being checked
    pub component: String,
    /// Current health status
    pub status: HealthStatus,
    /// Optional message providing more details
    pub message: Option<String>,
    /// Timestamp of the last health check (skipped in serialization)
    #[serde(skip_serializing, skip_deserializing, default = "Instant::now")]
    pub last_checked: Instant,
    /// Duration of the last health check in milliseconds
    pub check_duration_ms: Option<u64>,
    /// Additional metadata about the component
    pub metadata: HashMap<String, String>,
}

impl HealthCheck {
    /// Create a new health check result
    pub fn new(component: impl Into<String>, status: HealthStatus) -> Self {
        Self {
            component: component.into(),
            status,
            message: None,
            last_checked: Instant::now(),
            check_duration_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a message to the health check
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Add check duration
    pub const fn with_duration(mut self, duration: Duration) -> Self {
        self.check_duration_ms = Some(duration.as_millis() as u64);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if the health check is stale (older than specified duration)
    pub fn is_stale(&self, max_age: Duration) -> bool {
        self.last_checked.elapsed() > max_age
    }
}

/// Overall health status of the relayer system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Overall system status
    pub status: HealthStatus,
    /// Individual component health checks
    pub components: HashMap<String, HealthCheck>,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// Timestamp of the health report (skipped in serialization)
    #[serde(skip_serializing, skip_deserializing, default = "Instant::now")]
    pub timestamp: Instant,
}

impl SystemHealth {
    /// Determine overall system status from component statuses
    pub fn calculate_overall_status(&mut self) {
        let mut has_unhealthy = false;
        let mut has_degraded = false;

        for health_check in self.components.values() {
            match health_check.status {
                HealthStatus::Unhealthy => has_unhealthy = true,
                HealthStatus::Degraded | HealthStatus::Unknown => has_degraded = true, /* Treat unknown as degraded */
                HealthStatus::Healthy => {}
            }
        }

        self.status = if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else if self.components.is_empty() {
            HealthStatus::Unknown
        } else {
            HealthStatus::Healthy
        };
    }
}

/// Health checker for monitoring system components
#[derive(Debug)]
pub struct HealthChecker {
    /// Component health checks
    checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    /// System start time for uptime calculation
    start_time: Instant,
    /// Maximum age for health checks before they're considered stale
    max_check_age: Duration,
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new() -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            max_check_age: Duration::from_secs(300), // 5 minutes default
        }
    }

    /// Create a health checker with custom max check age
    pub fn with_max_check_age(max_age: Duration) -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            max_check_age: max_age,
        }
    }

    /// Register a health check result for a component
    pub fn update_component_health(&self, health_check: HealthCheck) {
        let component_name = health_check.component.clone();

        debug!(
            component = %component_name,
            status = %health_check.status,
            message = ?health_check.message,
            "Updating component health"
        );

        if let Ok(mut checks) = self.checks.write() {
            checks.insert(component_name, health_check);
        } else {
            warn!("Failed to acquire write lock for health checks");
        }
    }

    /// Get health status for a specific component
    pub fn get_component_health(&self, component: &str) -> Option<HealthCheck> {
        self.checks.read().ok()?.get(component).cloned()
    }

    /// Get overall system health
    pub fn get_system_health(&self) -> SystemHealth {
        let components = self
            .checks
            .read()
            .map(|checks| checks.clone())
            .unwrap_or_default();

        let mut system_health = SystemHealth {
            status: HealthStatus::Unknown,
            components,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            timestamp: Instant::now(),
        };

        system_health.calculate_overall_status();

        // Mark stale checks as unknown
        let stale_components: Vec<String> = system_health
            .components
            .iter()
            .filter(|(_, check)| check.is_stale(self.max_check_age))
            .map(|(name, _)| name.clone())
            .collect();

        for component in stale_components {
            if let Some(check) = system_health.components.get_mut(&component) {
                warn!(
                    component = %component,
                    last_checked = ?check.last_checked,
                    max_age = ?self.max_check_age,
                    "Health check is stale, marking as unknown"
                );
                check.status = HealthStatus::Unknown;
                check.message = Some("Health check is stale".to_string());
            }
        }

        // Recalculate overall status after marking stale checks
        system_health.calculate_overall_status();
        system_health
    }

    /// Remove a component from health monitoring
    pub fn remove_component(&self, component: &str) {
        if let Ok(mut checks) = self.checks.write() {
            checks.remove(component);
            debug!(component = %component, "Removed component from health monitoring");
        }
    }

    /// Clear all health checks
    pub fn clear_all(&self) {
        if let Ok(mut checks) = self.checks.write() {
            checks.clear();
            debug!("Cleared all health checks");
        }
    }

    /// Get list of all monitored components
    pub fn get_component_names(&self) -> Vec<String> {
        self.checks
            .read()
            .map(|checks| checks.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// Common health check implementations
pub mod checks {
    use std::sync::Arc;

    use super::*;
    use crate::reliability::CircuitBreaker;

    /// Check the health of a circuit breaker
    pub fn circuit_breaker_health(
        circuit_breaker: &Arc<CircuitBreaker>,
        name: &str,
    ) -> HealthCheck {
        let status = match circuit_breaker.get_state() {
            crate::reliability::CircuitState::Closed => HealthStatus::Healthy,
            crate::reliability::CircuitState::HalfOpen => HealthStatus::Degraded,
            crate::reliability::CircuitState::Open => HealthStatus::Unhealthy,
        };

        let failure_count = circuit_breaker.get_failure_count();
        let success_count = circuit_breaker.get_success_count();

        HealthCheck::new(format!("circuit_breaker_{name}"), status)
            .with_message(format!(
                "Circuit breaker state: {:?}, failures: {}, successes: {}",
                circuit_breaker.get_state(),
                failure_count,
                success_count
            ))
            .with_metadata("failure_count", failure_count.to_string())
            .with_metadata("success_count", success_count.to_string())
            .with_metadata("state", format!("{:?}", circuit_breaker.get_state()))
    }

    /// Create a health check for network connectivity
    pub async fn network_connectivity_health(url: &str) -> HealthCheck {
        let start = Instant::now();

        // Simple connectivity test - this could be enhanced with actual HTTP requests
        let status = if url.starts_with("http://") || url.starts_with("https://") {
            // For now, assume network is healthy if URL format is valid
            // In a real implementation, you might want to make an actual HTTP request
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };

        let duration = start.elapsed();

        HealthCheck::new("network_connectivity", status)
            .with_message(format!("Network connectivity to {url}"))
            .with_duration(duration)
            .with_metadata("target_url", url.to_string())
    }

    /// Create a health check for configuration validity
    pub fn configuration_health() -> HealthCheck {
        // Check if required environment variables are set
        let required_vars = [
            "ACCOUNT_PRIVATE_KEY",
            "L2_MSG_PROXY",
            "ETH_RPC_URL",
            "L1_MESSAGE_SENDER",
        ];

        let mut missing_vars = Vec::new();
        for var in &required_vars {
            if std::env::var(var).is_err() {
                missing_vars.push(*var);
            }
        }

        let (status, message) = if missing_vars.is_empty() {
            (
                HealthStatus::Healthy,
                "All required configuration is present".to_string(),
            )
        } else {
            (
                HealthStatus::Unhealthy,
                format!(
                    "Missing required environment variables: {}",
                    missing_vars.join(", ")
                ),
            )
        };

        HealthCheck::new("configuration", status)
            .with_message(message)
            .with_metadata("required_vars_count", required_vars.len().to_string())
            .with_metadata("missing_vars_count", missing_vars.len().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
        assert_eq!(HealthStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_health_check_creation() {
        let check = HealthCheck::new("test_component", HealthStatus::Healthy)
            .with_message("All systems operational")
            .with_metadata("version", "1.0.0");

        assert_eq!(check.component, "test_component");
        assert_eq!(check.status, HealthStatus::Healthy);
        assert_eq!(check.message, Some("All systems operational".to_string()));
        assert_eq!(check.metadata.get("version"), Some(&"1.0.0".to_string()));
    }

    #[test]
    fn test_system_health_calculation() {
        let mut system_health = SystemHealth {
            status: HealthStatus::Unknown,
            components: HashMap::new(),
            uptime_seconds: 100,
            timestamp: Instant::now(),
        };

        // All healthy components
        system_health.components.insert(
            "comp1".to_string(),
            HealthCheck::new("comp1", HealthStatus::Healthy),
        );
        system_health.components.insert(
            "comp2".to_string(),
            HealthCheck::new("comp2", HealthStatus::Healthy),
        );
        system_health.calculate_overall_status();
        assert_eq!(system_health.status, HealthStatus::Healthy);

        // One degraded component
        system_health.components.insert(
            "comp3".to_string(),
            HealthCheck::new("comp3", HealthStatus::Degraded),
        );
        system_health.calculate_overall_status();
        assert_eq!(system_health.status, HealthStatus::Degraded);

        // One unhealthy component
        system_health.components.insert(
            "comp4".to_string(),
            HealthCheck::new("comp4", HealthStatus::Unhealthy),
        );
        system_health.calculate_overall_status();
        assert_eq!(system_health.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_checker() {
        let checker = HealthChecker::new();

        // Add a healthy component
        let health_check = HealthCheck::new("test_service", HealthStatus::Healthy)
            .with_message("Service is running normally");
        checker.update_component_health(health_check);

        // Verify component health
        let component_health = checker.get_component_health("test_service");
        assert!(component_health.is_some());
        assert_eq!(component_health.unwrap().status, HealthStatus::Healthy);

        // Get system health
        let system_health = checker.get_system_health();
        assert_eq!(system_health.status, HealthStatus::Healthy);
        assert_eq!(system_health.components.len(), 1);
    }

    #[test]
    fn test_stale_health_checks() {
        let health_check = HealthCheck::new("test", HealthStatus::Healthy);

        // Fresh check should not be stale
        assert!(!health_check.is_stale(Duration::from_secs(60)));

        // Simulate old check
        let mut old_check = health_check;
        old_check.last_checked = Instant::now() - Duration::from_secs(120);
        assert!(old_check.is_stale(Duration::from_secs(60)));
    }
}
