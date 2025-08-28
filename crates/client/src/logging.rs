//! Advanced logging utilities for the Fossil Light Client.
//!
//! This module provides structured logging capabilities with contextual information,
//! metrics collection, and performance monitoring integration.

use std::time::{Duration, Instant};

use tracing;
use tracing_subscriber::{
    filter::EnvFilter,
    fmt::{format::FmtSpan, time::ChronoUtc},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

/// Contextual information for client operations.
///
/// This struct provides a way to attach consistent contextual information
/// to log entries throughout the application.
#[derive(Debug, Clone, Default)]
pub struct ClientContext {
    /// Current block being processed
    pub current_block: Option<u64>,

    /// Latest known block on the network
    pub latest_network_block: Option<u64>,

    /// Number of events found in current batch
    pub events_in_batch: Option<usize>,

    /// Current operation being performed
    pub operation: Option<String>,

    /// Account address being used
    pub account_address: Option<String>,

    /// Chain ID
    pub chain_id: Option<u64>,
}

impl ClientContext {
    /// Creates a new context with basic block information.
    pub fn with_block(current_block: u64) -> Self {
        Self {
            current_block: Some(current_block),
            ..Default::default()
        }
    }

    /// Creates a new context with operation information.
    pub fn with_operation(operation: impl Into<String>) -> Self {
        Self {
            operation: Some(operation.into()),
            ..Default::default()
        }
    }

    /// Adds block range information to the context.
    pub const fn with_block_range(mut self, current: u64, latest: u64) -> Self {
        self.current_block = Some(current);
        self.latest_network_block = Some(latest);
        self
    }

    /// Adds event count information to the context.
    pub const fn with_events(mut self, count: usize) -> Self {
        self.events_in_batch = Some(count);
        self
    }

    /// Adds chain and account information to the context.
    pub fn with_chain_info(mut self, chain_id: u64, account_address: String) -> Self {
        self.chain_id = Some(chain_id);
        self.account_address = Some(account_address);
        self
    }
}

/// A performance monitoring logger that tracks operation durations.
///
/// This struct provides structured logging with timing information for
/// operations, making it easier to identify performance bottlenecks.
pub struct PerformanceLogger {
    operation: String,
    start_time: Instant,
    context: ClientContext,
}

impl PerformanceLogger {
    /// Starts timing a new operation.
    ///
    /// # Arguments
    ///
    /// * `operation` - Name of the operation being timed
    /// * `context` - Contextual information for the operation
    pub fn start_operation(operation: impl Into<String>, context: ClientContext) -> Self {
        let operation = operation.into();
        let start_time = Instant::now();

        tracing::debug!(
            operation = %operation,
            current_block = context.current_block,
            latest_network_block = context.latest_network_block,
            account_address = context.account_address.as_deref(),
            chain_id = context.chain_id,
            "Starting operation"
        );

        Self {
            operation,
            start_time,
            context,
        }
    }

    /// Records a milestone within the operation.
    ///
    /// # Arguments
    ///
    /// * `milestone` - Description of the milestone reached
    /// * `additional_context` - Any additional context for this milestone
    pub fn log_milestone(&self, milestone: &str, additional_context: Option<&str>) {
        let elapsed = self.start_time.elapsed();

        tracing::debug!(
            operation = %self.operation,
            milestone = %milestone,
            elapsed_ms = elapsed.as_millis() as u64,
            current_block = self.context.current_block,
            events_in_batch = self.context.events_in_batch,
            additional_context = additional_context,
            "Operation milestone reached"
        );
    }

    /// Records successful completion of the operation.
    ///
    /// # Arguments
    ///
    /// * `result_context` - Information about the operation result
    pub fn log_success(self, result_context: Option<&str>) {
        let elapsed = self.start_time.elapsed();

        tracing::debug!(
            operation = %self.operation,
            elapsed_ms = elapsed.as_millis() as u64,
            current_block = self.context.current_block,
            latest_network_block = self.context.latest_network_block,
            events_in_batch = self.context.events_in_batch,
            result_context = result_context,
            "Operation completed successfully"
        );
    }

    /// Records failed completion of the operation.
    ///
    /// # Arguments
    ///
    /// * `error` - The error that caused the failure
    /// * `error_context` - Additional context about the error
    pub fn log_error(self, error: &dyn std::error::Error, error_context: Option<&str>) {
        let elapsed = self.start_time.elapsed();

        tracing::error!(
            operation = %self.operation,
            elapsed_ms = elapsed.as_millis() as u64,
            error = %error,
            error_context = error_context,
            current_block = self.context.current_block,
            latest_network_block = self.context.latest_network_block,
            "Operation failed"
        );
    }
}

/// Initializes structured logging for the client.
///
/// This function sets up comprehensive logging with structured output,
/// performance metrics, and configurable log levels.
///
/// # Arguments
///
/// * `log_level` - The minimum log level to output
/// * `structured` - Whether to use JSON structured logging
/// * `log_format` - The log format ("json", "pretty", or "compact")
///
/// # Example
///
/// ```rust
/// use client::logging::init_structured_logging;
///
/// init_structured_logging("info", true, "pretty");
/// ```
pub fn init_structured_logging(log_level: &str, structured: bool, log_format: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = if structured && log_format == "json" {
        tracing_subscriber::fmt::layer()
            .json()
            .with_timer(ChronoUtc::rfc_3339())
            .with_current_span(true)
            .with_span_list(true)
            .with_thread_ids(true)
            .with_target(true)
            .boxed()
    } else if log_format == "compact" {
        tracing_subscriber::fmt::layer()
            .compact()
            .with_timer(ChronoUtc::rfc_3339())
            .with_target(false)
            .boxed()
    } else {
        // Pretty format (default)
        tracing_subscriber::fmt::layer()
            .pretty()
            .with_timer(ChronoUtc::rfc_3339())
            .with_target(true)
            .with_thread_ids(false)
            .with_span_events(FmtSpan::CLOSE)
            .boxed()
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    tracing::info!(
        log_level = log_level,
        structured = structured,
        format = log_format,
        "Structured logging initialized"
    );
}

/// Creates a structured span for client operations.
///
/// This macro creates a tracing span with consistent fields for client operations,
/// making it easier to correlate log entries and analyze performance.
///
/// # Arguments
///
/// * `$name` - The name of the span
/// * `$context` - The `ClientContext` for the operation
/// * `$($field:ident = $value:expr),*` - Additional fields to include in the span
#[macro_export]
macro_rules! client_span {
    ($name:expr, $context:expr) => {
        tracing::info_span!(
            $name,
            current_block = $context.current_block,
            latest_network_block = $context.latest_network_block,
            events_in_batch = $context.events_in_batch,
            operation = $context.operation.as_deref(),
            account_address = $context.account_address.as_deref(),
            chain_id = $context.chain_id,
        )
    };
    ($name:expr, $context:expr, $($field:ident = $value:expr),+ $(,)?) => {
        tracing::info_span!(
            $name,
            current_block = $context.current_block,
            latest_network_block = $context.latest_network_block,
            events_in_batch = $context.events_in_batch,
            operation = $context.operation.as_deref(),
            account_address = $context.account_address.as_deref(),
            chain_id = $context.chain_id,
            $($field = $value),+
        )
    };
}

/// Logs structured event processing information.
///
/// This function provides consistent logging for event processing operations
/// with all relevant contextual information.
///
/// # Arguments
///
/// * `from_block` - Starting block number
/// * `to_block` - Ending block number
/// * `event_count` - Number of events found
/// * `processing_time` - Time taken to process events
/// * `context` - Additional context information
pub fn log_event_processing(
    from_block: u64,
    to_block: u64,
    event_count: usize,
    processing_time: Duration,
    context: &ClientContext,
) {
    if event_count > 0 {
        tracing::debug!(
            from_block = from_block,
            to_block = to_block,
            event_count = event_count,
            processing_time_ms = processing_time.as_millis() as u64,
            blocks_processed = to_block - from_block + 1,
            events_per_block = event_count as f64 / (to_block - from_block + 1) as f64,
            account_address = context.account_address.as_deref(),
            chain_id = context.chain_id,
            "Events processed successfully"
        );
    } else {
        tracing::debug!(
            from_block = from_block,
            to_block = to_block,
            processing_time_ms = processing_time.as_millis() as u64,
            blocks_checked = to_block - from_block + 1,
            "No events found in block range"
        );
    }
}

/// Logs structured MMR operation information.
///
/// This function provides consistent logging for MMR operations with
/// performance metrics and contextual information.
///
/// # Arguments
///
/// * `operation` - The type of MMR operation performed
/// * `block_number` - Block number being processed
/// * `batch_size` - Size of the processing batch
/// * `processing_time` - Time taken for the operation
/// * `success` - Whether the operation was successful
/// * `context` - Additional context information
#[allow(clippy::too_many_arguments)]
pub fn log_mmr_operation(
    operation: &str,
    block_number: u64,
    batch_size: u64,
    processing_time: Duration,
    success: bool,
    error: Option<&dyn std::error::Error>,
    context: &ClientContext,
) {
    if success {
        tracing::debug!(
            operation = operation,
            block_number = block_number,
            batch_size = batch_size,
            processing_time_ms = processing_time.as_millis() as u64,
            account_address = context.account_address.as_deref(),
            chain_id = context.chain_id,
            "MMR operation completed successfully"
        );
    } else {
        tracing::error!(
            operation = operation,
            block_number = block_number,
            batch_size = batch_size,
            processing_time_ms = processing_time.as_millis() as u64,
            error = error.map(|e| e.to_string()).as_deref(),
            account_address = context.account_address.as_deref(),
            chain_id = context.chain_id,
            "MMR operation failed"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::*;

    #[test]
    fn test_client_context_builder() {
        let context = ClientContext::with_block(100)
            .with_events(5)
            .with_chain_info(1, "0x123".to_string());

        assert_eq!(context.current_block, Some(100));
        assert_eq!(context.events_in_batch, Some(5));
        assert_eq!(context.chain_id, Some(1));
        assert_eq!(context.account_address, Some("0x123".to_string()));
    }

    #[test]
    fn test_performance_logger() {
        let context = ClientContext::with_operation("test operation");
        let logger = PerformanceLogger::start_operation("test", context);

        // Simulate some work
        thread::sleep(Duration::from_millis(10));
        logger.log_milestone("halfway", Some("test milestone"));

        thread::sleep(Duration::from_millis(10));
        logger.log_success(Some("test completed"));
    }
}
