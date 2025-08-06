use std::time::Duration;

use alloy::primitives::{TxHash, U256};
use async_trait::async_trait;

use super::{EthereumProvider, L1MessageSender};
use crate::error::Result;

/// Implementation of `L1MessageSender` using `EthereumProvider`
#[derive(Debug)]
pub struct L1MessageSenderContract {
    provider: Box<dyn EthereumProvider>,
    contract_address: alloy::primitives::Address,
}

impl L1MessageSenderContract {
    /// Create a new `L1MessageSender` contract instance
    pub fn new(
        provider: Box<dyn EthereumProvider>,
        contract_address: alloy::primitives::Address,
    ) -> Self {
        Self {
            provider,
            contract_address,
        }
    }
}

#[async_trait]
impl L1MessageSender for L1MessageSenderContract {
    async fn send_finalized_block_hash(
        &self,
        recipient: U256,
        value: U256,
        timeout: Duration,
        confirmations: u64,
    ) -> Result<TxHash> {
        self.provider
            .send_transaction(
                self.contract_address,
                recipient,
                value,
                timeout,
                confirmations,
            )
            .await
    }
}
