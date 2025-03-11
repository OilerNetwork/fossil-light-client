use common::get_env_var;
use eth_rlp_types::BlockHeader;
use eyre::{eyre, Result};
use mmr_utils::{create_database_file, ensure_directory_exists};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

#[derive(Debug)]
pub struct DbConnection {
    pub pool: Pool<Postgres>,
}

// Use Arc to allow thread-safe cloning
impl DbConnection {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: Duration = Duration::from_secs(5);

    /// Creates a new database connection with retries
    pub async fn new() -> Result<Arc<Self>> {
        let mut attempt = 0;

        while attempt < Self::MAX_RETRIES {
            match Self::try_connect().await {
                Ok(db) => {
                    if attempt > 0 {
                        info!(
                            "Successfully connected to database after {} attempts",
                            attempt + 1
                        );
                    }
                    return Ok(db);
                }
                Err(e) => {
                    attempt += 1;
                    if attempt < Self::MAX_RETRIES {
                        error!(
                            error = %e,
                            attempt,
                            "Database connection failed, retrying in {} seconds...",
                            Self::RETRY_DELAY.as_secs()
                        );
                        sleep(Self::RETRY_DELAY).await;
                    } else {
                        return Err(eyre!(
                            "Failed to connect after {} attempts: {}",
                            Self::MAX_RETRIES,
                            e
                        ));
                    }
                }
            }
        }

        unreachable!()
    }

    /// Internal method to attempt a database connection
    async fn try_connect() -> Result<Arc<Self>> {
        let database_url = get_env_var("DATABASE_URL")?;

        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .max_lifetime(std::time::Duration::from_secs(30 * 60))
            .idle_timeout(std::time::Duration::from_secs(10 * 60))
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect(&database_url)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?;

        Ok(Arc::new(Self { pool }))
    }

    pub async fn get_block_headers_by_block_range(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<BlockHeader>> {
        if start_block > end_block {
            return Err(eyre!(
                "Invalid block range: start block {} is greater than end block {}",
                start_block,
                end_block
            ));
        }
        let temp_headers = sqlx::query_as!(
            TempBlockHeader,
            r#"
            SELECT block_hash, number, gas_limit, gas_used, nonce, 
                   transaction_root, receipts_root, state_root, 
                   base_fee_per_gas, parent_hash, miner, logs_bloom, 
                   difficulty, totaldifficulty, sha3_uncles, timestamp, 
                   extra_data, mix_hash, withdrawals_root, 
                   blob_gas_used, excess_blob_gas, parent_beacon_block_root,
                   requests_hash
            FROM public.blockheaders
            WHERE number BETWEEN $1 AND $2
            ORDER BY number ASC
            "#,
            start_block as i64,
            end_block as i64
        )
        .fetch_all(&self.pool)
        .await?;

        let headers: Vec<BlockHeader> =
            temp_headers.into_iter().map(temp_to_block_header).collect();

        Ok(headers)
    }

    /// Fetches a single block header by block number
    pub async fn get_block_header_by_number(&self, block_number: u64) -> Result<BlockHeader> {
        let temp_header = sqlx::query_as!(
            TempBlockHeader,
            r#"
            SELECT block_hash, number, gas_limit, gas_used, nonce, 
                   transaction_root, receipts_root, state_root, 
                   base_fee_per_gas, parent_hash, miner, logs_bloom, 
                   difficulty, totaldifficulty, sha3_uncles, timestamp, 
                   extra_data, mix_hash, withdrawals_root, 
                   blob_gas_used, excess_blob_gas, parent_beacon_block_root,
                   requests_hash
            FROM public.blockheaders
            WHERE number = $1
            "#,
            block_number as i64
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| eyre!("Block header not found for block number: {}", block_number))?;

        Ok(temp_to_block_header(temp_header))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct TempBlockHeader {
    pub block_hash: Option<String>,       // character(66), nullable
    pub number: i64,                      // bigint NOT NULL
    pub gas_limit: i64,                   // bigint NOT NULL
    pub gas_used: i64,                    // bigint NOT NULL
    pub base_fee_per_gas: Option<String>, // character varying(78), nullable
    pub nonce: String,                    // character varying(78) NOT NULL
    pub transaction_root: Option<String>, // character(66), nullable
    pub receipts_root: Option<String>,    // character(66), nullable
    pub state_root: Option<String>,       // character(66), nullable
    pub parent_hash: Option<String>,      // character varying(66), nullable
    pub miner: Option<String>,            // character varying(42), nullable
    pub logs_bloom: Option<String>,       // character varying(1024), nullable
    pub difficulty: Option<String>,       // character varying(78), nullable
    pub totaldifficulty: Option<String>,  // character varying(78), nullable
    pub sha3_uncles: Option<String>,      // character varying(66), nullable
    pub timestamp: Option<String>,        // character varying(100), nullable
    pub extra_data: Option<String>,       // character varying(1024), nullable
    pub mix_hash: Option<String>,         // character varying(66), nullable
    pub withdrawals_root: Option<String>, // character varying(66), nullable
    pub blob_gas_used: Option<String>,    // character varying(78), nullable
    pub excess_blob_gas: Option<String>,  // character varying(78), nullable
    pub parent_beacon_block_root: Option<String>, // character varying(66), nullable
    pub requests_hash: Option<String>,    // character varying(66), nullable
}

fn temp_to_block_header(temp: TempBlockHeader) -> BlockHeader {
    BlockHeader {
        block_hash: temp.block_hash.unwrap(), // String (not Option<String>)
        number: temp.number,                  // i64 (not Option<i64>)
        gas_limit: temp.gas_limit,            // i64 (not Option<i64>)
        gas_used: temp.gas_used,              // i64 (not Option<i64>)
        nonce: temp.nonce,                    // String (not Option<String>)
        transaction_root: temp.transaction_root, // Option<String>
        receipts_root: temp.receipts_root,    // Option<String>
        state_root: temp.state_root,          // Option<String>
        base_fee_per_gas: temp.base_fee_per_gas, // Option<String>

        // Only assign fields that exist in EthBlockHeader
        parent_hash: temp.parent_hash, // Option<String> (if exists)
        ommers_hash: temp.sha3_uncles.clone(), // Option<String> (if exists)
        miner: temp.miner,             // Option<String> (if exists)

        // For the following, use Option<String> correctly
        logs_bloom: temp.logs_bloom,
        difficulty: temp.difficulty,
        totaldifficulty: temp.totaldifficulty,
        sha3_uncles: temp.sha3_uncles, // Option<String> (if exists)

        // Convert timestamp from decimal to hex string format
        timestamp: temp.timestamp.map(|ts| {
            // Parse the decimal string to u64, then format as hex
            ts.parse::<u64>()
                .map(|t| format!("0x{:x}", t))
                .unwrap_or(ts)
        }),
        extra_data: temp.extra_data,
        mix_hash: temp.mix_hash,
        withdrawals_root: temp.withdrawals_root,
        blob_gas_used: temp.blob_gas_used,
        excess_blob_gas: temp.excess_blob_gas,
        parent_beacon_block_root: temp.parent_beacon_block_root,
        request_hash: temp.requests_hash,
    }
}

pub fn get_store_path(db_file: Option<String>) -> Result<String> {
    // Load the database file path from the environment or use the provided argument
    let store_path = if let Some(db_file) = db_file {
        db_file
    } else {
        // Otherwise, create a new database file
        let current_dir = ensure_directory_exists("db-instances")?;
        create_database_file(&current_dir, 0)?
    };

    Ok(store_path)
}
