#![deny(unused_crate_dependencies)]
use clap as _;
use dotenv as _;
use guest_mmr as _;
use pyo3 as _;
use tracing_subscriber as _;
pub mod api;
pub mod cli;
pub mod core;
pub mod db;
pub mod error;
pub mod service;
pub mod utils;

pub use api::operations::{get_block_hash_inclusion_proof, prove_mmr_update};
pub use error::{PublisherError, PublisherResult, Result};
