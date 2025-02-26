#![deny(unused_crate_dependencies)]

use eyre::{eyre, Result};
use hasher::sha2::Sha2Hasher;
use mmr::MMR;
use sqlx::{Row, SqlitePool};
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use store::sqlite::SQLiteStore;

#[allow(dead_code)]
pub struct StoreFactory;

#[allow(dead_code)]
impl StoreFactory {
    pub async fn create_store(path: &str, id: Option<&str>) -> Result<SQLiteStore> {
        Ok(SQLiteStore::new(path, Some(true), id).await?)
    }
}

#[allow(dead_code)]
pub struct StoreManager {
    stores: Mutex<HashMap<String, Arc<SQLiteStore>>>,
}

impl StoreManager {
    pub async fn new(path: &str) -> Result<Self> {
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

    pub async fn initialize_value_index_table(&self, pool: &SqlitePool) -> Result<()> {
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
    ) -> Result<()> {
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

    pub async fn get_all_elements(&self, pool: &SqlitePool) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT value FROM value_index_map")
            .fetch_all(pool)
            .await?;
        Ok(rows.iter().map(|r| r.get("value")).collect())
    }

    /// Retrieves the element index based on the given hash value
    #[allow(dead_code)]
    pub async fn get_element_index_for_value(
        &self,
        pool: &SqlitePool,
        value: &str,
    ) -> Result<Option<usize>> {
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
    ) -> Result<Option<String>> {
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
pub async fn initialize_mmr(store_path: &str) -> Result<(StoreManager, MMR, SqlitePool)> {
    let pool = SqlitePool::connect(store_path).await?;
    let store_manager = StoreManager::new(store_path).await?;
    let store = Arc::new(SQLiteStore::new(store_path, Some(true), None).await?);

    // Retrieve or generate a new MMR ID
    let mmr_id = if let Some(id) = get_mmr_id(&pool).await? {
        id
    } else {
        let new_id = uuid::Uuid::new_v4().to_string();
        save_mmr_id(&pool, &new_id).await?;
        new_id
    };

    let hasher = Arc::new(Sha2Hasher::new());
    let mmr = MMR::new(store, hasher, Some(mmr_id.clone()));

    Ok((store_manager, mmr, pool))
}

/// Retrieves the MMR ID from the `mmr_metadata` table
async fn get_mmr_id(pool: &SqlitePool) -> Result<Option<String>> {
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
async fn save_mmr_id(pool: &SqlitePool, mmr_id: &str) -> Result<()> {
    sqlx::query("INSERT OR REPLACE INTO mmr_metadata (mmr_id) VALUES (?)")
        .bind(mmr_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Ensures that a directory exists, creates it if necessary
pub fn ensure_directory_exists(dir_name: &str) -> Result<PathBuf> {
    let current_dir = env::current_dir()?.join(dir_name);
    if !current_dir.exists() {
        fs::create_dir_all(&current_dir)?; // Ensure directory is created
    }
    Ok(current_dir)
}

/// Creates a database file if it doesn't exist and returns the path to the file
pub fn create_database_file(current_dir: &Path, db_file_counter: usize) -> Result<String> {
    let store_path = current_dir.join(format!("{}.db", db_file_counter));
    let store_path_str = store_path.to_str().ok_or(eyre!("Invalid path"))?;

    if !Path::new(store_path_str).exists() {
        File::create(store_path_str)?;
    }

    Ok(store_path_str.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> (StoreManager, SqlitePool) {
        // Use in-memory SQLite database for testing
        let db_url = "sqlite::memory:";
        let pool = SqlitePool::connect(db_url).await.unwrap();

        // Create mmr_metadata table first
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS mmr_metadata (
                mmr_id TEXT PRIMARY KEY
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Now create the manager which will create value_index_map table
        let manager = StoreManager::new(&format!("sqlite:{}", db_url))
            .await
            .unwrap();

        // Initialize the value-to-index table explicitly with the same pool
        manager.initialize_value_index_table(&pool).await.unwrap();

        (manager, pool)
    }

    #[tokio::test]
    async fn test_store_manager_initialization() {
        let (_, pool) = setup_test_db().await;

        // Verify the tables were created
        let result = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='value_index_map'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();

        assert!(result.is_some(), "value_index_map table should exist");
    }

    #[tokio::test]
    async fn test_insert_and_get_value_index_mapping() {
        let (manager, pool) = setup_test_db().await;

        // Test inserting a mapping
        let test_value = "test_hash";
        let test_index = 42;

        manager
            .insert_value_index_mapping(&pool, test_value, test_index)
            .await
            .unwrap();

        // Test retrieving the mapping
        let result = manager
            .get_element_index_for_value(&pool, test_value)
            .await
            .unwrap();

        assert_eq!(result, Some(test_index));
    }

    #[tokio::test]
    async fn test_get_all_elements() {
        let (manager, pool) = setup_test_db().await;
        manager
            .insert_value_index_mapping(&pool, "test_hash", 42)
            .await
            .unwrap();
        manager
            .insert_value_index_mapping(&pool, "test_hash2", 43)
            .await
            .unwrap();
        manager
            .insert_value_index_mapping(&pool, "test_hash3", 44)
            .await
            .unwrap();
        let elements = manager.get_all_elements(&pool).await.unwrap();
        assert_eq!(elements.len(), 3);
    }

    #[tokio::test]
    async fn test_get_nonexistent_value_index() {
        let (manager, pool) = setup_test_db().await;

        let result = manager
            .get_element_index_for_value(&pool, "nonexistent")
            .await
            .unwrap();

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_value_for_element_index() {
        let (manager, pool) = setup_test_db().await;

        // First, create the store table that would normally exist
        sqlx::query("CREATE TABLE IF NOT EXISTS store (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();

        // Insert a test value
        let test_index = 123;
        let test_value = "test_stored_value";
        sqlx::query("INSERT INTO store (key, value) VALUES (?, ?)")
            .bind(format!("test:hashes:{}", test_index))
            .bind(test_value)
            .execute(&pool)
            .await
            .unwrap();

        // Test retrieving the value
        let result = manager
            .get_value_for_element_index(&pool, test_index)
            .await
            .unwrap();

        assert_eq!(result, Some(test_value.to_string()));
    }

    #[tokio::test]
    async fn test_get_nonexistent_value_for_element_index() {
        let (manager, pool) = setup_test_db().await;

        // Create store table
        sqlx::query("CREATE TABLE IF NOT EXISTS store (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();

        let result = manager
            .get_value_for_element_index(&pool, 999)
            .await
            .unwrap();

        assert_eq!(result, None);
    }
}
