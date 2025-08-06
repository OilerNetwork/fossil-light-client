use std::time::Duration;

use tracing::{error, info, warn};

use crate::{
    error::{NetworkError, RelayerError},
    network::EthereumProviderConfig,
    reliability::{CircuitBreakerConfig, RetryConfig},
};

/// Error recovery strategies
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Retry with exponential backoff
    Retry(RetryConfig),
    /// Circuit breaker protection
    CircuitBreaker(CircuitBreakerConfig),
    /// Failover to alternative configuration
    Failover(Vec<EthereumProviderConfig>),
    /// Degraded mode operation
    DegradedMode,
    /// Complete failure - stop operation
    Fail,
}

/// Error recovery coordinator
#[derive(Debug)]
pub struct ErrorRecoveryCoordinator {
    strategies: Vec<RecoveryStrategy>,
    current_strategy_index: usize,
    failure_count: usize,
    last_success: Option<std::time::Instant>,
}

impl ErrorRecoveryCoordinator {
    /// Create a new error recovery coordinator
    pub const fn new(strategies: Vec<RecoveryStrategy>) -> Self {
        Self {
            strategies,
            current_strategy_index: 0,
            failure_count: 0,
            last_success: None,
        }
    }

    /// Create default recovery strategies
    pub fn default_strategies() -> Vec<RecoveryStrategy> {
        vec![
            RecoveryStrategy::Retry(
                RetryConfig::new()
                    .with_max_attempts(3)
                    .with_base_delay(Duration::from_millis(500))
                    .with_max_delay(Duration::from_secs(5)),
            ),
            RecoveryStrategy::CircuitBreaker(
                CircuitBreakerConfig::new()
                    .with_failure_threshold(5)
                    .with_recovery_timeout(Duration::from_secs(30)),
            ),
            RecoveryStrategy::DegradedMode,
            RecoveryStrategy::Fail,
        ]
    }

    /// Handle an error and determine recovery action
    pub async fn handle_error(&mut self, error: &RelayerError) -> RecoveryAction {
        self.failure_count += 1;

        let recovery_action = self.determine_recovery_action(error);

        match &recovery_action {
            RecoveryAction::Retry { strategy, delay } => {
                info!(
                    error = %error,
                    strategy = ?strategy,
                    delay_ms = delay.as_millis(),
                    failure_count = self.failure_count,
                    "Attempting error recovery with retry"
                );
            }
            RecoveryAction::Failover { config } => {
                warn!(
                    error = %error,
                    config = ?config,
                    failure_count = self.failure_count,
                    "Attempting error recovery with failover"
                );
            }
            RecoveryAction::DegradedMode => {
                warn!(
                    error = %error,
                    failure_count = self.failure_count,
                    "Entering degraded mode due to persistent errors"
                );
            }
            RecoveryAction::Fail => {
                error!(
                    error = %error,
                    failure_count = self.failure_count,
                    "All recovery strategies exhausted, failing operation"
                );
            }
        }

        recovery_action
    }

    /// Record a successful operation
    pub fn record_success(&mut self) {
        if self.failure_count > 0 {
            info!(
                previous_failure_count = self.failure_count,
                "Operation recovered successfully"
            );
        }

        self.failure_count = 0;
        self.current_strategy_index = 0;
        self.last_success = Some(std::time::Instant::now());
    }

    /// Determine the appropriate recovery action for the given error
    fn determine_recovery_action(&mut self, error: &RelayerError) -> RecoveryAction {
        // Check if error is recoverable at all
        if !self.is_recoverable_error(error) {
            return RecoveryAction::Fail;
        }

        // Try current strategy
        if let Some(strategy) = self.strategies.get(self.current_strategy_index) {
            match strategy {
                RecoveryStrategy::Retry(config) => {
                    if self.failure_count <= config.max_attempts {
                        let delay = self.calculate_retry_delay(config);
                        return RecoveryAction::Retry {
                            strategy: config.clone(),
                            delay,
                        };
                    }
                }
                RecoveryStrategy::CircuitBreaker(_config) => {
                    // Circuit breaker is handled at the network layer
                    // If we reach here, the circuit is open
                    self.advance_to_next_strategy();
                    return self.determine_recovery_action(error);
                }
                RecoveryStrategy::Failover(configs) => {
                    if let Some(config) = configs.first() {
                        return RecoveryAction::Failover {
                            config: config.clone(),
                        };
                    }
                }
                RecoveryStrategy::DegradedMode => {
                    return RecoveryAction::DegradedMode;
                }
                RecoveryStrategy::Fail => {
                    return RecoveryAction::Fail;
                }
            }
        }

        // Move to next strategy
        self.advance_to_next_strategy();

        // Try next strategy if available
        if self.current_strategy_index < self.strategies.len() {
            self.determine_recovery_action(error)
        } else {
            RecoveryAction::Fail
        }
    }

    /// Check if an error is recoverable
    fn is_recoverable_error(&self, error: &RelayerError) -> bool {
        match error {
            RelayerError::Network(network_error) => match network_error {
                NetworkError::ConnectionFailed(_)
                | NetworkError::ProviderError(_)
                | NetworkError::Timeout { .. } => true,
                NetworkError::TransactionFailed(msg) => {
                    // Some transaction failures are recoverable
                    msg.contains("timeout")
                        || msg.contains("nonce")
                        || msg.contains("gas")
                        || msg.contains("network")
                        || msg.contains("connection")
                        || msg.contains("temporary")
                }
                NetworkError::InvalidResponse(_) => false,
            },
            RelayerError::Config(_) | RelayerError::Contract(_) | RelayerError::Environment(_) => {
                false
            } // Config, contract, and environment errors are not recoverable
            RelayerError::Io(io_error) => {
                // Some IO errors are recoverable
                matches!(
                    io_error.kind(),
                    std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                )
            }
        }
    }

    /// Calculate retry delay based on failure count and configuration
    fn calculate_retry_delay(&self, config: &RetryConfig) -> Duration {
        if self.failure_count == 0 {
            return config.base_delay;
        }

        let delay = config.base_delay.as_millis() as f64
            * config
                .multiplier
                .powi(self.failure_count.saturating_sub(1) as i32);

        let delay = Duration::from_millis(delay as u64);
        std::cmp::min(delay, config.max_delay)
    }

    /// Advance to the next recovery strategy
    fn advance_to_next_strategy(&mut self) {
        self.current_strategy_index += 1;
        info!(
            current_strategy_index = self.current_strategy_index,
            total_strategies = self.strategies.len(),
            "Advanced to next recovery strategy"
        );
    }

    /// Get current recovery statistics
    pub const fn get_stats(&self) -> RecoveryStats {
        RecoveryStats {
            failure_count: self.failure_count,
            current_strategy_index: self.current_strategy_index,
            last_success: self.last_success,
            total_strategies: self.strategies.len(),
        }
    }
}

impl Default for ErrorRecoveryCoordinator {
    fn default() -> Self {
        Self::new(Self::default_strategies())
    }
}

/// Recovery action to take in response to an error
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Retry the operation with the given strategy and delay
    Retry {
        /// Retry strategy configuration
        strategy: RetryConfig,
        /// Delay before retry
        delay: Duration,
    },
    /// Failover to alternative configuration
    Failover {
        /// Failover configuration
        config: EthereumProviderConfig,
    },
    /// Enter degraded mode (limited functionality)
    DegradedMode,
    /// Fail the operation completely
    Fail,
}

/// Recovery statistics
#[derive(Debug, Clone)]
pub struct RecoveryStats {
    /// Current failure count
    pub failure_count: usize,
    /// Current strategy index
    pub current_strategy_index: usize,
    /// Last successful operation timestamp
    pub last_success: Option<std::time::Instant>,
    /// Total number of strategies available
    pub total_strategies: usize,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::error::ConfigError;

    #[tokio::test]
    async fn test_error_recovery_coordinator_retry() {
        let mut coordinator = ErrorRecoveryCoordinator::new(vec![RecoveryStrategy::Retry(
            RetryConfig::new()
                .with_max_attempts(3)
                .with_base_delay(Duration::from_millis(100)),
        )]);

        let error = RelayerError::Network(NetworkError::ConnectionFailed("test".to_string()));

        // First error should trigger retry
        let action = coordinator.handle_error(&error).await;
        match action {
            RecoveryAction::Retry { strategy, delay } => {
                assert_eq!(strategy.max_attempts, 3);
                assert_eq!(delay, Duration::from_millis(100));
            }
            _ => panic!("Expected retry action"),
        }

        assert_eq!(coordinator.failure_count, 1);
    }

    #[tokio::test]
    async fn test_error_recovery_coordinator_non_recoverable() {
        let mut coordinator = ErrorRecoveryCoordinator::default();

        let error = RelayerError::Config(ConfigError::InvalidPrivateKey("test".to_string()));

        // Non-recoverable error should fail immediately
        let action = coordinator.handle_error(&error).await;
        match action {
            RecoveryAction::Fail => {}
            _ => panic!("Expected fail action for non-recoverable error"),
        }
    }

    #[tokio::test]
    async fn test_error_recovery_coordinator_success_reset() {
        let mut coordinator = ErrorRecoveryCoordinator::default();

        let error = RelayerError::Network(NetworkError::ConnectionFailed("test".to_string()));

        // Trigger some failures
        coordinator.handle_error(&error).await;
        coordinator.handle_error(&error).await;
        assert_eq!(coordinator.failure_count, 2);

        // Record success should reset failure count
        coordinator.record_success();
        assert_eq!(coordinator.failure_count, 0);
        assert_eq!(coordinator.current_strategy_index, 0);
        assert!(coordinator.last_success.is_some());
    }

    #[tokio::test]
    async fn test_recovery_strategy_progression() {
        let mut coordinator = ErrorRecoveryCoordinator::new(vec![
            RecoveryStrategy::Retry(
                RetryConfig::new().with_max_attempts(1), // Only 1 attempt
            ),
            RecoveryStrategy::DegradedMode,
        ]);

        let error = RelayerError::Network(NetworkError::ConnectionFailed("test".to_string()));

        // First error - should retry
        let action = coordinator.handle_error(&error).await;
        match action {
            RecoveryAction::Retry { .. } => {}
            _ => panic!("Expected retry action"),
        }

        // Second error - should move to degraded mode
        let action = coordinator.handle_error(&error).await;
        match action {
            RecoveryAction::DegradedMode => {}
            _ => panic!("Expected degraded mode action"),
        }
    }

    #[test]
    fn test_is_recoverable_error() {
        let coordinator = ErrorRecoveryCoordinator::default();

        // Recoverable errors
        assert!(coordinator.is_recoverable_error(&RelayerError::Network(
            NetworkError::ConnectionFailed("test".to_string())
        )));
        assert!(coordinator.is_recoverable_error(&RelayerError::Network(
            NetworkError::TransactionFailed("timeout".to_string())
        )));
        assert!(
            coordinator.is_recoverable_error(&RelayerError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "test"
            )))
        );

        // Non-recoverable errors
        assert!(!coordinator.is_recoverable_error(&RelayerError::Config(
            ConfigError::InvalidPrivateKey("test".to_string())
        )));
        assert!(!coordinator.is_recoverable_error(&RelayerError::Network(
            NetworkError::InvalidResponse("test".to_string())
        )));
        assert!(!coordinator.is_recoverable_error(&RelayerError::Contract("test".to_string())));
    }
}
