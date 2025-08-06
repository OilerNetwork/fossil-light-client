//! # Publisher Crate
//!
//! The Publisher crate is a core component of the Fossil Light Client that handles:
//!
//! - **MMR (Merkle Mountain Range) operations**: Building, updating, and managing MMR data structures for blockchain state
//! - **Proof generation**: Creating cryptographic proofs for block hash inclusion and MMR updates
//! - **Block hash verification**: Validating and proving the inclusion of specific block hashes in the MMR
//! - **Starknet integration**: Interfacing with Starknet networks for state management and proof submission
//!
//! ## Core Components
//!
//! ### Configuration Management
//! The crate provides flexible configuration management through:
//! - [`PublisherConfig`] - Main configuration structure with builder pattern support
//! - [`AccountConfig`] - Account-specific settings for Starknet operations
//!
//! ### Service Layer
//! Business logic is organized into services:
//! - [`ProofService`] - Handles proof generation and MMR operations
//! - [`MmrService`] - Specialized MMR state management (when available)
//!
//! ### API Layer
//! Public API functions maintain backward compatibility:
//! - [`prove_mmr_update`] - Generate and submit MMR update proofs
//! - [`update_mmr`] - Update MMR without proof generation
//! - [`get_block_hash_inclusion_proof`] - Get inclusion proof for a specific block hash
//!
//! ## Quick Start
//!
//! ### Basic Configuration
//!
//! ```rust
//! use publisher::{PublisherConfig, AccountConfig, ProofService};
//!
//! Create configuration using builder pattern
//! let config = PublisherConfig::builder()
//! .rpc_url("http://localhost:8545")
//! .chain_id(1)
//! .verifier_address("0x1234567890123456789012345678901234567890")
//! .store_address("0x0987654321098765432109876543210987654321")
//! .batch_size(100)
//! .build()?;
//!
//! Create service instance
//! let service = ProofService::with_config(config);
//! ```
//!
//! ### Generating Block Hash Proofs
//!
//! ```rust
//! use publisher::get_block_hash_inclusion_proof;
//!
//! let block_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
//! let rpc_url = "http://localhost:8545";
//! let store_address = "0x0987654321098765432109876543210987654321";
//! let batch_size = 100;
//!
//! let proof_response = get_block_hash_inclusion_proof(
//! block_hash.to_string(),
//! rpc_url.to_string(),
//! store_address.to_string(),
//! batch_size,
//! ).await?;
//!
//! println!("Proof generated for batch: {}", proof_response.batch_index);
//! ```
//!
//! ### Error Handling
//!
//! The crate provides comprehensive error handling through [`PublisherError`]:
//!
//! ```rust
//! use publisher::{PublisherResult, PublisherError};
//!
//! fn handle_publisher_operation() -> PublisherResult<()> {
//! Operation that might fail
//! Err(PublisherError::database("Connection failed"))
//! }
//!
//! match handle_publisher_operation() {
//! Ok(()) => println!("Operation succeeded"),
//! Err(PublisherError::Database(msg)) => println!("Database error: {}", msg),
//! Err(PublisherError::Ipfs(msg)) => println!("IPFS error: {}", msg),
//! Err(err) => println!("Other error: {}", err),
//! }
//! ```
//!
//! ## Architecture
//!
//! The crate follows a layered architecture:
//!
//! 1. **API Layer** (`api/`) - Public interface functions
//! 2. **Service Layer** (`service/`) - Business logic and orchestration
//! 3. **Core Layer** (`core/`) - Core MMR and proof generation logic
//! 4. **Database Layer** (`db/`) - Database access and management
//! 5. **Configuration** (`config/`) - Configuration management
//! 6. **Error Handling** (`error/`) - Centralized error types
//!
//! ## Testing
//!
//! The crate includes comprehensive testing utilities:
//!
//! ```rust
//! #[cfg(test)]
//! mod tests {
//! use publisher::testing::{create_test_config, create_test_account_config};
//!
//! #[tokio::test]
//! async fn test_service_creation() {
//! let config = create_test_config();
//! let service = ProofService::with_config(config);
//! Test logic here...
//! }
//! }
//! ```
//!
//! ## Backward Compatibility
//!
//! All public APIs maintain backward compatibility. Legacy function signatures are preserved
//! while internally using the new configuration and service architecture.

#![deny(unused_crate_dependencies)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use clap as _;
use dotenv as _;
use guest_mmr as _;
use pyo3 as _;
use tracing_subscriber as _;
/// API layer for publishing operations
pub mod api;
/// CLI interface for MMR operations
pub mod cli;
pub mod config;
/// Core MMR processing functionality
pub mod core;
/// Database access and management
pub mod db;
/// Error types and handling
pub mod error;
/// Service layer for business logic
pub mod service;
#[cfg(test)]
pub mod testing;
/// Trait abstractions for dependency injection
pub mod traits;
/// Utility types and helpers
pub mod utils;
// Note: validator module is not included as it has dependency issues and is not currently used

pub use api::operations::{get_block_hash_inclusion_proof, prove_mmr_update};
pub use config::{AccountConfig, PublisherConfig, PublisherConfigBuilder};
pub use error::{PublisherError, PublisherResult, Result, ValidatorError};
