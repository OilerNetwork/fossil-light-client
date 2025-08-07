//! Async utilities for the Fossil Light Client.
//!
//! This module provides utilities for handling async operations with
//! better error context, timeouts, and retry logic.

use std::future::Future;

use tokio::time::{timeout, Duration};
use tracing::{error, warn};

use crate::error::{ClientError, Result};

/// Timeout configuration for different types of operations.
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Timeout for network operations (RPC calls, etc.)
    pub network_timeout: Duration,
    /// Timeout for database operations
    pub database_timeout: Duration,
    /// Timeout for cryptographic operations
    pub crypto_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            network_timeout: Duration::from_secs(30),
            database_timeout: Duration::from_secs(10),
            crypto_timeout: Duration::from_secs(60),
        }
    }
}

impl TimeoutConfig {
    /// Creates a new timeout configuration with custom durations.
    pub const fn new(network_secs: u64, database_secs: u64, crypto_secs: u64) -> Self {
        Self {
            network_timeout: Duration::from_secs(network_secs),
            database_timeout: Duration::from_secs(database_secs),
            crypto_timeout: Duration::from_secs(crypto_secs),
        }
    }
}

/// Executes a future with a timeout and proper error context.
///
/// This function wraps any async operation with a timeout and provides
/// clear error context if the operation fails or times out.
///
/// # Arguments
///
/// * `future` - The async operation to execute
/// * `timeout_duration` - How long to wait before timing out
/// * `operation_name` - Description of the operation for error context
///
/// # Returns
///
/// Returns the result of the future or a timeout error.
///
/// # Example
///
/// ```rust,no_run
/// use std::time::Duration;
/// use client::async_utils::with_timeout;
///
/// async fn example() -> Result<String, Box<dyn std::error::Error>> {
///     let result = with_timeout(
///         async { Ok::<String, std::io::Error>("success".to_string()) },
///         Duration::from_secs(5),
///         "example operation"
///     ).await?;
///     Ok(result)
/// }
/// ```
pub async fn with_timeout<F, T, E>(
    future: F,
    timeout_duration: Duration,
    operation_name: &str,
) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    match timeout(timeout_duration, future).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(ClientError::async_operation_failed(
            format!("Operation '{operation_name}' failed"),
            e,
        )),
        Err(_) => Err(ClientError::operation_timeout(
            operation_name,
            timeout_duration.as_millis() as u64,
        )),
    }
}

/// Executes a future with retry logic and exponential backoff.
///
/// This function implements a retry mechanism with exponential backoff
/// for operations that might fail due to temporary issues.
///
/// # Arguments
///
/// * `operation` - A closure that returns a future to execute
/// * `max_retries` - Maximum number of retry attempts
/// * `initial_delay` - Initial delay between retries
/// * `operation_name` - Description of the operation for logging
///
/// # Returns
///
/// Returns the result of the operation or the last error encountered.
pub async fn with_retry<F, Fut, T, E>(
    mut operation: F,
    max_retries: u32,
    initial_delay: Duration,
    operation_name: &str,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut attempt = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt >= max_retries {
                    error!(
                        operation = operation_name,
                        attempts = attempt + 1,
                        error = %e,
                        "Operation failed after all retry attempts"
                    );
                    return Err(ClientError::async_operation_failed(
                        format!(
                            "Operation '{}' failed after {} attempts",
                            operation_name,
                            attempt + 1
                        ),
                        e,
                    ));
                }

                let delay = initial_delay * 2u32.pow(attempt);
                warn!(
                    operation = operation_name,
                    attempt = attempt + 1,
                    max_retries = max_retries,
                    retry_in = ?delay,
                    error = %e,
                    "Operation failed, retrying..."
                );

                tokio::time::sleep(delay).await;
                attempt += 1;
            }
        }
    }
}

/// Executes a future with both timeout and retry logic.
///
/// This combines timeout handling with retry logic for robust async operations.
///
/// # Arguments
///
/// * `operation` - A closure that returns a future to execute
/// * `timeout_duration` - Timeout for each individual attempt
/// * `max_retries` - Maximum number of retry attempts
/// * `initial_delay` - Initial delay between retries
/// * `operation_name` - Description of the operation for logging and errors
pub async fn with_timeout_and_retry<F, Fut, T, E>(
    operation: F,
    timeout_duration: Duration,
    max_retries: u32,
    initial_delay: Duration,
    operation_name: &str,
) -> Result<T>
where
    F: Fn() -> Fut + Clone,
    Fut: Future<Output = std::result::Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let operation_clone = operation.clone();
    with_retry(
        move || {
            let op = operation_clone.clone();
            async move { with_timeout(op(), timeout_duration, operation_name).await }
        },
        max_retries,
        initial_delay,
        operation_name,
    )
    .await
}

#[cfg(test)]
mod tests {
    use tokio::time::sleep;

    use super::*;

    #[tokio::test]
    async fn test_with_timeout_success() {
        let result = with_timeout(
            async { Ok::<i32, std::io::Error>(42) },
            Duration::from_secs(1),
            "test operation",
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_timeout_failure() {
        let result = with_timeout(
            async {
                sleep(Duration::from_secs(2)).await;
                Ok::<i32, std::io::Error>(42)
            },
            Duration::from_millis(100),
            "slow operation",
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ClientError::OperationTimeout { .. })));
    }

    #[tokio::test]
    async fn test_with_retry_success_on_second_attempt() {
        use std::sync::{
            atomic::{AtomicU32, Ordering},
            Arc,
        };

        let counter = Arc::new(AtomicU32::new(0));

        let result = with_retry(
            {
                let counter = counter.clone();
                move || {
                    let counter = counter.clone();
                    async move {
                        let current = counter.fetch_add(1, Ordering::SeqCst);
                        if current == 0 {
                            Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "temporary failure",
                            ))
                        } else {
                            Ok::<i32, std::io::Error>(42)
                        }
                    }
                }
            },
            3,
            Duration::from_millis(10),
            "test retry",
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_with_retry_exhausts_attempts() {
        use std::sync::{
            atomic::{AtomicU32, Ordering},
            Arc,
        };

        let counter = Arc::new(AtomicU32::new(0));

        let result = with_retry(
            {
                let counter = counter.clone();
                move || {
                    let counter = counter.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Err::<i32, std::io::Error>(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "persistent failure",
                        ))
                    }
                }
            },
            2,
            Duration::from_millis(10),
            "failing operation",
        )
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Initial attempt + 2 retries
    }
}
