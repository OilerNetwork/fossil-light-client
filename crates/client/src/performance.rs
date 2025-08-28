//! Performance monitoring and optimization utilities.
//!
//! This module provides performance monitoring capabilities and optimizations
//! for the light client operations.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

/// Performance metrics for monitoring client operations.
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Total events processed
    pub events_processed: u64,
    /// Total processing time in milliseconds
    pub total_processing_time_ms: u64,
    /// Average processing time per event in milliseconds
    pub avg_processing_time_per_event_ms: f64,
    /// Number of successful operations
    pub successful_operations: u64,
    /// Number of failed operations
    pub failed_operations: u64,
    /// Current blocks per second processing rate
    pub blocks_per_second: f64,
}

/// Trait for performance monitoring.
///
/// This trait provides methods for tracking and reporting performance metrics.
pub trait PerformanceMonitorTrait {
    /// Records the start of an operation.
    fn start_operation(&mut self, operation_name: &str);

    /// Records the completion of an operation.
    fn complete_operation(&mut self, operation_name: &str, success: bool);

    /// Records event processing metrics.
    fn record_event_processing(&mut self, event_count: usize, processing_time_ms: u64);

    /// Gets current performance metrics.
    fn get_metrics(&self) -> PerformanceMetrics;

    /// Resets all metrics.
    fn reset_metrics(&mut self);
}

/// Performance monitor implementation for tracking client operations.
///
/// This struct provides comprehensive performance monitoring with metrics
/// tracking for operations, event processing, and system performance.
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// Track operation start times
    operation_starts: Arc<Mutex<HashMap<String, Instant>>>,
    /// Track operation counts and durations
    operation_metrics: Arc<Mutex<HashMap<String, OperationStats>>>,
    /// Global counters for atomic operations
    events_processed: AtomicU64,
    total_processing_time_ms: AtomicU64,
    successful_operations: AtomicU64,
    failed_operations: AtomicU64,
}

#[derive(Debug, Clone)]
struct OperationStats {
    count: u64,
    total_duration_ms: u64,
    success_count: u64,
    failure_count: u64,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMonitor {
    /// Creates a new performance monitor.
    pub fn new() -> Self {
        Self {
            operation_starts: Arc::new(Mutex::new(HashMap::new())),
            operation_metrics: Arc::new(Mutex::new(HashMap::new())),
            events_processed: AtomicU64::new(0),
            total_processing_time_ms: AtomicU64::new(0),
            successful_operations: AtomicU64::new(0),
            failed_operations: AtomicU64::new(0),
        }
    }

    /// Creates a performance monitor optimized for high-throughput scenarios.
    ///
    /// This version pre-allocates space for common operations and uses
    /// optimized data structures for better performance.
    pub fn high_performance() -> Self {
        let monitor = Self::new();

        // Pre-allocate space for common operations
        if let Ok(mut metrics) = monitor.operation_metrics.lock() {
            let common_operations = [
                "process_new_events",
                "fetch_events",
                "update_mmr",
                "get_latest_block",
                "handle_events",
            ];

            for op in &common_operations {
                metrics.insert(
                    op.to_string(),
                    OperationStats {
                        count: 0,
                        total_duration_ms: 0,
                        success_count: 0,
                        failure_count: 0,
                    },
                );
            }
        }

        monitor
    }

    /// Records the start and completion of an operation with timing.
    ///
    /// This is a convenience method that combines `start_operation` and `complete_operation`
    /// for operations where you have the full duration available.
    pub fn record_operation(&mut self, operation_name: &str, duration: Duration, success: bool) {
        let duration_ms = duration.as_millis() as u64;

        if let Ok(mut metrics) = self.operation_metrics.lock() {
            let stats = metrics
                .entry(operation_name.to_string())
                .or_insert(OperationStats {
                    count: 0,
                    total_duration_ms: 0,
                    success_count: 0,
                    failure_count: 0,
                });

            stats.count += 1;
            stats.total_duration_ms += duration_ms;

            if success {
                stats.success_count += 1;
                self.successful_operations.fetch_add(1, Ordering::Relaxed);
            } else {
                stats.failure_count += 1;
                self.failed_operations.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Gets performance metrics for a specific operation.
    pub fn get_operation_metrics(&self, operation_name: &str) -> Option<OperationMetrics> {
        self.operation_metrics.lock().map_or(None, |metrics| {
            metrics.get(operation_name).map(|stats| OperationMetrics {
                operation_name: operation_name.to_string(),
                total_calls: stats.count,
                total_duration_ms: stats.total_duration_ms,
                average_duration_ms: if stats.count > 0 {
                    stats.total_duration_ms as f64 / stats.count as f64
                } else {
                    0.0
                },
                success_rate: if stats.count > 0 {
                    stats.success_count as f64 / stats.count as f64
                } else {
                    0.0
                },
                successful_calls: stats.success_count,
                failed_calls: stats.failure_count,
            })
        })
    }

    /// Gets all operation metrics.
    pub fn get_all_operation_metrics(&self) -> Vec<OperationMetrics> {
        self.operation_metrics.lock().map_or_else(
            |_| Vec::new(),
            |metrics| {
                metrics
                    .iter()
                    .map(|(name, stats)| OperationMetrics {
                        operation_name: name.clone(),
                        total_calls: stats.count,
                        total_duration_ms: stats.total_duration_ms,
                        average_duration_ms: if stats.count > 0 {
                            stats.total_duration_ms as f64 / stats.count as f64
                        } else {
                            0.0
                        },
                        success_rate: if stats.count > 0 {
                            stats.success_count as f64 / stats.count as f64
                        } else {
                            0.0
                        },
                        successful_calls: stats.success_count,
                        failed_calls: stats.failure_count,
                    })
                    .collect()
            },
        )
    }
}

impl PerformanceMonitorTrait for PerformanceMonitor {
    fn start_operation(&mut self, operation_name: &str) {
        if let Ok(mut starts) = self.operation_starts.lock() {
            starts.insert(operation_name.to_string(), Instant::now());
        }
    }

    fn complete_operation(&mut self, operation_name: &str, success: bool) {
        let duration = self.operation_starts.lock().map_or(None, |mut starts| {
            starts.remove(operation_name).map(|start| start.elapsed())
        });

        if let Some(duration) = duration {
            self.record_operation(operation_name, duration, success);
        }
    }

    fn record_event_processing(&mut self, event_count: usize, processing_time_ms: u64) {
        self.events_processed
            .fetch_add(event_count as u64, Ordering::Relaxed);
        self.total_processing_time_ms
            .fetch_add(processing_time_ms, Ordering::Relaxed);
    }

    fn get_metrics(&self) -> PerformanceMetrics {
        let events_processed = self.events_processed.load(Ordering::Relaxed);
        let total_time = self.total_processing_time_ms.load(Ordering::Relaxed);
        let successful_ops = self.successful_operations.load(Ordering::Relaxed);
        let failed_ops = self.failed_operations.load(Ordering::Relaxed);

        PerformanceMetrics {
            events_processed,
            total_processing_time_ms: total_time,
            avg_processing_time_per_event_ms: if events_processed > 0 {
                total_time as f64 / events_processed as f64
            } else {
                0.0
            },
            successful_operations: successful_ops,
            failed_operations: failed_ops,
            blocks_per_second: if total_time > 0 {
                (events_processed as f64 * 1000.0) / total_time as f64
            } else {
                0.0
            },
        }
    }

    fn reset_metrics(&mut self) {
        if let Ok(mut starts) = self.operation_starts.lock() {
            starts.clear();
        }
        if let Ok(mut metrics) = self.operation_metrics.lock() {
            metrics.clear();
        }
        self.events_processed.store(0, Ordering::Relaxed);
        self.total_processing_time_ms.store(0, Ordering::Relaxed);
        self.successful_operations.store(0, Ordering::Relaxed);
        self.failed_operations.store(0, Ordering::Relaxed);
    }
}

/// Detailed metrics for a specific operation.
#[derive(Debug, Clone)]
pub struct OperationMetrics {
    /// Name of the operation
    pub operation_name: String,
    /// Total number of calls to this operation
    pub total_calls: u64,
    /// Total duration of all calls in milliseconds
    pub total_duration_ms: u64,
    /// Average duration per call in milliseconds
    pub average_duration_ms: f64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Number of successful calls
    pub successful_calls: u64,
    /// Number of failed calls
    pub failed_calls: u64,
}

/// Performance optimization utilities.
pub struct PerformanceOptimizer;

impl PerformanceOptimizer {
    /// Suggests optimal batch sizes based on current performance metrics.
    ///
    /// This method analyzes processing times and suggests batch sizes
    /// that maximize throughput while maintaining reasonable latency.
    pub fn suggest_batch_size(metrics: &PerformanceMetrics, current_batch_size: u64) -> u64 {
        // Base optimization on average processing time per event
        let avg_time_per_event = metrics.avg_processing_time_per_event_ms;

        if avg_time_per_event < 10.0 {
            // Very fast processing, can increase batch size
            std::cmp::min(current_batch_size * 2, 10000)
        } else if avg_time_per_event > 100.0 {
            // Slow processing, decrease batch size
            std::cmp::max(current_batch_size / 2, 10)
        } else {
            // Good performance, keep current batch size
            current_batch_size
        }
    }

    /// Suggests optimal polling intervals based on event frequency.
    ///
    /// This method analyzes event processing patterns to suggest
    /// polling intervals that balance responsiveness with efficiency.
    pub fn suggest_polling_interval(
        metrics: &PerformanceMetrics,
        current_interval_secs: u64,
    ) -> u64 {
        let blocks_per_second = metrics.blocks_per_second;

        if blocks_per_second > 1.0 {
            // High activity, poll more frequently
            std::cmp::max(current_interval_secs / 2, 1)
        } else if blocks_per_second < 0.1 {
            // Low activity, can poll less frequently
            std::cmp::min(current_interval_secs * 2, 300) // Max 5 minutes
        } else {
            // Reasonable activity, keep current interval
            current_interval_secs
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::*;

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new();
        let metrics = monitor.get_metrics();

        assert_eq!(metrics.events_processed, 0);
        assert_eq!(metrics.successful_operations, 0);
        assert_eq!(metrics.failed_operations, 0);
    }

    #[test]
    fn test_event_processing_recording() {
        let mut monitor = PerformanceMonitor::new();
        monitor.record_event_processing(10, 100);

        let metrics = monitor.get_metrics();
        assert_eq!(metrics.events_processed, 10);
        assert_eq!(metrics.total_processing_time_ms, 100);
        assert_eq!(metrics.avg_processing_time_per_event_ms, 10.0);
    }

    #[test]
    fn test_operation_tracking() {
        let mut monitor = PerformanceMonitor::new();

        monitor.start_operation("test_op");
        thread::sleep(Duration::from_millis(10));
        monitor.complete_operation("test_op", true);

        let metrics = monitor.get_metrics();
        assert_eq!(metrics.successful_operations, 1);
        assert_eq!(metrics.failed_operations, 0);

        let op_metrics = monitor.get_operation_metrics("test_op").unwrap();
        assert_eq!(op_metrics.total_calls, 1);
        assert_eq!(op_metrics.successful_calls, 1);
        assert_eq!(op_metrics.success_rate, 1.0);
    }

    #[test]
    fn test_batch_size_optimization() {
        let metrics = PerformanceMetrics {
            events_processed: 100,
            total_processing_time_ms: 500, // 5ms per event (fast)
            avg_processing_time_per_event_ms: 5.0,
            successful_operations: 10,
            failed_operations: 0,
            blocks_per_second: 20.0,
        };

        let suggested = PerformanceOptimizer::suggest_batch_size(&metrics, 1000);
        assert_eq!(suggested, 2000); // Should increase for fast processing
    }

    #[test]
    fn test_polling_interval_optimization() {
        let high_activity_metrics = PerformanceMetrics {
            events_processed: 100,
            total_processing_time_ms: 1000,
            avg_processing_time_per_event_ms: 10.0,
            successful_operations: 10,
            failed_operations: 0,
            blocks_per_second: 2.0, // High activity
        };

        let suggested = PerformanceOptimizer::suggest_polling_interval(&high_activity_metrics, 10);
        assert_eq!(suggested, 5); // Should decrease interval for high activity
    }

    #[test]
    fn test_reset_metrics() {
        let mut monitor = PerformanceMonitor::new();
        monitor.record_event_processing(10, 100);

        let metrics_before = monitor.get_metrics();
        assert_eq!(metrics_before.events_processed, 10);

        monitor.reset_metrics();

        let metrics_after = monitor.get_metrics();
        assert_eq!(metrics_after.events_processed, 0);
    }
}
