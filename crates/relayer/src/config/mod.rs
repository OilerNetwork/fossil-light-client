use std::time::Duration;

use alloy::{
    primitives::{Address, U256},
    signers::local::PrivateKeySigner,
};
use common::{get_env_var, get_var};

use crate::error::{ConfigError, Result};

/// Configuration for the relayer application
#[derive(Debug, Clone)]
pub struct RelayerConfig {
    /// Private key signer for transactions
    pub private_key: PrivateKeySigner,

    /// L2 recipient address for block hash messages
    pub l2_recipient_addr: U256,

    /// Ethereum RPC URL
    pub eth_rpc_url: String,

    /// L1 message sender contract address
    pub l1_message_sender: Address,

    /// Transaction value in Wei
    pub transaction_value: U256,

    /// Timeout for transaction confirmations
    pub confirmation_timeout: Duration,

    /// Required number of confirmations
    pub required_confirmations: u64,
}

impl RelayerConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let private_key = Self::load_private_key()?;
        let l2_recipient_addr = Self::load_l2_address()?;
        let eth_rpc_url = Self::load_eth_rpc_url()?;
        let l1_message_sender = Self::load_l1_message_sender()?;

        Ok(Self {
            private_key,
            l2_recipient_addr,
            eth_rpc_url,
            l1_message_sender,
            transaction_value: U256::from(30000), // Default value
            confirmation_timeout: Duration::from_secs(60), // Default timeout
            required_confirmations: 1,            // Default confirmations
        })
    }

    /// Load and validate private key from environment
    fn load_private_key() -> Result<PrivateKeySigner> {
        get_var("ACCOUNT_PRIVATE_KEY")
            .map_err(|e| ConfigError::InvalidPrivateKey(e.to_string()).into())
    }

    /// Load and validate L2 address from environment
    fn load_l2_address() -> Result<U256> {
        let addr_str = get_env_var("L2_MSG_PROXY")
            .map_err(|e| ConfigError::MissingEnvVar(format!("L2_MSG_PROXY: {e}")))?;

        Self::validate_starknet_address(&addr_str)?;

        U256::from_str_radix(&addr_str[2..], 16).map_err(|e| {
            ConfigError::InvalidL2Address(format!("Invalid hex characters: {e}")).into()
        })
    }

    /// Load and validate Ethereum RPC URL
    fn load_eth_rpc_url() -> Result<String> {
        let url = get_env_var("ETH_RPC_URL")
            .map_err(|e| ConfigError::MissingEnvVar(format!("ETH_RPC_URL: {e}")))?;

        // Basic URL validation
        if !url.starts_with("http://")
            && !url.starts_with("https://")
            && !url.starts_with("ws://")
            && !url.starts_with("wss://")
        {
            return Err(ConfigError::InvalidUrl(format!(
                "URL must start with http://, https://, ws://, or wss://: {url}"
            ))
            .into());
        }

        Ok(url)
    }

    /// Load L1 message sender contract address
    fn load_l1_message_sender() -> Result<Address> {
        get_var("L1_MESSAGE_SENDER")
            .map_err(|e| ConfigError::InvalidAddress(format!("L1_MESSAGE_SENDER: {e}")).into())
    }

    /// Validate Starknet address format
    fn validate_starknet_address(addr_str: &str) -> Result<()> {
        if !addr_str.starts_with("0x") || addr_str.len() != 66 {
            return Err(ConfigError::InvalidL2Address(format!(
                "Expected 0x + 64 hex chars, got {addr_str}"
            ))
            .into());
        }
        Ok(())
    }
}

impl RelayerConfig {
    /// Builder for creating custom configurations (mainly for testing)
    pub fn builder() -> RelayerConfigBuilder {
        RelayerConfigBuilder::default()
    }
}

/// Builder for `RelayerConfig`
#[derive(Default)]
pub struct RelayerConfigBuilder {
    private_key: Option<PrivateKeySigner>,
    l2_recipient_addr: Option<U256>,
    eth_rpc_url: Option<String>,
    l1_message_sender: Option<Address>,
    transaction_value: Option<U256>,
    confirmation_timeout: Option<Duration>,
    required_confirmations: Option<u64>,
}

impl RelayerConfigBuilder {
    /// Set the private key signer
    pub fn private_key(mut self, key: PrivateKeySigner) -> Self {
        self.private_key = Some(key);
        self
    }

    /// Set the L2 recipient address
    pub const fn l2_recipient_addr(mut self, addr: U256) -> Self {
        self.l2_recipient_addr = Some(addr);
        self
    }

    /// Set the Ethereum RPC URL
    pub fn eth_rpc_url(mut self, url: impl Into<String>) -> Self {
        self.eth_rpc_url = Some(url.into());
        self
    }

    /// Set the L1 message sender contract address
    pub const fn l1_message_sender(mut self, addr: Address) -> Self {
        self.l1_message_sender = Some(addr);
        self
    }

    /// Set the transaction value in Wei
    pub const fn transaction_value(mut self, value: U256) -> Self {
        self.transaction_value = Some(value);
        self
    }

    /// Set the confirmation timeout
    pub const fn confirmation_timeout(mut self, timeout: Duration) -> Self {
        self.confirmation_timeout = Some(timeout);
        self
    }

    /// Set the required number of confirmations
    pub const fn required_confirmations(mut self, confirmations: u64) -> Self {
        self.required_confirmations = Some(confirmations);
        self
    }

    /// Build the `RelayerConfig`
    pub fn build(self) -> std::result::Result<RelayerConfig, &'static str> {
        Ok(RelayerConfig {
            private_key: self.private_key.ok_or("private_key is required")?,
            l2_recipient_addr: self
                .l2_recipient_addr
                .ok_or("l2_recipient_addr is required")?,
            eth_rpc_url: self.eth_rpc_url.ok_or("eth_rpc_url is required")?,
            l1_message_sender: self
                .l1_message_sender
                .ok_or("l1_message_sender is required")?,
            transaction_value: self.transaction_value.unwrap_or_else(|| U256::from(30000)),
            confirmation_timeout: self.confirmation_timeout.unwrap_or(Duration::from_secs(60)),
            required_confirmations: self.required_confirmations.unwrap_or(1),
        })
    }
}
