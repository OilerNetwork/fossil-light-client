//! Fossil Light Client Library
//!
//! This library provides a Starknet light client for processing blockchain events
//! and maintaining MMR (Merkle Mountain Range) state synchronization between L1 and L2.

#![deny(unused_crate_dependencies)]

// Import unused dependencies to satisfy the linter
#[allow(unused_imports)]
use chrono as _;
#[allow(unused_imports)]
use clap as _;
#[allow(unused_imports)]
use derive_more as _;
#[allow(unused_imports)]
use dotenv as _;

#[cfg(test)]
mod test_imports {
    #[allow(unused_imports)]
    use hex as _;
    #[allow(unused_imports)]
    use mockall as _;
    #[allow(unused_imports)]
    use proptest as _;
    #[allow(unused_imports)]
    use tempfile as _;
    #[allow(unused_imports)]
    use tokio_test as _;
    #[allow(unused_imports)]
    use toml as _;
}

pub mod async_utils;
mod builder;
mod client;
mod config;
pub mod config_manager;
mod error;
pub mod events;
pub mod logging;
mod mmr;
pub mod performance;
pub mod types;

// Re-export public API
pub use builder::LightClientBuilder;
pub use client::LightClient;
pub use error::{ClientError, Result};
