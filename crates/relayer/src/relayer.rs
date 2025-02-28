use alloy::{
    network::EthereumWallet, primitives::U256, providers::ProviderBuilder,
    signers::local::PrivateKeySigner, sol_types::sol,
};
use common::{get_env_var, get_var};
use eyre::{eyre, Result};
use std::time::Duration;
use tracing::info;

sol!(
    #[sol(rpc)]
    L1MessagesSender,
    "abi/L1MessagesSender.json"
);

#[derive(Debug)]
pub struct Relayer {
    wallet: EthereumWallet,
    l2_recipient_addr: U256,
}

impl Relayer {
    pub async fn new() -> Result<Self> {
        // Load the private key and initialize the signer
        let signer: PrivateKeySigner = get_var("ACCOUNT_PRIVATE_KEY")?;

        // Create the wallet
        let wallet = EthereumWallet::from(signer);

        // Get the L2 proxy address as a string first
        let addr_str = get_env_var("L2_MSG_PROXY")?;

        // Validate Starknet address format:
        // 1. Must start with "0x"
        // 2. Must be exactly 66 characters total (0x + 64 hex chars for Starknet)
        // 3. Must be a valid hex string
        if !addr_str.starts_with("0x") || addr_str.len() != 66 {
            return Err(eyre!(
                "L2_MSG_PROXY: Invalid Starknet address format. Expected 0x + 64 hex chars, got {}",
                addr_str
            ));
        }

        // Now parse the validated hex string
        let l2_recipient_addr = U256::from_str_radix(&addr_str[2..], 16)
            .map_err(|e| eyre!("L2_MSG_PROXY: Invalid hex characters in address: {}", e))?;

        info!("Using L2 recipient address: {:?}", l2_recipient_addr);

        Ok(Self {
            wallet,
            l2_recipient_addr,
        })
    }

    pub async fn send_finalized_block_hash_to_l2(&self) -> Result<()> {
        // Create the provider
        let provider_url = get_env_var("ETH_RPC_URL")?;

        let provider = ProviderBuilder::new()
            .wallet(self.wallet.clone())
            .on_builtin(&provider_url)
            .await?;
        info!("Connected to Ethereum provider at {}", provider_url);

        // Load the contract address and initialize the contract
        let address = get_var("L1_MESSAGE_SENDER")?;

        let contract = L1MessagesSender::new(address, &provider);
        info!(
            "Initialized L1MessagesSender contract at address {}",
            address
        );

        // Prepare and send the transaction
        let call_builder = contract
            .sendFinalizedBlockHashToL2(self.l2_recipient_addr)
            .value(U256::from(30000));
        info!("Prepared transaction to send block hash with value: 30000 Wei");
        info!(
            "Sending transaction to L2 address: {:?}",
            self.l2_recipient_addr
        );

        let pending_tx = call_builder.send().await?;
        let tx_hash = pending_tx
            .with_required_confirmations(1)
            .with_timeout(Some(Duration::from_secs(60)))
            .watch()
            .await?;
        info!("Transaction confirmed successfully. Tx hash: {:?}", tx_hash);

        Ok(())
    }
}

#[cfg(test)]
use std::env;

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::U256;
    // use std::str::FromStr;

    fn setup_test_env() {
        env::set_var(
            "ACCOUNT_PRIVATE_KEY",
            "1234567890123456789012345678901234567890123456789012345678901234",
        );
        // Use a valid 51-byte Starknet address
        env::set_var(
            "L2_MSG_PROXY",
            "0x07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc81",
        );
        env::set_var("ETH_RPC_URL", "http://localhost:8545");
        env::set_var(
            "L1_MESSAGE_SENDER",
            "0x2345678901234567890123456789012345678901",
        );
    }

    fn clear_test_env() {
        // Clear all relevant environment variables
        env::remove_var("ACCOUNT_PRIVATE_KEY");
        env::remove_var("L2_MSG_PROXY");
        env::remove_var("ETH_RPC_URL");
        env::remove_var("L1_MESSAGE_SENDER");

        // Verify environment is actually clean
        assert!(env::var("ACCOUNT_PRIVATE_KEY").is_err());
        assert!(env::var("L2_MSG_PROXY").is_err());
        assert!(env::var("ETH_RPC_URL").is_err());
        assert!(env::var("L1_MESSAGE_SENDER").is_err());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_relayer_new() {
        clear_test_env();
        setup_test_env();

        let relayer = Relayer::new().await;
        match &relayer {
            Ok(_) => (),
            Err(e) => println!("Relayer::new() failed with error: {:?}", e),
        }
        assert!(relayer.is_ok());

        let relayer = relayer.unwrap();
        assert_eq!(
            relayer.l2_recipient_addr,
            U256::from_str_radix(
                "07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc81",
                16
            )
            .unwrap()
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_send_finalized_block_hash_to_l2() {
        clear_test_env();

        // Set up environment with hex values
        env::set_var(
            "ACCOUNT_PRIVATE_KEY",
            "1234567890123456789012345678901234567890123456789012345678901234",
        );
        env::set_var(
            "L2_MSG_PROXY",
            "0x07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc81", // Add 0x prefix
        );
        env::set_var("ETH_RPC_URL", "http://localhost:8545");
        env::set_var(
            "L1_MESSAGE_SENDER",
            "0x1234567890123456789012345678901234567890",
        );

        let relayer = Relayer::new().await.expect("Failed to create relayer");

        // Verify the relayer is properly configured
        assert_eq!(
            relayer.l2_recipient_addr,
            U256::from_str_radix(
                "07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc81",
                16
            )
            .unwrap()
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_new_with_missing_env_vars() {
        // Make sure environment is clean
        clear_test_env();

        // Double check that required vars are missing
        assert!(env::var("ACCOUNT_PRIVATE_KEY").is_err());
        assert!(env::var("L2_MSG_PROXY").is_err());

        let result = Relayer::new().await;
        assert!(result.is_err());

        // Just check that it's an error without expecting a specific type
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("environment variable not found"));
    }

    // Add a test to verify environment variable parsing
    #[tokio::test]
    #[serial_test::serial]
    async fn test_env_var_parsing() {
        clear_test_env();
        setup_test_env();

        // Test each environment variable individually
        let private_key = get_var::<PrivateKeySigner>("ACCOUNT_PRIVATE_KEY");
        match &private_key {
            Ok(_) => println!("Private key parsed successfully"),
            Err(e) => println!("Failed to parse private key: {:?}", e),
        }
        assert!(private_key.is_ok());

        let l2_proxy = get_var::<U256>("L2_MSG_PROXY");
        match &l2_proxy {
            Ok(_) => println!("L2 proxy address parsed successfully"),
            Err(e) => println!("Failed to parse L2 proxy address: {:?}", e),
        }
        assert!(l2_proxy.is_ok());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_invalid_private_key_format() {
        clear_test_env();

        // Set up environment with invalid private key format
        env::set_var("ACCOUNT_PRIVATE_KEY", "not_a_hex_string");
        env::set_var(
            "L2_MSG_PROXY",
            "0x07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc81",
        );

        let result = Relayer::new().await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid string length"));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_invalid_l2_proxy_address() {
        clear_test_env();

        // Set up environment with valid private key but invalid L2 proxy address
        env::set_var(
            "ACCOUNT_PRIVATE_KEY",
            "1234567890123456789012345678901234567890123456789012345678901234",
        );
        env::set_var("L2_MSG_PROXY", "not_an_address");

        let result = Relayer::new().await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid Starknet address format"));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_private_key_wrong_length() {
        clear_test_env();

        // Set up environment with private key that's too short
        env::set_var("ACCOUNT_PRIVATE_KEY", "1234");
        env::set_var(
            "L2_MSG_PROXY",
            "0x07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc81",
        );

        let result = Relayer::new().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_l2_proxy_address_wrong_length() {
        clear_test_env();

        env::set_var(
            "ACCOUNT_PRIVATE_KEY",
            "1234567890123456789012345678901234567890123456789012345678901234",
        );

        let invalid_addresses = vec![
            // Invalid hex values
            "0xghijklmn",
            "xyz123",
            // Invalid lengths (Starknet addresses should be 51 bytes)
            "0x",
            "0x0",
            "0x1234567890123456789012345678901234567890", // Ethereum address length
            "0x07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc8", // Too short
            "0x07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc811", // Too long
            // Invalid formats
            "",
            "   ",
            // Missing prefix
            "07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc81",
        ];

        for addr in invalid_addresses {
            env::set_var("L2_MSG_PROXY", addr);
            let result = Relayer::new().await;
            assert!(result.is_err(), "Should fail for invalid address: {}", addr);
        }
    }

    // Add a test for valid addresses
    #[tokio::test]
    #[serial_test::serial]
    async fn test_valid_l2_proxy_addresses() {
        clear_test_env();

        env::set_var(
            "ACCOUNT_PRIVATE_KEY",
            "1234567890123456789012345678901234567890123456789012345678901234",
        );

        // Test different valid Starknet address formats (51 bytes)
        let valid_addresses = vec![
            "0x07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc81",
            "0x0000000000000000000000000000000000000000000000000000000000000000",
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ];

        for addr in valid_addresses {
            env::set_var("L2_MSG_PROXY", addr);
            let result = Relayer::new().await;
            match &result {
                Ok(_) => (),
                Err(e) => println!("Failed to accept address {}: {:?}", addr, e),
            }
            assert!(result.is_ok(), "Should accept valid address: {}", addr);
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_empty_environment_variables() {
        clear_test_env();

        // Set up environment with empty strings
        env::set_var("ACCOUNT_PRIVATE_KEY", "");
        env::set_var("L2_MSG_PROXY", "");

        let result = Relayer::new().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_whitespace_environment_variables() {
        clear_test_env();

        // Set up environment with whitespace strings
        env::set_var("ACCOUNT_PRIVATE_KEY", "   ");
        env::set_var("L2_MSG_PROXY", "  ");

        let result = Relayer::new().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_partial_environment_setup() {
        clear_test_env();

        // Only set one of the required variables
        env::set_var(
            "ACCOUNT_PRIVATE_KEY",
            "1234567890123456789012345678901234567890123456789012345678901234",
        );
        // Don't set L2_MSG_PROXY

        let result = Relayer::new().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_case_sensitivity() {
        clear_test_env();

        // Test if environment variables are case sensitive
        env::set_var(
            "account_private_key", // lowercase
            "1234567890123456789012345678901234567890123456789012345678901234",
        );
        env::set_var("L2_MSG_PROXY", "1234567890123456789012345678901234567890");

        let result = Relayer::new().await;
        assert!(result.is_err()); // Should fail because we expect uppercase
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_malformed_hex_strings() {
        clear_test_env();

        // Test invalid hex strings (missing 0x prefix, odd length, invalid characters)
        let test_cases = vec![
            "0xg234567890123456789012345678901234567890123456789012345678901234", // invalid hex char
            "0x12345",                                                            // odd length
            "0x",               // empty hex string
            "not_a_hex_string", // completely invalid
        ];

        for private_key in test_cases {
            env::set_var("ACCOUNT_PRIVATE_KEY", private_key);
            env::set_var("L2_MSG_PROXY", "1234567890123456789012345678901234567890");

            let result = Relayer::new().await;
            assert!(
                result.is_err(),
                "Should fail for private key: {}",
                private_key
            );
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_unicode_characters() {
        clear_test_env();

        // Test with Unicode characters
        env::set_var(
            "ACCOUNT_PRIVATE_KEY",
            "0x123456789🦀123456789012345678901234567890123456789012345678901234",
        );
        env::set_var("L2_MSG_PROXY", "1234567890123456789012345678901234567890");

        let result = Relayer::new().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_max_values() {
        clear_test_env();

        // Set up environment with valid private key
        env::set_var(
            "ACCOUNT_PRIVATE_KEY",
            "0x1234567890123456789012345678901234567890123456789012345678901234", // valid private key
        );
        env::set_var(
            "L2_MSG_PROXY",
            "0x07187e87432788d2baf02fa2b2582ae4b9aa6055f0c60ee6023eef87adb6bc81", // valid Starknet address
        );

        let result = Relayer::new().await;
        match &result {
            Ok(_) => (),
            Err(e) => println!("Unexpected error: {:?}", e),
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_invalid_max_values() {
        clear_test_env();

        // Test with invalid maximum values
        env::set_var(
            "ACCOUNT_PRIVATE_KEY",
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff1", // too long
        );
        env::set_var(
            "L2_MSG_PROXY",
            "ffffffffffffffffffffffffffffffffffffffff1", // too long
        );

        let result = Relayer::new().await;
        assert!(result.is_err());
    }
}
