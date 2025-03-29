#![deny(unused_crate_dependencies)]
use clap as _;
use dotenv as _;
use pyo3 as _;
use tracing_subscriber as _;
pub mod api;
pub mod cli;
pub mod core;
pub mod db;
pub mod utils;

pub use api::operations::prove_mmr_update;
