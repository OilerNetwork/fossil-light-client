use crate::api::operations::extract_fees;
use clap::Parser;
use common::get_env_var;
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Start block
    #[arg(long)]
    pub start_block: u64,

    /// End block
    #[arg(long)]
    pub end_block: u64,
}

#[derive(Debug)]
pub struct Config {
    pub chain_id: u64,
    pub rpc_url: String,
    pub store_address: String,
    pub private_key: String,
    pub account_address: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            chain_id: get_env_var("CHAIN_ID")?.parse()?,
            rpc_url: get_env_var("STARKNET_RPC_URL")?,
            store_address: get_env_var("FOSSIL_STORE")?,
            private_key: get_env_var("STARKNET_PRIVATE_KEY")?,
            account_address: get_env_var("STARKNET_ACCOUNT_ADDRESS")?,
        })
    }
}

pub async fn run(config: Config, args: Args) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting fee extraction...");

    extract_fees(
        &config.rpc_url,
        &config.store_address,
        config.chain_id,
        1024, // batch_size
        args.start_block,
        args.end_block,
        None, // skip_proof_verification
    )
    .await?;

    info!("Fee extraction completed");
    info!("Host finished");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_test_env() {
        env::remove_var("CHAIN_ID");
        env::remove_var("STARKNET_RPC_URL");
        env::remove_var("FOSSIL_STORE");
        env::remove_var("STARKNET_PRIVATE_KEY");
        env::remove_var("STARKNET_ACCOUNT_ADDRESS");
    }

    fn set_valid_env_vars() {
        env::set_var("CHAIN_ID", "1");
        env::set_var("STARKNET_RPC_URL", "http://test.url");
        env::set_var("FOSSIL_STORE", "store_addr");
        env::set_var("STARKNET_PRIVATE_KEY", "private_key");
        env::set_var("STARKNET_ACCOUNT_ADDRESS", "account_addr");
    }

    impl Config {
        fn from_env_test() -> Result<Self, Box<dyn std::error::Error>> {
            Ok(Self {
                chain_id: env::var("CHAIN_ID")?.parse()?,
                rpc_url: env::var("STARKNET_RPC_URL")?,
                store_address: env::var("FOSSIL_STORE")?,
                private_key: env::var("STARKNET_PRIVATE_KEY")?,
                account_address: env::var("STARKNET_ACCOUNT_ADDRESS")?,
            })
        }
    }

    #[test]
    fn test_args_default_values() {
        let args = Args::parse_from(["extract_fees", "--start-block", "100", "--end-block", "200"]);

        assert_eq!(args.start_block, 100);
        assert_eq!(args.end_block, 200);
    }

    #[test]
    fn test_config_from_env() {
        setup_test_env();
        set_valid_env_vars();

        let config = Config::from_env_test().unwrap();

        assert_eq!(config.chain_id, 1);
        assert_eq!(config.rpc_url, "http://test.url");
        assert_eq!(config.store_address, "store_addr");
        assert_eq!(config.private_key, "private_key");
        assert_eq!(config.account_address, "account_addr");
    }

    #[test]
    fn test_config_missing_env() {
        setup_test_env();
        let result = Config::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_invalid_chain_id() {
        setup_test_env();
        set_valid_env_vars();
        env::set_var("CHAIN_ID", "invalid_chain_id");

        let result = Config::from_env();
        assert!(result.is_err());
    }
}
