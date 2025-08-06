use std::time::Duration;

use alloy::primitives::{Address, TxHash, U256};
use async_trait::async_trait;

use crate::{
    error::{NetworkError, Result},
    network::{EthereumProvider, L1MessageSender},
};

/// Mock Ethereum provider for testing
#[derive(Debug, Clone)]
pub struct MockEthereumProvider {
    /// Whether this mock should fail
    pub should_fail: bool,
    /// Failure message to return
    pub failure_message: String,
    /// Expected transaction hash to return
    pub expected_tx_hash: TxHash,
    /// Call count tracker
    pub call_count: std::sync::Arc<std::sync::Mutex<usize>>,
}

impl Default for MockEthereumProvider {
    fn default() -> Self {
        Self {
            should_fail: false,
            failure_message: "Mock failure".to_string(),
            expected_tx_hash: TxHash::default(),
            call_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
        }
    }
}

impl MockEthereumProvider {
    /// Create a new mock provider
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure mock to fail with message
    pub fn with_failure(mut self, message: impl Into<String>) -> Self {
        self.should_fail = true;
        self.failure_message = message.into();
        self
    }

    /// Configure mock to succeed with tx hash
    pub fn with_success(mut self, tx_hash: TxHash) -> Self {
        self.should_fail = false;
        self.expected_tx_hash = tx_hash;
        self
    }

    /// Get number of times this mock was called
    pub fn get_call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

#[async_trait]
impl EthereumProvider for MockEthereumProvider {
    async fn send_transaction(
        &self,
        _contract_address: Address,
        _recipient: U256,
        _value: U256,
        _timeout: Duration,
        _confirmations: u64,
    ) -> Result<TxHash> {
        // Increment call count
        {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
        }

        if self.should_fail {
            Err(NetworkError::TransactionFailed(self.failure_message.clone()).into())
        } else {
            Ok(self.expected_tx_hash)
        }
    }
}

/// Mock L1 message sender for testing
#[derive(Debug, Clone)]
pub struct MockL1MessageSender {
    /// Whether this mock should fail
    pub should_fail: bool,
    /// Failure message to return
    pub failure_message: String,
    /// Expected transaction hash to return
    pub expected_tx_hash: TxHash,
    /// Call count tracker
    pub call_count: std::sync::Arc<std::sync::Mutex<usize>>,
}

impl Default for MockL1MessageSender {
    fn default() -> Self {
        Self {
            should_fail: false,
            failure_message: "Mock failure".to_string(),
            expected_tx_hash: TxHash::default(),
            call_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
        }
    }
}

impl MockL1MessageSender {
    /// Create a new mock L1 message sender
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure mock to fail with message
    pub fn with_failure(mut self, message: impl Into<String>) -> Self {
        self.should_fail = true;
        self.failure_message = message.into();
        self
    }

    /// Configure mock to succeed with tx hash
    pub fn with_success(mut self, tx_hash: TxHash) -> Self {
        self.should_fail = false;
        self.expected_tx_hash = tx_hash;
        self
    }

    /// Get number of times this mock was called
    pub fn get_call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

#[async_trait]
impl L1MessageSender for MockL1MessageSender {
    async fn send_finalized_block_hash(
        &self,
        _recipient: U256,
        _value: U256,
        _timeout: Duration,
        _confirmations: u64,
    ) -> Result<TxHash> {
        // Increment call count
        {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
        }

        if self.should_fail {
            Err(NetworkError::TransactionFailed(self.failure_message.clone()).into())
        } else {
            Ok(self.expected_tx_hash)
        }
    }
}
