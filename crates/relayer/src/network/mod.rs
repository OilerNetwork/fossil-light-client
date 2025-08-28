use std::time::Duration;

use alloy::{
    primitives::{Address, TxHash, U256},
    sol_types::sol,
};
use async_trait::async_trait;

use crate::error::Result;

/// Contract interaction layer
pub mod contract;
/// Ethereum provider implementation
pub mod provider;

pub use contract::*;
pub use provider::*;

sol!(
    #[sol(rpc)]
    L1MessagesSender,
    "abi/L1MessagesSender.json"
);

/// Trait for Ethereum network operations
#[async_trait]
pub trait EthereumProvider: Send + Sync + std::fmt::Debug {
    /// Send a transaction and wait for confirmation
    async fn send_transaction(
        &self,
        contract_address: Address,
        recipient: U256,
        value: U256,
        timeout: Duration,
        confirmations: u64,
    ) -> Result<TxHash>;
}

/// Trait for L1 message sender contract operations
#[async_trait]
pub trait L1MessageSender: Send + Sync + std::fmt::Debug {
    /// Send finalized block hash to L2
    async fn send_finalized_block_hash(
        &self,
        recipient: U256,
        value: U256,
        timeout: Duration,
        confirmations: u64,
    ) -> Result<TxHash>;
}
