use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use serde::{Deserialize, Serialize};

/// Metrics collector for relayer operations
///
/// This struct provides centralized metrics collection for monitoring
/// relayer performance, transaction success rates, and operational health.
#[derive(Debug)]
pub struct RelayerMetrics {
    /// Start time for uptime calculation
    start_time: Instant,
    /// Total number of successful transactions
    successful_transactions: AtomicU64,
    /// Total number of failed transactions
    failed_transactions: AtomicU64,
    /// Current retry count
    current_retries: AtomicU64,
}

impl Default for RelayerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl RelayerMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            successful_transactions: AtomicU64::new(0),
            failed_transactions: AtomicU64::new(0),
            current_retries: AtomicU64::new(0),
        }
    }

    /// Initialize metrics descriptions
    ///
    /// This should be called once at startup to register metric descriptions
    /// with the metrics registry.
    pub fn init_descriptions() {
        // Counters
        describe_counter!(
            "relayer_transactions_total",
            "Total number of transactions processed by outcome"
        );
        describe_counter!(
            "relayer_retries_total",
            "Total number of transaction retries"
        );
        describe_counter!(
            "relayer_circuit_breaker_state_changes_total",
            "Total number of circuit breaker state changes"
        );

        // Histograms
        describe_histogram!(
            "relayer_transaction_duration_seconds",
            "Duration of transaction processing in seconds"
        );
        describe_histogram!(
            "relayer_confirmation_duration_seconds",
            "Duration of transaction confirmation in seconds"
        );
        describe_histogram!(
            "relayer_retry_backoff_duration_seconds",
            "Duration of retry backoff delays in seconds"
        );

        // Gauges
        describe_gauge!(
            "relayer_uptime_seconds",
            "Total uptime of the relayer in seconds"
        );
        describe_gauge!(
            "relayer_active_connections",
            "Number of active network connections"
        );
        describe_gauge!(
            "relayer_circuit_breaker_failure_count",
            "Current failure count in circuit breaker"
        );
    }

    /// Record a successful transaction
    pub fn record_transaction_success(&self) {
        self.successful_transactions.fetch_add(1, Ordering::Relaxed);
        counter!("relayer_transactions_total", "outcome" => "success").increment(1);
    }

    /// Record a failed transaction
    pub fn record_transaction_failure(&self, error_type: &str) {
        self.failed_transactions.fetch_add(1, Ordering::Relaxed);
        counter!("relayer_transactions_total", "outcome" => "failure", "error_type" => error_type.to_string()).increment(1);
    }

    /// Record a retry attempt
    pub fn record_retry(&self, attempt: u32) {
        self.current_retries.fetch_add(1, Ordering::Relaxed);
        counter!("relayer_retries_total", "attempt" => attempt.to_string()).increment(1);
    }

    /// Record transaction processing duration
    pub fn record_transaction_duration(&self, duration: Duration) {
        histogram!("relayer_transaction_duration_seconds").record(duration.as_secs_f64());
    }

    /// Record transaction confirmation duration
    pub fn record_confirmation_duration(&self, duration: Duration) {
        histogram!("relayer_confirmation_duration_seconds").record(duration.as_secs_f64());
    }

    /// Record retry backoff duration
    pub fn record_backoff_duration(&self, duration: Duration) {
        histogram!("relayer_retry_backoff_duration_seconds").record(duration.as_secs_f64());
    }

    /// Update uptime gauge
    pub fn update_uptime(&self) {
        let uptime = self.start_time.elapsed().as_secs_f64();
        gauge!("relayer_uptime_seconds").set(uptime);
    }

    /// Update active connections gauge
    pub fn update_active_connections(&self, count: u64) {
        gauge!("relayer_active_connections").set(count as f64);
    }

    /// Update circuit breaker failure count
    pub fn update_circuit_breaker_failures(&self, count: u64) {
        gauge!("relayer_circuit_breaker_failure_count").set(count as f64);
    }

    /// Record circuit breaker state change
    pub fn record_circuit_breaker_state_change(&self, from_state: &str, to_state: &str) {
        counter!(
            "relayer_circuit_breaker_state_changes_total",
            "from_state" => from_state.to_string(),
            "to_state" => to_state.to_string()
        )
        .increment(1);
    }

    /// Get current metrics snapshot
    pub fn get_snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            uptime_seconds: self.start_time.elapsed().as_secs(),
            successful_transactions: self.successful_transactions.load(Ordering::Relaxed),
            failed_transactions: self.failed_transactions.load(Ordering::Relaxed),
            total_retries: self.current_retries.load(Ordering::Relaxed),
            success_rate: self.calculate_success_rate(),
        }
    }

    /// Calculate current success rate
    fn calculate_success_rate(&self) -> f64 {
        let successful = self.successful_transactions.load(Ordering::Relaxed);
        let failed = self.failed_transactions.load(Ordering::Relaxed);
        let total = successful + failed;

        if total == 0 {
            0.0
        } else {
            successful as f64 / total as f64
        }
    }
}

/// Snapshot of current metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Total uptime in seconds
    pub uptime_seconds: u64,
    /// Number of successful transactions
    pub successful_transactions: u64,
    /// Number of failed transactions
    pub failed_transactions: u64,
    /// Total retry attempts
    pub total_retries: u64,
    /// Current success rate (0.0 to 1.0)
    pub success_rate: f64,
}

/// Timer for measuring operation durations
#[derive(Debug)]
pub struct MetricsTimer {
    start: Instant,
    name: String,
}

impl MetricsTimer {
    /// Start a new timer
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
        }
    }

    /// Record the elapsed time and finish the timer
    pub fn finish(self) -> Duration {
        let duration = self.start.elapsed();
        histogram!(format!("{}_duration_seconds", self.name)).record(duration.as_secs_f64());
        duration
    }

    /// Get elapsed time without finishing the timer
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

/// Initialize metrics exporter
///
/// Sets up Prometheus metrics exporter if the feature is enabled.
/// This function is optional and requires the `metrics-prometheus` feature.
#[cfg(feature = "metrics-prometheus")]
pub fn init_metrics_exporter(
    bind_address: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use metrics_exporter_prometheus::PrometheusBuilder;

    let builder = PrometheusBuilder::new();
    let handle = builder
        .with_http_listener(bind_address.parse::<std::net::SocketAddr>()?)
        .install()?;

    tracing::info!(
        bind_address = %bind_address,
        "Prometheus metrics exporter initialized"
    );

    // Spawn a task to handle metrics updates
    tokio::spawn(async move {
        // Keep the handle alive
        let _handle = handle;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });

    Ok(())
}

/// No-op metrics exporter when Prometheus feature is disabled
#[cfg(not(feature = "metrics-prometheus"))]
pub fn init_metrics_exporter(
    _bind_address: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Metrics exporter not enabled (compile with --features metrics-prometheus)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_metrics_snapshot() {
        let metrics = RelayerMetrics::new();

        // Record some metrics
        metrics.record_transaction_success();
        metrics.record_transaction_success();
        metrics.record_transaction_failure("network_error");

        let snapshot = metrics.get_snapshot();
        assert_eq!(snapshot.successful_transactions, 2);
        assert_eq!(snapshot.failed_transactions, 1);
        assert_eq!(snapshot.success_rate, 2.0 / 3.0);
    }

    #[test]
    fn test_success_rate_calculation() {
        let metrics = RelayerMetrics::new();

        // No transactions yet
        assert_eq!(metrics.calculate_success_rate(), 0.0);

        // All successful
        metrics.record_transaction_success();
        metrics.record_transaction_success();
        assert_eq!(metrics.calculate_success_rate(), 1.0);

        // Mixed results
        metrics.record_transaction_failure("test");
        assert_eq!(metrics.calculate_success_rate(), 2.0 / 3.0);
    }

    #[test]
    fn test_metrics_timer() {
        let timer = MetricsTimer::start("test_operation");
        std::thread::sleep(Duration::from_millis(10));
        let duration = timer.finish();
        assert!(duration >= Duration::from_millis(10));
    }
}
