use common::get_env_var;
use eyre::{eyre, Result, WrapErr};
use mmr_utils::{create_database_file, ensure_directory_exists};
use starknet::{
    core::types::{BlockId, EventFilter, Felt},
    macros::selector,
    providers::Provider as EventProvider,
};
use starknet_handler::provider::StarknetProvider;
use tokio::time::Duration;
use tracing::{debug, error, info, instrument};

#[cfg(test)]
use mockall::automock;

#[cfg(test)]
#[automock]
trait StarknetProviderFactory {
    fn create_provider(&self, rpc_url: &str) -> Result<StarknetProvider>;
}

#[cfg(test)]
#[automock]
trait DatabaseUtils {
    fn ensure_directory_exists(&self, path: &str) -> Result<std::path::PathBuf>;
    fn create_database_file(&self, dir: &std::path::Path, index: u64) -> Result<String>;
}

#[cfg(test)]
#[automock]
trait EnvVarReader {
    fn get_env_var(&self, key: &str) -> Result<String>;
}

#[cfg(test)]
pub(crate) struct TestDependencies {
    db_utils: Box<dyn DatabaseUtils>,
    env_reader: Box<dyn EnvVarReader>,
    provider_factory: Box<dyn StarknetProviderFactory>,
}

#[derive(Debug)]
pub struct LightClient {
    starknet_provider: StarknetProvider,
    l2_store_addr: String,
    verifier_addr: String,
    chain_id: u64,
    latest_processed_events_block: u64,
    latest_processed_mmr_block: u64,
    starknet_private_key: String,
    starknet_account_address: String,
    polling_interval: Duration,
    batch_size: u64,
    blocks_per_run: u64,
}

impl LightClient {
    /// Creates a new instance of the light client.
    pub async fn new(
        polling_interval: u64,
        batch_size: u64,
        start_block: u64,
        blocks_per_run: u64,
    ) -> Result<Self> {
        if polling_interval == 0 {
            error!("Polling interval must be greater than zero");
            return Err(eyre!("Polling interval must be greater than zero"));
        }

        // Load environment variables
        let starknet_rpc_url = get_env_var("STARKNET_RPC_URL")
            .wrap_err("Failed to get STARKNET_RPC_URL environment variable")?;
        let l2_store_addr = get_env_var("FOSSIL_STORE")
            .wrap_err("Failed to get FOSSIL_STORE environment variable")?;
        let verifier_addr = get_env_var("FOSSIL_VERIFIER")
            .wrap_err("Failed to get FOSSIL_VERIFIER environment variable")?;
        let starknet_private_key = get_env_var("STARKNET_PRIVATE_KEY")
            .wrap_err("Failed to get STARKNET_PRIVATE_KEY environment variable")?;
        let starknet_account_address = get_env_var("STARKNET_ACCOUNT_ADDRESS")
            .wrap_err("Failed to get STARKNET_ACCOUNT_ADDRESS environment variable")?;
        let chain_id = get_env_var("CHAIN_ID")
            .wrap_err("Failed to get CHAIN_ID environment variable")?
            .parse::<u64>()
            .wrap_err("Failed to parse CHAIN_ID as u64")?;

        // Initialize providers
        let starknet_provider = StarknetProvider::new(&starknet_rpc_url)
            .wrap_err("Failed to initialize Starknet provider")?;

        // Set up the database file path
        let current_dir = ensure_directory_exists("../../db-instances")
            .wrap_err("Failed to ensure database directory exists")?;
        let db_file =
            create_database_file(&current_dir, 0).wrap_err("Failed to create database file")?;

        if !std::path::Path::new(&db_file).exists() {
            error!("Database file does not exist at path: {}", db_file);
            return Err(eyre!("Database file does not exist at path: {}", db_file));
        }

        Ok(Self {
            starknet_provider,
            l2_store_addr,
            verifier_addr,
            chain_id,
            latest_processed_events_block: start_block.saturating_sub(1),
            latest_processed_mmr_block: start_block.saturating_sub(1),
            starknet_private_key,
            starknet_account_address,
            polling_interval: Duration::from_secs(polling_interval),
            batch_size,
            blocks_per_run,
        })
    }

    /// Processes new events from the Starknet store contract.
    pub async fn process_new_events(&mut self) -> Result<()> {
        // Get the latest block number
        let latest_block = self
            .starknet_provider
            .provider()
            .block_number()
            .await
            .wrap_err("Failed to get latest block number from Starknet")?;

        // Don't process if we're already caught up with events
        if self.latest_processed_events_block >= latest_block {
            return Ok(());
        }

        // Calculate the to_block based on blocks_per_run
        let to_block = if self.blocks_per_run > 0 {
            std::cmp::min(
                self.latest_processed_events_block + self.blocks_per_run,
                latest_block,
            )
        } else {
            latest_block
        };

        let from_block = self.latest_processed_events_block + 1;

        // Add validation to prevent block number regression
        if from_block > to_block {
            error!(
                from_block,
                to_block, "Invalid block range: from_block is greater than to_block"
            );
            return Ok(());
        }

        let event_filter = EventFilter {
            from_block: Some(BlockId::Number(from_block)),
            to_block: Some(BlockId::Number(to_block)),
            address: Some(
                Felt::from_hex(&self.l2_store_addr)
                    .wrap_err("Failed to convert store address to Felt")?,
            ),
            keys: Some(vec![vec![selector!("LatestBlockhashFromL1Stored")]]),
        };

        let events = self
            .starknet_provider
            .provider()
            .get_events(event_filter, None, 1)
            .await
            .wrap_err("Failed to get events from Starknet provider")?;

        // Update the latest processed events block
        self.latest_processed_events_block = to_block;

        if !events.events.is_empty() {
            info!(event_count = events.events.len(), "Processing new events");
            // Process the events and update MMR
            self.handle_events().await?;
        }

        Ok(())
    }

    /// Handles the events by updating the MMR and verifying proofs.
    #[instrument(skip(self))]
    pub async fn handle_events(&mut self) -> Result<()> {
        // Fetch the latest stored blockhash from L1
        let latest_relayed_block = self
            .starknet_provider
            .get_latest_relayed_block(&self.l2_store_addr)
            .await
            .wrap_err("Failed to get latest relayed block from Starknet")?;

        // Fetch latest MMR state from L2
        let latest_mmr_block = self
            .starknet_provider
            .get_latest_mmr_block(&self.l2_store_addr)
            .await
            .wrap_err("Failed to get latest MMR block from Starknet")?;

        // Update MMR and verify proofs
        self.update_mmr(latest_mmr_block, latest_relayed_block)
            .await?;

        Ok(())
    }

    /// Updates the MMR and verifies the proof on-chain.
    #[instrument(skip(self))]
    pub async fn update_mmr(
        &mut self,
        latest_mmr_block: u64,
        latest_relayed_block: u64,
    ) -> Result<()> {
        info!(
            latest_mmr_block,
            latest_relayed_block, "Starting MMR update"
        );

        let start_block = latest_mmr_block + 1;
        let end_block = latest_relayed_block;

        if start_block > end_block {
            debug!("No new blocks to process for MMR update");
            return Ok(());
        }

        // Call the publisher function directly with all required parameters
        let _result = publisher::api::operations::update_mmr(
            &get_env_var("STARKNET_RPC_URL")
                .wrap_err("Failed to get STARKNET_RPC_URL environment variable")?,
            self.chain_id,
            &self.verifier_addr,
            &self.l2_store_addr,
            &self.starknet_private_key,
            &self.starknet_account_address,
            self.batch_size,
            start_block,
            end_block,
        )
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to update MMR");
            eyre!("Failed to update MMR: {}", e)
        })?;

        // Update the latest processed MMR block
        self.latest_processed_mmr_block = latest_relayed_block;

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            "Listening for events from block {} with polling interval {} seconds",
            self.latest_processed_events_block + 1,
            self.polling_interval.as_secs()
        );

        loop {
            self.process_new_events().await?;
            tokio::time::sleep(self.polling_interval).await;
        }
    }
}

#[cfg(test)]
impl LightClient {
    async fn new_with_deps(
        polling_interval: u64,
        batch_size: u64,
        start_block: u64,
        blocks_per_run: u64,
        deps: TestDependencies,
    ) -> Result<Self> {
        if polling_interval == 0 {
            return Err(eyre!("Polling interval must be greater than zero"));
        }

        let starknet_rpc_url = deps
            .env_reader
            .get_env_var("STARKNET_RPC_URL")
            .wrap_err("Failed to get STARKNET_RPC_URL environment variable")?;
        let l2_store_addr = deps
            .env_reader
            .get_env_var("FOSSIL_STORE")
            .wrap_err("Failed to get FOSSIL_STORE environment variable")?;
        let verifier_addr = deps
            .env_reader
            .get_env_var("FOSSIL_VERIFIER")
            .wrap_err("Failed to get FOSSIL_VERIFIER environment variable")?;
        let starknet_private_key = deps
            .env_reader
            .get_env_var("STARKNET_PRIVATE_KEY")
            .wrap_err("Failed to get STARKNET_PRIVATE_KEY environment variable")?;
        let starknet_account_address = deps
            .env_reader
            .get_env_var("STARKNET_ACCOUNT_ADDRESS")
            .wrap_err("Failed to get STARKNET_ACCOUNT_ADDRESS environment variable")?;
        let chain_id = deps
            .env_reader
            .get_env_var("CHAIN_ID")
            .wrap_err("Failed to get CHAIN_ID environment variable")?
            .parse::<u64>()
            .wrap_err("Failed to parse CHAIN_ID as u64")?;

        let starknet_provider = deps
            .provider_factory
            .create_provider(&starknet_rpc_url)
            .wrap_err("Failed to create Starknet provider")?;
        let current_dir = deps
            .db_utils
            .ensure_directory_exists("../../db-instances")
            .wrap_err("Failed to ensure database directory exists")?;
        let db_file = deps
            .db_utils
            .create_database_file(&current_dir, 0)
            .wrap_err("Failed to create database file")?;

        if !std::path::Path::new(&db_file).exists() {
            return Err(eyre!("Database file does not exist at path: {}", db_file));
        }

        Ok(Self {
            starknet_provider,
            l2_store_addr,
            verifier_addr,
            chain_id,
            latest_processed_events_block: start_block.saturating_sub(1),
            latest_processed_mmr_block: start_block.saturating_sub(1),
            starknet_private_key,
            starknet_account_address,
            polling_interval: Duration::from_secs(polling_interval),
            batch_size,
            blocks_per_run,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_lightclient_new_valid_inputs() {
        let mut mock_db = MockDatabaseUtils::new();
        let mut mock_env = MockEnvVarReader::new();
        let mut mock_provider_factory = MockStarknetProviderFactory::new();

        let tmp_dir = tempdir().unwrap();
        let db_path = tmp_dir.path().join("dbfile_0.sqlite");
        std::fs::File::create(&db_path).unwrap();

        mock_db
            .expect_ensure_directory_exists()
            .returning(move |_| Ok(tmp_dir.path().to_path_buf()));

        mock_db
            .expect_create_database_file()
            .returning(move |_, _| Ok(db_path.to_string_lossy().to_string()));

        mock_env.expect_get_env_var().returning(|key| {
            Ok(match key {
                "STARKNET_RPC_URL" => "http://localhost:5050".to_string(),
                "FOSSIL_STORE" => "0x1".to_string(),
                "FOSSIL_VERIFIER" => "0x2".to_string(),
                "STARKNET_PRIVATE_KEY" => "testkey".to_string(),
                "STARKNET_ACCOUNT_ADDRESS" => "0xabc".to_string(),
                "CHAIN_ID" => "5".to_string(),
                _ => "dummy".to_string(),
            })
        });

        mock_provider_factory
            .expect_create_provider()
            .returning(|rpc_url| Ok(StarknetProvider::new(rpc_url).unwrap()));

        let deps = TestDependencies {
            db_utils: Box::new(mock_db),
            env_reader: Box::new(mock_env),
            provider_factory: Box::new(mock_provider_factory),
        };

        let client = LightClient::new_with_deps(10, 100, 0, 10, deps).await;
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.chain_id, 5);
        assert_eq!(client.polling_interval.as_secs(), 10);
        assert_eq!(client.batch_size, 100);
    }

    #[tokio::test]
    async fn test_lightclient_new_zero_polling_interval() {
        let mock_db = MockDatabaseUtils::new();
        let mock_env = MockEnvVarReader::new();
        let mock_provider_factory = MockStarknetProviderFactory::new();

        let deps = TestDependencies {
            db_utils: Box::new(mock_db),
            env_reader: Box::new(mock_env),
            provider_factory: Box::new(mock_provider_factory),
        };

        let result = LightClient::new_with_deps(0, 100, 0, 10, deps).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Polling interval must be greater than zero"));
    }

    #[tokio::test]
    async fn test_lightclient_new_missing_db_file() {
        let mut mock_db = MockDatabaseUtils::new();
        let mut mock_env = MockEnvVarReader::new();
        let mut mock_provider_factory = MockStarknetProviderFactory::new();

        let tmp_dir = tempdir().unwrap();
        let db_path = tmp_dir.path().join("nonexistent_file.sqlite");

        mock_db
            .expect_ensure_directory_exists()
            .returning(move |_| Ok(tmp_dir.path().to_path_buf()));

        mock_db
            .expect_create_database_file()
            .returning(move |_, _| Ok(db_path.to_string_lossy().to_string()));

        // Mock all required environment variables
        mock_env.expect_get_env_var().returning(|key| {
            Ok(match key {
                "STARKNET_RPC_URL" => "http://localhost:5050".to_string(),
                "FOSSIL_STORE" => "0x1".to_string(),
                "FOSSIL_VERIFIER" => "0x2".to_string(),
                "STARKNET_PRIVATE_KEY" => "testkey".to_string(),
                "STARKNET_ACCOUNT_ADDRESS" => "0xabc".to_string(),
                "CHAIN_ID" => "5".to_string(),
                _ => "dummy".to_string(),
            })
        });

        mock_provider_factory
            .expect_create_provider()
            .returning(|_| Ok(StarknetProvider::new("http://localhost:5050").unwrap()));

        let deps = TestDependencies {
            db_utils: Box::new(mock_db),
            env_reader: Box::new(mock_env),
            provider_factory: Box::new(mock_provider_factory),
        };

        let result = LightClient::new_with_deps(10, 100, 0, 10, deps).await;

        // The file doesn't exist, so we should get an error
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Database file does not exist"));
    }

    #[tokio::test]
    async fn test_lightclient_new_bad_chain_id() {
        let mut mock_db = MockDatabaseUtils::new();
        let mut mock_env = MockEnvVarReader::new();
        let mut mock_provider_factory = MockStarknetProviderFactory::new();

        let tmp_dir = tempdir().unwrap();
        mock_db
            .expect_ensure_directory_exists()
            .returning(move |_| Ok(tmp_dir.path().to_path_buf()));

        mock_env.expect_get_env_var().returning(|key| {
            Ok(match key {
                "CHAIN_ID" => "not_a_number".to_string(),
                _ => "dummy".to_string(),
            })
        });

        mock_provider_factory
            .expect_create_provider()
            .returning(|rpc_url| Ok(StarknetProvider::new(rpc_url).unwrap()));

        let deps = TestDependencies {
            db_utils: Box::new(mock_db),
            env_reader: Box::new(mock_env),
            provider_factory: Box::new(mock_provider_factory),
        };

        let result = LightClient::new_with_deps(10, 100, 0, 10, deps).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse CHAIN_ID"));
    }
}
