#![deny(unused_crate_dependencies)]

use hasher::sha2::Sha2Hasher;
use mmr::MMR;
use sqlx::{Row, SqlitePool};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::{collections::HashMap, path::PathBuf};
use std::{env, fs};
use store::{sqlite::SQLiteStore, StoreError};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum MMRUtilsError {
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Invalid path: contains invalid UTF-8 characters")]
    InvalidPath,
}

#[allow(dead_code)]
pub struct StoreFactory;

#[allow(dead_code)]
impl StoreFactory {
    pub async fn create_store(path: &str, id: Option<&str>) -> Result<SQLiteStore, StoreError> {
        SQLiteStore::new(path, Some(true), id)
            .await
            .map_err(StoreError::SQLite)
    }
}

#[allow(dead_code)]
pub struct StoreManager {
    stores: Mutex<HashMap<String, Arc<SQLiteStore>>>,
}

impl StoreManager {
    pub async fn new(path: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(path).await?;

        // Create the mmr_metadata table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS mmr_metadata (
                mmr_id TEXT PRIMARY KEY
            )
            "#,
        )
        .execute(&pool)
        .await?;

        let manager = StoreManager {
            stores: Mutex::new(HashMap::new()),
        };

        // Initialize the value-to-index table
        manager.initialize_value_index_table(&pool).await?;

        Ok(manager)
    }

    pub async fn initialize_value_index_table(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS value_index_map (
                value TEXT PRIMARY KEY,
                element_index INTEGER NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn insert_value_index_mapping(
        &self,
        pool: &SqlitePool,
        value: &str,
        element_index: usize,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO value_index_map (value, element_index)
            VALUES (?, ?)
            "#,
        )
        .bind(value)
        .bind(element_index as i64) // SQLite uses i64 for integers
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Retrieves the element index based on the given hash value
    #[allow(dead_code)]
    pub async fn get_element_index_for_value(
        &self,
        pool: &SqlitePool,
        value: &str,
    ) -> Result<Option<usize>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT element_index FROM value_index_map WHERE value = ?
            "#,
        )
        .bind(value)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            let element_index: i64 = row.get("element_index");
            Ok(Some(element_index as usize))
        } else {
            Ok(None)
        }
    }

    /// Retrieves the stored value for the given element index, abstracting away the MMR ID
    #[allow(dead_code)]
    pub async fn get_value_for_element_index(
        &self,
        pool: &SqlitePool,
        element_index: usize,
    ) -> Result<Option<String>, sqlx::Error> {
        let element_index_str = element_index.to_string();

        // Query the store for the value associated with the given element_index
        let row = sqlx::query(
            r#"
            SELECT value FROM store WHERE key LIKE ?
            "#,
        )
        .bind(format!("%:hashes:{}", element_index_str)) // Match the key pattern using LIKE
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            let stored_value: String = row.get("value");
            Ok(Some(stored_value))
        } else {
            Ok(None)
        }
    }
}

/// Initializes the MMR by retrieving or creating the MMR ID and setting up the hasher and store
pub async fn initialize_mmr(
    store_path: &str,
) -> Result<(StoreManager, MMR, SqlitePool), MMRUtilsError> {
    let pool = SqlitePool::connect(store_path).await?;
    let store_manager = StoreManager::new(store_path).await?;
    let store = Arc::new(SQLiteStore::new(store_path, Some(true), None).await?);

    // Retrieve or generate a new MMR ID
    let mmr_id = if let Some(id) = get_mmr_id(&pool).await? {
        id
    } else {
        let new_id = Uuid::new_v4().to_string();
        save_mmr_id(&pool, &new_id).await?;
        new_id
    };

    let hasher = Arc::new(Sha2Hasher::new());
    let mmr = MMR::new(store, hasher, Some(mmr_id.clone()));

    Ok((store_manager, mmr, pool))
}

/// Retrieves the MMR ID from the `mmr_metadata` table
async fn get_mmr_id(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT mmr_id FROM mmr_metadata LIMIT 1")
        .fetch_optional(pool)
        .await?;

    if let Some(row) = row {
        let mmr_id: String = row.get("mmr_id");
        Ok(Some(mmr_id))
    } else {
        Ok(None)
    }
}

/// Saves the MMR ID to the `mmr_metadata` table
async fn save_mmr_id(pool: &SqlitePool, mmr_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR REPLACE INTO mmr_metadata (mmr_id) VALUES (?)")
        .bind(mmr_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Ensures that a directory exists, creates it if necessary
pub fn ensure_directory_exists(dir_name: &str) -> Result<PathBuf, MMRUtilsError> {
    let current_dir = env::current_dir()?.join(dir_name);
    if !current_dir.exists() {
        fs::create_dir_all(&current_dir)?; // Ensure directory is created
    }
    Ok(current_dir)
}

/// Creates a database file if it doesn't exist and returns the path to the file
pub fn create_database_file(
    current_dir: &Path,
    db_file_counter: usize,
) -> Result<String, MMRUtilsError> {
    let store_path = current_dir.join(format!("{}.db", db_file_counter));
    let store_path_str = store_path.to_str().ok_or(MMRUtilsError::InvalidPath)?;

    if !Path::new(store_path_str).exists() {
        File::create(store_path_str)?;
    }

    Ok(store_path_str.to_string())
}
