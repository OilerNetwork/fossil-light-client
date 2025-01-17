#![deny(unused_crate_dependencies)]
use clap as _;
use dotenv as _;
use pyo3 as _;

pub mod api;
pub mod cli;
pub mod core;
pub mod db;
pub mod errors;
pub mod utils;
pub mod validator;

pub use api::operations::{extract_fees, prove_mmr_update};
pub use errors::{PublisherError, ValidatorError};
