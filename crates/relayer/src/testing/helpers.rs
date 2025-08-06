use std::{env, time::Duration};

use alloy::{
    primitives::{Address, U256},
    signers::local::PrivateKeySigner,
};

use crate::config::{RelayerConfig, RelayerConfigBuilder};

/// Test configuration builder with common test values
pub struct TestConfigBuilder {
    private_key: Option<String>,
    l2_address: Option<String>,
    eth_rpc_url: Option<String>,
    l1_message_sender: Option<String>,
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self {
            private_key: None,
            l2_address: None,
            eth_rpc_url: None,
            l1_message_sender: None,
        }
    }
}

impl TestConfigBuilder {
    /// Create a new test config builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set up with valid default test values
    pub fn with_valid_defaults() -> Self {
        Self {
            private_key: Some(
                "1234567890123456789012345678901234567890123456789012345678901234".to_string(),
            ),
            l2_address: Some(
                "0x07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc81".to_string(),
            ),
            eth_rpc_url: Some("http://localhost:8545".to_string()),
            l1_message_sender: Some("0x2345678901234567890123456789012345678901".to_string()),
        }
    }

    /// Set an invalid private key
    pub fn with_invalid_private_key(mut self) -> Self {
        self.private_key = Some("not_a_hex_string".to_string());
        self
    }

    /// Set an invalid L2 address
    pub fn with_invalid_l2_address(mut self) -> Self {
        self.l2_address = Some("not_an_address".to_string());
        self
    }

    /// Set a short (invalid) private key
    pub fn with_short_private_key(mut self) -> Self {
        self.private_key = Some("1234".to_string());
        self
    }

    /// Don't set private key (missing)
    pub fn with_missing_private_key(mut self) -> Self {
        self.private_key = None;
        self
    }

    /// Don't set L2 address (missing)
    pub fn with_missing_l2_address(mut self) -> Self {
        self.l2_address = None;
        self
    }

    /// Apply the configuration to environment variables
    pub fn build_env_vars(self) -> TestEnvironment {
        TestEnvironment::new(self)
    }

    /// Build a RelayerConfig directly (for testing config builder)
    pub fn build_config(self) -> Result<RelayerConfig, String> {
        let private_key = self
            .private_key
            .ok_or("private_key is required")?
            .parse::<PrivateKeySigner>()
            .map_err(|e| format!("Invalid private key: {}", e))?;

        let l2_address = self.l2_address.ok_or("l2_address is required")?;

        let l2_addr = if l2_address.starts_with("0x") && l2_address.len() == 66 {
            U256::from_str_radix(&l2_address[2..], 16)
                .map_err(|e| format!("Invalid L2 address: {}", e))?
        } else {
            return Err("Invalid L2 address format".to_string());
        };

        let eth_rpc_url = self.eth_rpc_url.ok_or("eth_rpc_url is required")?;

        let l1_message_sender = self
            .l1_message_sender
            .ok_or("l1_message_sender is required")?
            .parse::<Address>()
            .map_err(|e| format!("Invalid L1 message sender: {}", e))?;

        RelayerConfigBuilder::default()
            .private_key(private_key)
            .l2_recipient_addr(l2_addr)
            .eth_rpc_url(eth_rpc_url)
            .l1_message_sender(l1_message_sender)
            .transaction_value(U256::from(30000))
            .confirmation_timeout(Duration::from_secs(60))
            .required_confirmations(1)
            .build()
            .map_err(|e| e.to_string())
    }
}

/// Manages test environment variables
pub struct TestEnvironment {
    vars_to_clean: Vec<String>,
}

impl TestEnvironment {
    fn new(config: TestConfigBuilder) -> Self {
        let mut vars_to_clean = Vec::new();

        if let Some(key) = config.private_key {
            env::set_var("ACCOUNT_PRIVATE_KEY", key);
            vars_to_clean.push("ACCOUNT_PRIVATE_KEY".to_string());
        }

        if let Some(addr) = config.l2_address {
            env::set_var("L2_MSG_PROXY", addr);
            vars_to_clean.push("L2_MSG_PROXY".to_string());
        }

        if let Some(url) = config.eth_rpc_url {
            env::set_var("ETH_RPC_URL", url);
            vars_to_clean.push("ETH_RPC_URL".to_string());
        }

        if let Some(addr) = config.l1_message_sender {
            env::set_var("L1_MESSAGE_SENDER", addr);
            vars_to_clean.push("L1_MESSAGE_SENDER".to_string());
        }

        Self { vars_to_clean }
    }

    /// Manually clear all environment variables
    pub fn clear(&self) {
        for var in &self.vars_to_clean {
            env::remove_var(var);
        }
    }

    /// Verify environment is clean
    pub fn verify_clean(&self) {
        for var in &self.vars_to_clean {
            assert!(
                env::var(var).is_err(),
                "Environment variable {} should be clean",
                var
            );
        }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Clear all relevant environment variables for tests
pub fn clear_test_env() {
    let vars = [
        "ACCOUNT_PRIVATE_KEY",
        "L2_MSG_PROXY",
        "ETH_RPC_URL",
        "L1_MESSAGE_SENDER",
    ];

    for var in &vars {
        env::remove_var(var);
    }

    // Verify cleanup
    for var in &vars {
        assert!(env::var(var).is_err(), "Failed to clear {}", var);
    }
}
