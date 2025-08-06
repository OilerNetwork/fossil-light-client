use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use metrics::counter;
use tracing::{debug, error, warn};

use super::is_retryable_error;
use crate::error::{NetworkError, Result};

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests are rejected immediately
    Open,
    /// Circuit is half-open, allowing limited requests to test if service recovered
    HalfOpen,
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: usize,
    /// Time to wait before attempting to close the circuit again
    pub recovery_timeout: Duration,
    /// Number of successful requests needed to close the circuit from half-open state
    pub success_threshold: usize,
    /// Timeout for individual requests
    pub request_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            success_threshold: 3,
            request_timeout: Duration::from_secs(10),
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new circuit breaker configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set failure threshold
    pub const fn with_failure_threshold(mut self, threshold: usize) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Set recovery timeout
    pub const fn with_recovery_timeout(mut self, timeout: Duration) -> Self {
        self.recovery_timeout = timeout;
        self
    }

    /// Set success threshold
    pub const fn with_success_threshold(mut self, threshold: usize) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Set request timeout
    pub const fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
}

/// Circuit breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<AtomicUsize>,
    success_count: Arc<AtomicUsize>,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
    name: String,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig, name: impl Into<String>) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicUsize::new(0)),
            success_count: Arc::new(AtomicUsize::new(0)),
            last_failure_time: Arc::new(Mutex::new(None)),
            name: name.into(),
        }
    }

    /// Execute an operation through the circuit breaker
    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check if circuit allows the request
        if !self.can_execute() {
            return Err(NetworkError::ProviderError(format!(
                "Circuit breaker '{}' is open",
                self.name
            ))
            .into());
        }

        debug!(
            circuit_breaker = %self.name,
            state = ?self.get_state(),
            "Executing operation through circuit breaker"
        );

        // Execute with timeout
        let result = tokio::time::timeout(self.config.request_timeout, operation()).await;

        match result {
            Ok(Ok(success)) => {
                self.on_success().await;
                Ok(success)
            }
            Ok(Err(error)) => {
                if is_retryable_error(&error) {
                    self.on_failure().await;
                }
                Err(error)
            }
            Err(_) => {
                let timeout_error = NetworkError::Timeout {
                    timeout: self.config.request_timeout,
                    operation: format!("Circuit breaker '{}' request", self.name),
                };
                self.on_failure().await;
                Err(timeout_error.into())
            }
        }
    }

    /// Check if the circuit breaker allows execution
    fn can_execute(&self) -> bool {
        let state = self.get_state();

        match state {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => {
                // Check if enough time has passed to try half-open
                if let Ok(guard) = self.last_failure_time.lock() {
                    if let Some(last_failure) = *guard {
                        if last_failure.elapsed() >= self.config.recovery_timeout {
                            self.transition_to_half_open();
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    // Mutex is poisoned, assume we can't execute safely
                    false
                }
            }
        }
    }

    /// Handle successful operation
    async fn on_success(&self) {
        let state = self.get_state();

        match state {
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;

                debug!(
                    circuit_breaker = %self.name,
                    success_count = success_count,
                    success_threshold = self.config.success_threshold,
                    "Success in half-open state"
                );

                if success_count >= self.config.success_threshold {
                    self.transition_to_closed();
                }
            }
            CircuitState::Open => {
                // This shouldn't happen, but if it does, transition to half-open
                warn!(
                    circuit_breaker = %self.name,
                    "Unexpected success in open state, transitioning to half-open"
                );
                self.transition_to_half_open();
            }
        }
    }

    /// Handle failed operation
    async fn on_failure(&self) {
        let state = self.get_state();
        let failure_count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;

        if let Ok(mut guard) = self.last_failure_time.lock() {
            *guard = Some(Instant::now());
        }

        warn!(
            circuit_breaker = %self.name,
            state = ?state,
            failure_count = failure_count,
            failure_threshold = self.config.failure_threshold,
            "Operation failed"
        );

        match state {
            CircuitState::Closed => {
                if failure_count >= self.config.failure_threshold {
                    self.transition_to_open();
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open state transitions back to open
                self.transition_to_open();
            }
            CircuitState::Open => {
                // Already open, just update failure time
            }
        }
    }

    /// Get current circuit state
    pub fn get_state(&self) -> CircuitState {
        self.state
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or(CircuitState::Open)
    }

    /// Get failure count
    pub fn get_failure_count(&self) -> usize {
        self.failure_count.load(Ordering::SeqCst)
    }

    /// Get success count (relevant in half-open state)
    pub fn get_success_count(&self) -> usize {
        self.success_count.load(Ordering::SeqCst)
    }

    /// Transition to closed state
    fn transition_to_closed(&self) {
        if let Ok(mut state) = self.state.lock() {
            if *state != CircuitState::Closed {
                let from_state = format!("{:?}", *state);

                debug!(
                    circuit_breaker = %self.name,
                    from_state = ?*state,
                    "Transitioning to closed state"
                );

                // Record state change metric
                counter!(
                    "relayer_circuit_breaker_state_changes_total",
                    "circuit_breaker" => self.name.clone(),
                    "from_state" => from_state,
                    "to_state" => "Closed".to_string()
                )
                .increment(1);

                *state = CircuitState::Closed;
                self.failure_count.store(0, Ordering::SeqCst);
                self.success_count.store(0, Ordering::SeqCst);
            }
        }
    }

    /// Transition to open state
    fn transition_to_open(&self) {
        if let Ok(mut state) = self.state.lock() {
            if *state != CircuitState::Open {
                let from_state = format!("{:?}", *state);

                error!(
                    circuit_breaker = %self.name,
                    from_state = ?*state,
                    failure_count = self.failure_count.load(Ordering::SeqCst),
                    "Transitioning to open state"
                );

                // Record state change metric
                counter!(
                    "relayer_circuit_breaker_state_changes_total",
                    "circuit_breaker" => self.name.clone(),
                    "from_state" => from_state,
                    "to_state" => "Open".to_string()
                )
                .increment(1);

                *state = CircuitState::Open;
                self.success_count.store(0, Ordering::SeqCst);
            }
        }
    }

    /// Transition to half-open state
    fn transition_to_half_open(&self) {
        if let Ok(mut state) = self.state.lock() {
            if *state != CircuitState::HalfOpen {
                let from_state = format!("{:?}", *state);

                debug!(
                    circuit_breaker = %self.name,
                    from_state = ?*state,
                    "Transitioning to half-open state"
                );

                // Record state change metric
                counter!(
                    "relayer_circuit_breaker_state_changes_total",
                    "circuit_breaker" => self.name.clone(),
                    "from_state" => from_state,
                    "to_state" => "HalfOpen".to_string()
                )
                .increment(1);

                *state = CircuitState::HalfOpen;
                self.success_count.store(0, Ordering::SeqCst);
            }
        }
    }

    /// Reset the circuit breaker to closed state
    pub fn reset(&self) {
        debug!(
            circuit_breaker = %self.name,
            "Resetting circuit breaker"
        );

        if let Ok(mut state) = self.state.lock() {
            *state = CircuitState::Closed;
        }
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        if let Ok(mut guard) = self.last_failure_time.lock() {
            *guard = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{NetworkError, RelayerError};

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let config = CircuitBreakerConfig::new().with_failure_threshold(2);
        let breaker = CircuitBreaker::new(config, "test");

        assert_eq!(breaker.get_state(), CircuitState::Closed);

        // Successful operation should keep circuit closed
        let result = breaker.call(|| async { Ok::<i32, RelayerError>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(breaker.get_state(), CircuitState::Closed);
        assert_eq!(breaker.get_failure_count(), 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig::new()
            .with_failure_threshold(2)
            .with_request_timeout(Duration::from_millis(100));
        let breaker = CircuitBreaker::new(config, "test");

        // First failure
        let result = breaker
            .call(|| async {
                Err::<i32, RelayerError>(RelayerError::Network(NetworkError::ConnectionFailed(
                    "test".to_string(),
                )))
            })
            .await;
        assert!(result.is_err());
        assert_eq!(breaker.get_state(), CircuitState::Closed);
        assert_eq!(breaker.get_failure_count(), 1);

        // Second failure should open the circuit
        let result = breaker
            .call(|| async {
                Err::<i32, RelayerError>(RelayerError::Network(NetworkError::ConnectionFailed(
                    "test".to_string(),
                )))
            })
            .await;
        assert!(result.is_err());
        assert_eq!(breaker.get_state(), CircuitState::Open);
        assert_eq!(breaker.get_failure_count(), 2);
    }

    #[tokio::test]
    async fn test_circuit_breaker_rejects_when_open() {
        let config = CircuitBreakerConfig::new()
            .with_failure_threshold(1)
            .with_recovery_timeout(Duration::from_secs(1));
        let breaker = CircuitBreaker::new(config, "test");

        // Cause failure to open circuit
        let _ = breaker
            .call(|| async {
                Err::<i32, RelayerError>(RelayerError::Network(NetworkError::ConnectionFailed(
                    "test".to_string(),
                )))
            })
            .await;
        assert_eq!(breaker.get_state(), CircuitState::Open);

        // Next call should be rejected immediately
        let result = breaker.call(|| async { Ok::<i32, RelayerError>(42) }).await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Circuit breaker"));
        assert!(error_msg.contains("is open"));
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_recovery() {
        let config = CircuitBreakerConfig::new()
            .with_failure_threshold(1)
            .with_recovery_timeout(Duration::from_millis(10))
            .with_success_threshold(2);
        let breaker = CircuitBreaker::new(config, "test");

        // Open the circuit
        let _ = breaker
            .call(|| async {
                Err::<i32, RelayerError>(RelayerError::Network(NetworkError::ConnectionFailed(
                    "test".to_string(),
                )))
            })
            .await;
        assert_eq!(breaker.get_state(), CircuitState::Open);

        // Wait for recovery timeout
        tokio::time::sleep(Duration::from_millis(15)).await;

        // First success should transition to half-open
        let result = breaker.call(|| async { Ok::<i32, RelayerError>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(breaker.get_state(), CircuitState::HalfOpen);

        // Second success should close the circuit
        let result = breaker.call(|| async { Ok::<i32, RelayerError>(43) }).await;
        assert!(result.is_ok());
        assert_eq!(breaker.get_state(), CircuitState::Closed);
        assert_eq!(breaker.get_failure_count(), 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_timeout() {
        let config = CircuitBreakerConfig::new()
            .with_request_timeout(Duration::from_millis(10))
            .with_failure_threshold(1);
        let breaker = CircuitBreaker::new(config, "test");

        // Operation that takes longer than timeout
        let result = breaker
            .call(|| async {
                tokio::time::sleep(Duration::from_millis(20)).await;
                Ok::<i32, RelayerError>(42)
            })
            .await;

        assert!(result.is_err());
        assert_eq!(breaker.get_state(), CircuitState::Open);

        let error = result.unwrap_err();
        match error {
            RelayerError::Network(NetworkError::Timeout { .. }) => {}
            _ => panic!("Expected timeout error"),
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig::new().with_failure_threshold(1);
        let breaker = CircuitBreaker::new(config, "test");

        // Open the circuit
        let _ = breaker
            .call(|| async {
                Err::<i32, RelayerError>(RelayerError::Network(NetworkError::ConnectionFailed(
                    "test".to_string(),
                )))
            })
            .await;
        assert_eq!(breaker.get_state(), CircuitState::Open);

        // Reset should close the circuit
        breaker.reset();
        assert_eq!(breaker.get_state(), CircuitState::Closed);
        assert_eq!(breaker.get_failure_count(), 0);

        // Should work normally now
        let result = breaker.call(|| async { Ok::<i32, RelayerError>(42) }).await;
        assert!(result.is_ok());
    }
}
