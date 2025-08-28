use std::time::Duration;

use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use tokio_retry::strategy::ExponentialBackoff as RetryStrategy;

use crate::error::NetworkError;

/// Circuit breaker pattern implementation
pub mod circuit_breaker;
/// Error recovery strategies
pub mod recovery;
/// Retry mechanisms with exponential backoff
pub mod retry;

pub use circuit_breaker::*;
pub use recovery::*;
pub use retry::*;

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Base delay between retries
    pub base_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
    /// Maximum total time for all retries
    pub max_elapsed_time: Option<Duration>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            max_elapsed_time: Some(Duration::from_secs(30)),
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum number of attempts
    pub const fn with_max_attempts(mut self, max_attempts: usize) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Set base delay
    pub const fn with_base_delay(mut self, delay: Duration) -> Self {
        self.base_delay = delay;
        self
    }

    /// Set maximum delay
    pub const fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set multiplier for exponential backoff
    pub const fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Set maximum elapsed time for all retries
    pub const fn with_max_elapsed_time(mut self, duration: Duration) -> Self {
        self.max_elapsed_time = Some(duration);
        self
    }

    /// Create exponential backoff strategy
    pub fn to_backoff_strategy(&self) -> ExponentialBackoff {
        let mut backoff = ExponentialBackoffBuilder::new()
            .with_initial_interval(self.base_delay)
            .with_max_interval(self.max_delay)
            .with_multiplier(self.multiplier)
            .build();

        if let Some(max_elapsed) = self.max_elapsed_time {
            backoff.max_elapsed_time = Some(max_elapsed);
        }

        backoff
    }

    /// Create tokio-retry strategy
    pub fn to_retry_strategy(&self) -> impl Iterator<Item = Duration> {
        RetryStrategy::from_millis(self.base_delay.as_millis() as u64)
            .max_delay(self.max_delay)
            .take(self.max_attempts)
    }
}

/// Determine if an error is retryable
pub fn is_retryable_error(error: &crate::error::RelayerError) -> bool {
    match error {
        crate::error::RelayerError::Network(network_error) => match network_error {
            NetworkError::ConnectionFailed(_)
            | NetworkError::ProviderError(_)
            | NetworkError::Timeout { .. } => true,
            NetworkError::TransactionFailed(msg) => {
                // Retry on specific transaction failures that might be temporary
                msg.contains("timeout")
                    || msg.contains("nonce")
                    || msg.contains("gas")
                    || msg.contains("network")
                    || msg.contains("connection")
            }
            NetworkError::InvalidResponse(_) => false, // Don't retry on invalid responses
        },
        crate::error::RelayerError::Config(_)
        | crate::error::RelayerError::Contract(_)
        | crate::error::RelayerError::Environment(_) => false, // Non-retryable errors
        crate::error::RelayerError::Io(_) => true, // IO errors might be temporary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{NetworkError, RelayerError};

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.base_delay, Duration::from_millis(100));
        assert_eq!(config.max_delay, Duration::from_secs(10));
        assert_eq!(config.multiplier, 2.0);
        assert_eq!(config.max_elapsed_time, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::new()
            .with_max_attempts(5)
            .with_base_delay(Duration::from_millis(200))
            .with_max_delay(Duration::from_secs(20))
            .with_multiplier(1.5)
            .with_max_elapsed_time(Duration::from_secs(60));

        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.base_delay, Duration::from_millis(200));
        assert_eq!(config.max_delay, Duration::from_secs(20));
        assert_eq!(config.multiplier, 1.5);
        assert_eq!(config.max_elapsed_time, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_is_retryable_error() {
        // Retryable errors
        assert!(is_retryable_error(&RelayerError::Network(
            NetworkError::ConnectionFailed("test".to_string())
        )));
        assert!(is_retryable_error(&RelayerError::Network(
            NetworkError::TransactionFailed("timeout".to_string())
        )));
        assert!(is_retryable_error(&RelayerError::Network(
            NetworkError::ProviderError("test".to_string())
        )));
        assert!(is_retryable_error(&RelayerError::Network(
            NetworkError::Timeout {
                timeout: Duration::from_secs(30),
                operation: "test".to_string()
            }
        )));
        assert!(is_retryable_error(&RelayerError::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "test"
        ))));

        // Non-retryable errors
        assert!(!is_retryable_error(&RelayerError::Network(
            NetworkError::InvalidResponse("test".to_string())
        )));
        assert!(!is_retryable_error(&RelayerError::Config(
            crate::error::ConfigError::InvalidPrivateKey("test".to_string())
        )));
        assert!(!is_retryable_error(&RelayerError::Contract(
            "test".to_string()
        )));
        assert!(!is_retryable_error(&RelayerError::Environment(
            std::env::VarError::NotPresent
        )));
    }
}
