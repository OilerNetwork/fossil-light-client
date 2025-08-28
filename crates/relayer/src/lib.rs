//! # Relayer
//!
//! A service for relaying finalized Ethereum block hashes to Layer 2 networks.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use relayer::{Relayer, RelayerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create relayer from environment variables
//!     let relayer = Relayer::new().await?;
//!     
//!     // Send block hash to L2
//!     relayer.send_finalized_block_hash_to_l2().await?;
//!     
//!     Ok(())
//! }
//! ```

#![deny(unused_crate_dependencies)]
#![warn(missing_docs)]

// Import dependencies used in various parts of the crate
use alloy as _;
use alloy_contract as _;
use alloy_sol_types as _;
use async_trait as _;
use backoff as _;
use metrics as _;
use serde as _;
use serde_json as _;
// Import test dependencies
#[cfg(test)]
use serial_test as _;
use thiserror as _;
use tokio_retry as _;
use tracing as _;
use tracing_subscriber as _;
// Import dependencies used only in main.rs but not in lib.rs
use {clap as _, dotenv as _, eyre as _, tokio as _};

/// Configuration management for the relayer
pub mod config;
/// Error types and handling
pub mod error;
/// Network layer abstractions and implementations
pub mod network;
/// Observability features including structured logging, metrics, and health checks
pub mod observability;
/// Reliability features including retry, circuit breaker, and recovery
pub mod reliability;
/// Service layer with business logic
pub mod service;

#[cfg(test)]
/// Testing utilities and mocks
pub mod testing;

// Re-export public API
pub use config::RelayerConfig;
pub use error::{RelayerError, Result};
pub use network::EthereumProviderConfig;
pub use observability::{
    HealthChecker, HealthStatus, LoggingConfig, MetricsSnapshot, RelayerMetrics,
};
pub use reliability::{CircuitBreakerConfig, RetryConfig};
pub use service::{Relayer, RelayerService};
