use std::sync::Arc;

use common::get_env_var;
use eth_rlp_types::BlockHeader;
use mmr_utils::{create_database_file, ensure_directory_exists};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

use crate::error::{PublisherError, PublisherResult};

#[derive(Debug)]
/// Database connection wrapper for Postgres operations
pub struct DbConnection {
    /// Connection pool for database operations
    pub pool: Pool<Postgres>,
}

// Use Arc to allow thread-safe cloning
impl DbConnection {
    const MAX_RETRIES: u32 = 10;
    const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(2);
    const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

    /// Creates a new database connection with exponential backoff retries
    #[allow(clippy::cognitive_complexity)]
    pub async fn new() -> PublisherResult<Arc<Self>> {
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
                        let delay = std::cmp::min(
                            Self::INITIAL_RETRY_DELAY * 2u32.pow(attempt.saturating_sub(1)),
                            Self::MAX_RETRY_DELAY,
                        );
                        warn!(
                            error = %e,
                            attempt,
                            max_retries = Self::MAX_RETRIES,
                            "Database connection failed, retrying in {} seconds...",
                            delay.as_secs()
                        );
                        sleep(delay).await;
                    } else {
                        error!(
                            error = %e,
                            attempts = Self::MAX_RETRIES,
                            "Database connection failed after all retry attempts"
                        );
                        return Err(PublisherError::database(format!(
                            "Failed to connect after {} attempts: {}",
                            Self::MAX_RETRIES,
                            e
                        )));
                    }
                }
            }
        }

        unreachable!()
    }

    /// Internal method to attempt a database connection
    async fn try_connect() -> PublisherResult<Arc<Self>> {
        let database_url = get_env_var("DATABASE_URL")?;

        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .max_lifetime(std::time::Duration::from_secs(30 * 60))
            .idle_timeout(std::time::Duration::from_secs(10 * 60))
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect(&database_url)
            .await
            .map_err(|e| PublisherError::database(format!("Failed to connect to database: {e}")))?;

        Ok(Arc::new(Self { pool }))
    }

    /// Execute a database operation with retry logic
    pub async fn with_retry<F, Fut, T>(
        &self,
        operation: F,
        operation_name: &str,
    ) -> PublisherResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        let mut attempt = 0;
        let max_retries = 3;
        let initial_delay = Duration::from_millis(500);

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempt >= max_retries {
                        error!(
                            operation = operation_name,
                            attempts = attempt + 1,
                            error = %e,
                            "Database operation failed after all retry attempts"
                        );
                        return Err(PublisherError::database(format!(
                            "Operation '{}' failed after {} attempts: {}",
                            operation_name,
                            attempt + 1,
                            e
                        )));
                    }

                    let delay = initial_delay * 2u32.pow(attempt);
                    warn!(
                        operation = operation_name,
                        attempt = attempt + 1,
                        max_retries = max_retries,
                        error = %e,
                        "Database operation failed, retrying in {}ms...",
                        delay.as_millis()
                    );

                    sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Get block hash by block number
    pub async fn get_block_hash_by_number(
        &self,
        block_number: u64,
    ) -> PublisherResult<Option<String>> {
        self.with_retry(
            || async {
                sqlx::query!(
                    r#"
                    SELECT block_hash
                    FROM public.blockheaders
                    WHERE number = $1
                    "#,
                    block_number as i64
                )
                .fetch_optional(&self.pool)
                .await
            },
            "get_block_hash_by_number",
        )
        .await
        .map(|result| result.and_then(|row| row.block_hash))
    }

    /// Get block headers within a specific block range
    pub async fn get_block_headers_by_block_range(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> PublisherResult<Vec<BlockHeader>> {
        if start_block > end_block {
            return Err(PublisherError::database(format!(
                "Invalid block range: start block {start_block} is greater than end block {end_block}"
            )));
        }
        let temp_headers = self
            .with_retry(
                || async {
                    sqlx::query_as!(
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
                    .await
                },
                "get_block_headers_by_block_range",
            )
            .await?;

        let headers: Result<Vec<BlockHeader>, PublisherError> =
            temp_headers.into_iter().map(temp_to_block_header).collect();
        let headers = headers?;

        Ok(headers)
    }

    /// Fetches a single block header by block number
    pub async fn get_block_header_by_number(
        &self,
        block_number: u64,
    ) -> PublisherResult<BlockHeader> {
        let temp_header = self
            .with_retry(
                || async {
                    sqlx::query_as!(
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
                    .await
                },
                "get_block_header_by_number",
            )
            .await?
            .ok_or_else(|| {
                PublisherError::database(format!(
                    "Block header not found for block number: {block_number}"
                ))
            })?;

        temp_to_block_header(temp_header)
    }

    /// Fetches a single block header by block hash
    pub async fn get_block_header_by_hash(&self, block_hash: &str) -> PublisherResult<BlockHeader> {
        let temp_header = self
            .with_retry(
                || async {
                    sqlx::query_as!(
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
                    WHERE block_hash = $1
                    "#,
                        block_hash
                    )
                    .fetch_optional(&self.pool)
                    .await
                },
                "get_block_header_by_hash",
            )
            .await?
            .ok_or_else(|| {
                PublisherError::database(format!(
                    "Block header not found for block hash: {block_hash}"
                ))
            })?;

        temp_to_block_header(temp_header)
    }

    /// Fetches hourly block headers in a given range
    pub async fn get_hourly_block_headers_in_range(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> PublisherResult<Vec<BlockHeader>> {
        if start_block > end_block {
            return Err(PublisherError::database(format!(
                "Invalid block range: start block {start_block} is greater than end block {end_block}"
            )));
        }

        // Get the first block of each hour within the range
        let temp_headers = self.with_retry(
            || async {
                sqlx::query_as!(
                    TempBlockHeader,
                    r#"
                    WITH hourly_blocks AS (
                        SELECT
                            *,
                            ROW_NUMBER() OVER (PARTITION BY DATE_TRUNC('hour', TO_TIMESTAMP(timestamp::numeric)) ORDER BY number) as row_num
                        FROM public.blockheaders
                        WHERE number BETWEEN $1 AND $2
                    )
                    SELECT
                        block_hash, number, gas_limit, gas_used, nonce,
                        transaction_root, receipts_root, state_root,
                        base_fee_per_gas, parent_hash, miner, logs_bloom,
                        difficulty, totaldifficulty, sha3_uncles, timestamp,
                        extra_data, mix_hash, withdrawals_root,
                        blob_gas_used, excess_blob_gas, parent_beacon_block_root,
                        requests_hash
                    FROM hourly_blocks
                    WHERE row_num = 1
                    ORDER BY number ASC
                    "#,
                    start_block as i64,
                    end_block as i64
                )
                .fetch_all(&self.pool)
                .await
            },
            "get_hourly_block_headers_in_range"
        ).await?;

        let headers: Result<Vec<BlockHeader>, PublisherError> =
            temp_headers.into_iter().map(temp_to_block_header).collect();
        let headers = headers?;
        Ok(headers)
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

fn temp_to_block_header(temp: TempBlockHeader) -> Result<BlockHeader, PublisherError> {
    Ok(BlockHeader {
        block_hash: temp
            .block_hash
            .ok_or_else(|| PublisherError::database("Block hash is null"))?, /* String (not Option<String>) */
        number: temp.number,                     // i64 (not Option<i64>)
        gas_limit: temp.gas_limit,               // i64 (not Option<i64>)
        gas_used: temp.gas_used,                 // i64 (not Option<i64>)
        nonce: temp.nonce,                       // String (not Option<String>)
        transaction_root: temp.transaction_root, // Option<String>
        receipts_root: temp.receipts_root,       // Option<String>
        state_root: temp.state_root,             // Option<String>
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

        // Convert timestamp from decimal to hex string format for eth-rlp-verify compatibility
        timestamp: temp.timestamp.map(|ts| {
            // If it's already a hex string, keep it; otherwise convert from decimal to hex
            let converted = if ts.starts_with("0x") {
                tracing::debug!(
                    "Timestamp already in hex format for block {}: {}",
                    temp.number,
                    ts
                );
                ts
            } else {
                let converted = ts
                    .parse::<u64>()
                    .map(|t| format!("0x{:x}", t))
                    .unwrap_or_else(|_| ts.clone());
                tracing::debug!(
                    "Converted timestamp for block {} from decimal '{}' to hex '{}'",
                    temp.number,
                    ts,
                    converted
                );
                converted
            };
            converted
        }),
        extra_data: temp.extra_data,
        mix_hash: temp.mix_hash,
        withdrawals_root: temp.withdrawals_root,
        blob_gas_used: temp.blob_gas_used,
        excess_blob_gas: temp.excess_blob_gas,
        parent_beacon_block_root: temp.parent_beacon_block_root,
        request_hash: temp.requests_hash.or_else(|| {
            // EIP-7685: Default to empty requests hash when NULL
            // This is the keccak hash of an empty RLP list
            Some("0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string())
        }),
    })
}

/// Get the path for storing database files
pub fn get_store_path(db_file: Option<String>) -> PublisherResult<String> {
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
