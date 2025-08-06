use std::future::Future;

use backoff::backoff::Backoff;
use metrics::{counter, histogram};
use tracing::{debug, warn};

use super::{is_retryable_error, RetryConfig};
use crate::error::Result;

/// Retry a fallible async operation with exponential backoff
pub async fn retry_with_backoff<F, Fut, T>(
    mut operation: F,
    config: &RetryConfig,
    operation_name: &str,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut backoff = config.to_backoff_strategy();
    let mut attempt = 0;

    loop {
        attempt += 1;

        debug!(
            operation = operation_name,
            attempt = attempt,
            max_attempts = config.max_attempts,
            "Attempting operation"
        );

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!(
                        operation = operation_name,
                        attempt = attempt,
                        "Operation succeeded after retry"
                    );
                }

                // Record success metrics
                counter!(
                    "relayer_retries_total",
                    "operation" => operation_name.to_string(),
                    "final_attempt" => attempt.to_string(),
                    "outcome" => "success"
                )
                .increment(1);

                return Ok(result);
            }
            Err(error) => {
                if attempt >= config.max_attempts || !is_retryable_error(&error) {
                    warn!(
                        operation = operation_name,
                        attempt = attempt,
                        error = %error,
                        "Operation failed permanently"
                    );

                    // Record permanent failure metrics
                    counter!(
                        "relayer_retries_total",
                        "operation" => operation_name.to_string(),
                        "final_attempt" => attempt.to_string(),
                        "outcome" => "permanent_failure"
                    )
                    .increment(1);

                    return Err(error);
                }

                if let Some(delay) = backoff.next_backoff() {
                    warn!(
                        operation = operation_name,
                        attempt = attempt,
                        error = %error,
                        delay_ms = delay.as_millis(),
                        "Operation failed, retrying after delay"
                    );

                    // Record retry attempt
                    counter!(
                        "relayer_retries_total",
                        "operation" => operation_name.to_string(),
                        "attempt" => attempt.to_string(),
                        "outcome" => "retry"
                    )
                    .increment(1);

                    // Record backoff duration
                    histogram!("relayer_retry_backoff_duration_seconds")
                        .record(delay.as_secs_f64());

                    tokio::time::sleep(delay).await;
                } else {
                    warn!(
                        operation = operation_name,
                        attempt = attempt,
                        error = %error,
                        "Backoff strategy exhausted"
                    );

                    // Record backoff exhaustion
                    counter!(
                        "relayer_retries_total",
                        "operation" => operation_name.to_string(),
                        "final_attempt" => attempt.to_string(),
                        "outcome" => "backoff_exhausted"
                    )
                    .increment(1);

                    return Err(error);
                }
            }
        }
    }
}

/// Retry wrapper for network operations
pub struct RetryableOperation<F> {
    operation: F,
    config: RetryConfig,
    operation_name: String,
}

impl<F> RetryableOperation<F> {
    /// Create a new retryable operation
    pub fn new(operation: F, operation_name: impl Into<String>) -> Self {
        Self {
            operation,
            config: RetryConfig::default(),
            operation_name: operation_name.into(),
        }
    }

    /// Set retry configuration
    pub const fn with_config(mut self, config: RetryConfig) -> Self {
        self.config = config;
        self
    }

    /// Execute the operation with retry logic
    pub async fn execute<Fut, T>(self) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        retry_with_backoff(self.operation, &self.config, &self.operation_name).await
    }
}

/// Helper function to create a retryable operation from a closure
pub fn retryable<F, Fut, T>(operation: F) -> RetryableOperation<F>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    RetryableOperation::new(operation, "retryable_operation")
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use super::*;
    use crate::error::{NetworkError, RelayerError};

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let operation = || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok::<i32, RelayerError>(42)
            }
        };

        let config = RetryConfig::new().with_max_attempts(3);
        let result = retry_with_backoff(operation, &config, "test_operation").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let operation = || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
                if count < 3 {
                    Err(RelayerError::Network(NetworkError::ConnectionFailed(
                        "test".to_string(),
                    )))
                } else {
                    Ok::<i32, RelayerError>(42)
                }
            }
        };

        let config = RetryConfig::new()
            .with_max_attempts(3)
            .with_base_delay(Duration::from_millis(1)); // Fast test

        let result = retry_with_backoff(operation, &config, "test_operation").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let operation = || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err::<i32, RelayerError>(RelayerError::Network(NetworkError::ConnectionFailed(
                    "test".to_string(),
                )))
            }
        };

        let config = RetryConfig::new()
            .with_max_attempts(2)
            .with_base_delay(Duration::from_millis(1)); // Fast test

        let result = retry_with_backoff(operation, &config, "test_operation").await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let operation = || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err::<i32, RelayerError>(RelayerError::Config(
                    crate::error::ConfigError::InvalidPrivateKey("test".to_string()),
                ))
            }
        };

        let config = RetryConfig::new().with_max_attempts(3);
        let result = retry_with_backoff(operation, &config, "test_operation").await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Should not retry
    }

    #[tokio::test]
    async fn test_retryable_operation_builder() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let operation = || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
                if count < 2 {
                    Err(RelayerError::Network(NetworkError::ConnectionFailed(
                        "test".to_string(),
                    )))
                } else {
                    Ok::<i32, RelayerError>(42)
                }
            }
        };

        let config = RetryConfig::new()
            .with_max_attempts(3)
            .with_base_delay(Duration::from_millis(1));

        let result = RetryableOperation::new(operation, "test_operation")
            .with_config(config)
            .execute()
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
