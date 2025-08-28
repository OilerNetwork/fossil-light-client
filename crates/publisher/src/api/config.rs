//! Configuration builders for API operations
//!
//! This module provides builder patterns for API function arguments to avoid
//! the clippy warning about having too many arguments (> 7).

use starknet_handler::provider::LatestRelayBlock;

/// Builder for `prove_mmr_update` function arguments
#[derive(Debug, Clone)]
pub struct ProveMMRUpdateConfigBuilder {
    rpc_url: Option<String>,
    chain_id: Option<u64>,
    verifier_address: Option<String>,
    store_address: Option<String>,
    account_private_key: Option<String>,
    account_address: Option<String>,
    batch_size: Option<u64>,
    start_block: Option<u64>,
    latest_relayed_block: Option<LatestRelayBlock>,
}

/// Configuration for `prove_mmr_update` function
#[derive(Debug, Clone)]
pub struct ProveMMRUpdateConfig {
    /// RPC endpoint URL for the blockchain network
    pub rpc_url: String,
    /// Chain ID of the target network
    pub chain_id: u64,
    /// Address of the verifier contract on Starknet
    pub verifier_address: String,
    /// Address of the store contract on Starknet
    pub store_address: String,
    /// Private key for the Starknet account
    pub account_private_key: String,
    /// Address of the Starknet account
    pub account_address: String,
    /// Number of blocks to process in each batch
    pub batch_size: u64,
    /// Starting block number for the update
    pub start_block: u64,
    /// Latest block information from the relay
    pub latest_relayed_block: LatestRelayBlock,
}

impl ProveMMRUpdateConfigBuilder {
    /// Create a new builder
    pub const fn new() -> Self {
        Self {
            rpc_url: None,
            chain_id: None,
            verifier_address: None,
            store_address: None,
            account_private_key: None,
            account_address: None,
            batch_size: None,
            start_block: None,
            latest_relayed_block: None,
        }
    }

    /// Set the RPC URL
    pub fn rpc_url<S: Into<String>>(mut self, rpc_url: S) -> Self {
        self.rpc_url = Some(rpc_url.into());
        self
    }

    /// Set the chain ID
    pub const fn chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Set the verifier address
    pub fn verifier_address<S: Into<String>>(mut self, verifier_address: S) -> Self {
        self.verifier_address = Some(verifier_address.into());
        self
    }

    /// Set the store address
    pub fn store_address<S: Into<String>>(mut self, store_address: S) -> Self {
        self.store_address = Some(store_address.into());
        self
    }

    /// Set the account private key
    pub fn account_private_key<S: Into<String>>(mut self, account_private_key: S) -> Self {
        self.account_private_key = Some(account_private_key.into());
        self
    }

    /// Set the account address
    pub fn account_address<S: Into<String>>(mut self, account_address: S) -> Self {
        self.account_address = Some(account_address.into());
        self
    }

    /// Set the batch size
    pub const fn batch_size(mut self, batch_size: u64) -> Self {
        self.batch_size = Some(batch_size);
        self
    }

    /// Set the start block
    pub const fn start_block(mut self, start_block: u64) -> Self {
        self.start_block = Some(start_block);
        self
    }

    /// Set the latest relayed block
    pub fn latest_relayed_block(mut self, latest_relayed_block: LatestRelayBlock) -> Self {
        self.latest_relayed_block = Some(latest_relayed_block);
        self
    }

    /// Build the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if any required field is missing
    pub fn build(self) -> Result<ProveMMRUpdateConfig, String> {
        Ok(ProveMMRUpdateConfig {
            rpc_url: self.rpc_url.ok_or("rpc_url is required")?,
            chain_id: self.chain_id.ok_or("chain_id is required")?,
            verifier_address: self
                .verifier_address
                .ok_or("verifier_address is required")?,
            store_address: self.store_address.ok_or("store_address is required")?,
            account_private_key: self
                .account_private_key
                .ok_or("account_private_key is required")?,
            account_address: self.account_address.ok_or("account_address is required")?,
            batch_size: self.batch_size.ok_or("batch_size is required")?,
            start_block: self.start_block.ok_or("start_block is required")?,
            latest_relayed_block: self
                .latest_relayed_block
                .ok_or("latest_relayed_block is required")?,
        })
    }
}

impl Default for ProveMMRUpdateConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for `update_mmr` function arguments
#[derive(Debug, Clone)]
pub struct UpdateMMRConfigBuilder {
    rpc_url: Option<String>,
    chain_id: Option<u64>,
    verifier_address: Option<String>,
    store_address: Option<String>,
    account_private_key: Option<String>,
    account_address: Option<String>,
    batch_size: Option<u64>,
    start_block: Option<u64>,
    latest_relayed_block: Option<LatestRelayBlock>,
}

/// Configuration for `update_mmr` function
#[derive(Debug, Clone)]
pub struct UpdateMMRConfig {
    /// RPC endpoint URL for the blockchain network
    pub rpc_url: String,
    /// Chain ID of the target network
    pub chain_id: u64,
    /// Address of the verifier contract on Starknet
    pub verifier_address: String,
    /// Address of the store contract on Starknet
    pub store_address: String,
    /// Private key for the Starknet account
    pub account_private_key: String,
    /// Address of the Starknet account
    pub account_address: String,
    /// Number of blocks to process in each batch
    pub batch_size: u64,
    /// Starting block number for the update
    pub start_block: u64,
    /// Latest block information from the relay
    pub latest_relayed_block: LatestRelayBlock,
}

impl UpdateMMRConfigBuilder {
    /// Create a new builder
    pub const fn new() -> Self {
        Self {
            rpc_url: None,
            chain_id: None,
            verifier_address: None,
            store_address: None,
            account_private_key: None,
            account_address: None,
            batch_size: None,
            start_block: None,
            latest_relayed_block: None,
        }
    }

    /// Set the RPC URL
    pub fn rpc_url<S: Into<String>>(mut self, rpc_url: S) -> Self {
        self.rpc_url = Some(rpc_url.into());
        self
    }

    /// Set the chain ID
    pub const fn chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Set the verifier address
    pub fn verifier_address<S: Into<String>>(mut self, verifier_address: S) -> Self {
        self.verifier_address = Some(verifier_address.into());
        self
    }

    /// Set the store address
    pub fn store_address<S: Into<String>>(mut self, store_address: S) -> Self {
        self.store_address = Some(store_address.into());
        self
    }

    /// Set the account private key
    pub fn account_private_key<S: Into<String>>(mut self, account_private_key: S) -> Self {
        self.account_private_key = Some(account_private_key.into());
        self
    }

    /// Set the account address
    pub fn account_address<S: Into<String>>(mut self, account_address: S) -> Self {
        self.account_address = Some(account_address.into());
        self
    }

    /// Set the batch size
    pub const fn batch_size(mut self, batch_size: u64) -> Self {
        self.batch_size = Some(batch_size);
        self
    }

    /// Set the start block
    pub const fn start_block(mut self, start_block: u64) -> Self {
        self.start_block = Some(start_block);
        self
    }

    /// Set the latest relayed block
    pub fn latest_relayed_block(mut self, latest_relayed_block: LatestRelayBlock) -> Self {
        self.latest_relayed_block = Some(latest_relayed_block);
        self
    }

    /// Build the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if any required field is missing
    pub fn build(self) -> Result<UpdateMMRConfig, String> {
        Ok(UpdateMMRConfig {
            rpc_url: self.rpc_url.ok_or("rpc_url is required")?,
            chain_id: self.chain_id.ok_or("chain_id is required")?,
            verifier_address: self
                .verifier_address
                .ok_or("verifier_address is required")?,
            store_address: self.store_address.ok_or("store_address is required")?,
            account_private_key: self
                .account_private_key
                .ok_or("account_private_key is required")?,
            account_address: self.account_address.ok_or("account_address is required")?,
            batch_size: self.batch_size.ok_or("batch_size is required")?,
            start_block: self.start_block.ok_or("start_block is required")?,
            latest_relayed_block: self
                .latest_relayed_block
                .ok_or("latest_relayed_block is required")?,
        })
    }
}

impl Default for UpdateMMRConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
