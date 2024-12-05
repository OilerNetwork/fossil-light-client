#![deny(unused_crate_dependencies)]
use clap as _;

pub mod api;
pub mod core;
pub mod db;
pub mod errors;
pub mod utils;
pub mod validator;

pub use api::operations::{prove_headers_integrity_and_inclusion, prove_mmr_update};
pub use errors::{PublisherError, ValidatorError};
