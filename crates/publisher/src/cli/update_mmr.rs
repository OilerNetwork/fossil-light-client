use crate::api::operations::prove_mmr_update;
use clap::Parser;
use common::get_env_var;
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Start block
    #[arg(short = 's', long)]
    pub start: u64,

    /// End block
    #[arg(short = 'e', long)]
    pub end: u64,

    /// Skip proof verification
    #[arg(short = 'p', long, default_value_t = false)]
    pub skip_proof: bool,

    /// Number of blocks to process in each batch
    #[arg(short = 'b', long, default_value_t = 1024)]
    pub batch_size: u64,
}

pub struct Config {
    pub chain_id: u64,
    pub rpc_url: String,
    pub verifier_address: String,
    pub store_address: String,
    pub private_key: String,
    pub account_address: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            chain_id: get_env_var("CHAIN_ID")?.parse()?,
            rpc_url: get_env_var("STARKNET_RPC_URL")?,
            verifier_address: get_env_var("FOSSIL_VERIFIER")?,
            store_address: get_env_var("FOSSIL_STORE")?,
            private_key: get_env_var("STARKNET_PRIVATE_KEY")?,
            account_address: get_env_var("STARKNET_ACCOUNT_ADDRESS")?,
        })
    }
}

pub async fn run(config: Config, args: Args) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Publisher...");

    prove_mmr_update(
        &config.rpc_url,
        config.chain_id,
        &config.verifier_address,
        &config.store_address,
        &config.private_key,
        &config.account_address,
        args.batch_size,
        args.start,
        args.end,
        args.skip_proof,
    )
    .await?;

    info!("MMR building completed");
    info!("Host finished");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Helper function to set up environment variables with valid test values
    fn setup_test_env() {
        env::remove_var("CHAIN_ID");
        env::remove_var("STARKNET_RPC_URL");
        env::remove_var("FOSSIL_VERIFIER");
        env::remove_var("FOSSIL_STORE");
        env::remove_var("STARKNET_PRIVATE_KEY");
        env::remove_var("STARKNET_ACCOUNT_ADDRESS");
    }

    // Create a test-specific Config constructor that uses env::var directly
    impl Config {
        fn from_env_test() -> Result<Self, Box<dyn std::error::Error>> {
            Ok(Self {
                chain_id: env::var("CHAIN_ID")?.parse()?,
                rpc_url: env::var("STARKNET_RPC_URL")?,
                verifier_address: env::var("FOSSIL_VERIFIER")?,
                store_address: env::var("FOSSIL_STORE")?,
                private_key: env::var("STARKNET_PRIVATE_KEY")?,
                account_address: env::var("STARKNET_ACCOUNT_ADDRESS")?,
            })
        }
    }

    fn set_valid_env_vars() {
        env::set_var("CHAIN_ID", "1");
        env::set_var("STARKNET_RPC_URL", "http://test.url");
        env::set_var("FOSSIL_VERIFIER", "verifier_addr");
        env::set_var("FOSSIL_STORE", "store_addr");
        env::set_var("STARKNET_PRIVATE_KEY", "private_key");
        env::set_var("STARKNET_ACCOUNT_ADDRESS", "account_addr");
    }

    #[test]
    fn test_args_default_values() {
        let args = Args::parse_from(["update_mmr", "--start", "100", "--end", "200"]);

        assert_eq!(args.start, 100);
        assert_eq!(args.end, 200);
        assert_eq!(args.skip_proof, false);
        assert_eq!(args.batch_size, 1024); // default value
    }

    #[test]
    fn test_args_custom_values() {
        let args = Args::parse_from([
            "update_mmr",
            "--start",
            "100",
            "--end",
            "200",
            "--skip-proof",
            "--batch-size",
            "500",
        ]);

        assert_eq!(args.start, 100);
        assert_eq!(args.end, 200);
        assert_eq!(args.skip_proof, true);
        assert_eq!(args.batch_size, 500);
    }

    #[test]
    fn test_config_from_env() {
        setup_test_env();
        set_valid_env_vars();

        let config = Config::from_env_test().unwrap();

        assert_eq!(config.chain_id, 1);
        assert_eq!(config.rpc_url, "http://test.url");
        assert_eq!(config.verifier_address, "verifier_addr");
        assert_eq!(config.store_address, "store_addr");
        assert_eq!(config.private_key, "private_key");
        assert_eq!(config.account_address, "account_addr");
    }

    #[test]
    fn test_config_missing_env() {
        setup_test_env();
        // Don't set any variables - we want them all missing
        let result = Config::from_env_test();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_invalid_chain_id() {
        setup_test_env();
        set_valid_env_vars(); // Set all variables first
        env::set_var("CHAIN_ID", "not_a_number"); // Then override CHAIN_ID with invalid value

        let result = Config::from_env_test();
        assert!(result.is_err());
    }
}
