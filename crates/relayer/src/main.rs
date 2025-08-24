// Dependencies used in relayer library but not directly in main
use std::time::Duration;

use alloy as _;
use alloy_contract as _;
use alloy_sol_types as _;
use async_trait as _;
use backoff as _;
use clap::Parser;
use common::initialize_logger;
use metrics as _;
use relayer::{
    observability::{init_metrics_exporter, LoggingConfig},
    Relayer,
};
use serde as _;
use serde_json as _;
// Test dependencies
#[cfg(test)]
use serial_test as _;
use thiserror as _;
use tokio::time;
use tokio_retry as _;
use tracing::info;
use tracing_subscriber as _;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to environment file (optional)
    #[arg(short = 'e', long, default_value = ".env")]
    env_file: String,

    /// Relay interval in minutes (0 for single run)
    #[arg(short = 't', long, default_value = "0")]
    relay_time_minutes: u64,

    /// Enable JSON logging format
    #[arg(long)]
    json_logs: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Enable Prometheus metrics endpoint (requires --features metrics-prometheus)
    #[arg(long, default_value = "127.0.0.1:9090")]
    metrics_addr: String,

    /// Disable metrics collection entirely
    #[arg(long)]
    no_metrics: bool,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();

    // Initialize environment with specified file
    dotenv::from_path(&args.env_file)?;

    // Initialize structured logging with relayer's observability
    let logging_config = LoggingConfig::new()
        .with_json_format(args.json_logs)
        .with_log_level(format!("relayer={}", args.log_level));

    if let Err(e) = logging_config.init() {
        // Fallback to common logger if relayer logging fails
        eprintln!("Failed to initialize structured logging: {e}, falling back to common logger");
        initialize_logger()?;
    }

    // Initialize metrics exporter if not disabled
    if !args.no_metrics {
        if let Err(e) = init_metrics_exporter(&args.metrics_addr) {
            tracing::warn!(
                error = %e,
                metrics_addr = %args.metrics_addr,
                "Failed to initialize metrics exporter, continuing without metrics"
            );
        } else {
            info!(
                metrics_addr = %args.metrics_addr,
                "Metrics exporter initialized"
            );
        }
    } else {
        info!("Metrics collection disabled by --no-metrics flag");
    }

    info!("Starting the relayer...");

    // Create the relayer instance
    let relayer = Relayer::new()
        .await
        .map_err(|e| eyre::eyre!("Failed to create relayer: {}", e))?;

    // Get health checker for monitoring
    let health_checker = relayer.get_health_checker();

    info!(
        single_run = (args.relay_time_minutes == 0),
        interval_minutes = args.relay_time_minutes,
        json_logs = args.json_logs,
        log_level = %args.log_level,
        metrics_enabled = !args.no_metrics,
        "Relayer initialized with observability features"
    );

    if args.relay_time_minutes == 0 {
        // Single run mode
        info!("Running in single execution mode");
        relayer
            .send_finalized_block_hash_to_l2()
            .await
            .map_err(|e| eyre::eyre!("Failed to relay block hash: {}", e))?;
        info!("Relayer finished successfully");
    } else {
        // Continuous mode with specified interval
        info!(
            "Running in continuous mode with {} minute interval",
            args.relay_time_minutes
        );

        let interval = Duration::from_secs(args.relay_time_minutes * 60);

        loop {
            let start_time = std::time::Instant::now();

            info!("Sending finalized block hash to L2...");
            let relay_result = relayer.send_finalized_block_hash_to_l2().await;

            match &relay_result {
                Ok(_) => {
                    info!("Successfully relayed block hash to L2");

                    // Log health status periodically
                    let health_status = health_checker.get_system_health();
                    info!(
                        system_status = %health_status.status,
                        uptime_seconds = health_status.uptime_seconds,
                        components_monitored = health_status.components.len(),
                        "System health status"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "Failed to relay block hash"
                    );

                    // Log detailed health status on failures
                    let health_status = health_checker.get_system_health();
                    tracing::error!(
                        system_status = %health_status.status,
                        uptime_seconds = health_status.uptime_seconds,
                        "System health status after failure"
                    );

                    for (component, health) in &health_status.components {
                        if matches!(
                            health.status,
                            relayer::HealthStatus::Unhealthy | relayer::HealthStatus::Degraded
                        ) {
                            tracing::error!(
                                component = %component,
                                status = %health.status,
                                message = ?health.message,
                                "Unhealthy component detected"
                            );
                        }
                    }
                }
            }

            let elapsed = start_time.elapsed();
            if elapsed < interval {
                let sleep_time = interval - elapsed;
                info!(
                    "Waiting for {} minutes before next relay...",
                    sleep_time.as_secs() / 60
                );
                time::sleep(sleep_time).await;
            }
        }
    }

    Ok(())
}
