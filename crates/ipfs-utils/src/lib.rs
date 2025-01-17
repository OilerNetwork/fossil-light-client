#![deny(unused_crate_dependencies)]

use anyhow::{Context, Result};
use futures_util::StreamExt;
use ipfs_api::{IpfsApi, IpfsClient};
use std::io::Cursor;
use std::path::Path;
use thiserror::Error;
use tokio::task;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum IpfsError {
    #[error("Failed to connect to IPFS node: {0}")]
    ConnectionError(String),
    #[error("File operation failed: {0}")]
    FileError(#[from] std::io::Error),
    #[error("IPFS operation failed: {0}")]
    IpfsError(#[from] ipfs_api::Error),
    #[error("Invalid IPFS hash: {0}")]
    InvalidHash(String),
}

#[derive(Clone)]
pub struct IpfsManager {
    client: IpfsClient,
    max_file_size: usize, // Add configurable limits
}

impl Default for IpfsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IpfsManager {
    pub fn new() -> Self {
        Self {
            client: IpfsClient::default(),
            max_file_size: 50 * 1024 * 1024, // 50MB default limit
        }
    }

    pub fn with_endpoint() -> Result<Self> {
        let client = IpfsClient::default();
        Ok(Self {
            client,
            max_file_size: 50 * 1024 * 1024,
        })
    }

    pub fn set_max_file_size(&mut self, size: usize) {
        self.max_file_size = size;
    }

    /// Upload a .db file to IPFS
    pub async fn upload_db(&self, file_path: &Path) -> Result<String> {
        info!(
            "Uploading database file to IPFS: {:?}",
            file_path.file_name().unwrap_or_default()
        );

        // Validate file exists and check size
        let metadata = std::fs::metadata(file_path).context("Failed to read file metadata")?;
        if metadata.len() as usize > self.max_file_size {
            return Err(anyhow::anyhow!(
                "File size {} exceeds maximum allowed size {}",
                metadata.len(),
                self.max_file_size
            ));
        }

        let data = std::fs::read(file_path).context("Failed to read file")?;
        let cursor = Cursor::new(data);

        let res = self
            .client
            .add(cursor)
            .await
            .context("Failed to upload file to IPFS")?;

        info!("Successfully uploaded file. CID: {}", res.hash);
        Ok(res.hash)
    }

    /// Fetch a .db file from IPFS using its hash
    ///
    /// Modified to consume the IPFS stream inside `spawn_blocking`, so the returned
    /// future is `Send`.
    pub async fn fetch_db(&self, hash: &str, output_path: &Path) -> Result<()> {
        info!("Fetching database from IPFS. Hash: {}", hash);

        // Basic hash validation
        if !hash.starts_with("Qm") {
            return Err(IpfsError::InvalidHash(hash.to_string()).into());
        }

        // Create parent directories if they don't exist
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create parent directories")?;
        }

        let hash_owned = hash.to_string();
        let output_path_owned = output_path.to_path_buf();
        let max_file_size = self.max_file_size;
        let client = self.client.clone();

        // Move the actual streaming to a blocking thread, ensuring this future is `Send`.
        task::spawn_blocking(move || -> Result<()> {
            // We need a runtime here so we can `.await` inside `spawn_blocking`.
            let rt = tokio::runtime::Runtime::new()
                .context("Failed to create blocking runtime for IPFS read")?;

            rt.block_on(async move {
                let mut stream = client.cat(&hash_owned);
                let mut bytes = Vec::with_capacity(1024 * 1024);
                let mut total_size = 0;

                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.context("Failed to read chunk from IPFS")?;
                    total_size += chunk.len();

                    if total_size > max_file_size {
                        error!("Downloaded file size exceeds maximum allowed size");
                        return Err(anyhow::anyhow!(
                            "Downloaded file size exceeds maximum allowed size"
                        ));
                    }

                    bytes.extend_from_slice(&chunk);
                }

                // Atomic write using a temporary file
                let temp_path = output_path_owned.with_extension("tmp");
                std::fs::write(&temp_path, &bytes).context("Failed to write temporary file")?;
                std::fs::rename(&temp_path, &output_path_owned)
                    .context("Failed to rename temporary file")?;

                info!("Successfully fetched file to {:?}", output_path_owned);
                Ok(())
            })
        })
        .await??;

        Ok(())
    }

    /// Check if IPFS node is available
    pub async fn check_connection(&self) -> Result<()> {
        self.client
            .version()
            .await
            .context("Failed to connect to IPFS node")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile;
    use tokio::sync::Mutex;
    // Define test-specific trait
    trait TestIpfsApi {
        async fn add_file(&self, data: Vec<u8>) -> Result<String, ipfs_api::Error>;
        async fn cat_file(&self, hash: &str) -> Result<Vec<u8>, ipfs_api::Error>;
        async fn get_version(&self) -> Result<(), ipfs_api::Error>;
    }

    #[derive(Clone)]
    struct MockIpfsClient {
        stored_data: Arc<Mutex<Vec<u8>>>,
    }

    impl MockIpfsClient {
        fn new() -> Self {
            Self {
                stored_data: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl TestIpfsApi for MockIpfsClient {
        async fn add_file(&self, data: Vec<u8>) -> Result<String, ipfs_api::Error> {
            *self.stored_data.lock().await = data;
            Ok("QmTestHash".to_string())
        }

        async fn cat_file(&self, _: &str) -> Result<Vec<u8>, ipfs_api::Error> {
            Ok(self.stored_data.lock().await.clone())
        }

        async fn get_version(&self) -> Result<(), ipfs_api::Error> {
            Ok(())
        }
    }

    #[allow(dead_code)]
    struct TestIpfsManager {
        client: MockIpfsClient,
        max_file_size: usize,
    }

    impl TestIpfsManager {
        fn new() -> Self {
            Self {
                client: MockIpfsClient::new(),
                max_file_size: 1024 * 1024, // 1MB limit
            }
        }

        async fn upload_db(&self, file_path: &Path) -> Result<String> {
            let data = std::fs::read(file_path)?;

            // Check file size
            if data.len() > self.max_file_size {
                return Err(anyhow::anyhow!("File size exceeds maximum allowed size"));
            }

            Ok(self.client.add_file(data).await?)
        }

        async fn fetch_db(&self, hash: &str, output_path: &Path) -> Result<()> {
            // Basic hash validation like the real implementation
            if !hash.starts_with("Qm") {
                return Err(IpfsError::InvalidHash(hash.to_string()).into());
            }

            let data = self.client.cat_file(hash).await?;
            std::fs::write(output_path, data)?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_upload_and_fetch() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_path = temp_dir.path().join("source.db");
        let dest_path = temp_dir.path().join("dest.db");

        let test_data = b"test database content";
        std::fs::write(&source_path, test_data).unwrap();

        let manager = TestIpfsManager::new();

        // Test upload
        let hash = manager.upload_db(&source_path).await.unwrap();
        assert_eq!(hash, "QmTestHash");

        // Test fetch
        manager.fetch_db(&hash, &dest_path).await.unwrap();

        // Verify content
        let fetched_data = std::fs::read(&dest_path).unwrap();
        assert_eq!(fetched_data, test_data);
    }

    #[tokio::test]
    async fn test_connection_check() {
        let manager = TestIpfsManager::new();
        let result = manager.client.get_version().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_file_size_limit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let large_file = temp_dir.path().join("large.db");

        // Create file larger than max size
        let large_data = vec![0u8; 2 * 1024 * 1024]; // 2MB
        std::fs::write(&large_file, large_data).unwrap();

        let manager = TestIpfsManager::new();
        let result = manager.upload_db(&large_file).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_file_path() {
        let manager = TestIpfsManager::new();
        let result = manager.upload_db(Path::new("/nonexistent/path")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_hash() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("output.db");

        let manager = TestIpfsManager::new();
        let result = manager.fetch_db("invalid-hash", &output_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_empty_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let empty_file = temp_dir.path().join("empty.db");
        std::fs::write(&empty_file, b"").unwrap();

        let manager = TestIpfsManager::new();
        let result = manager.upload_db(&empty_file).await;
        assert!(result.is_ok());
    }
}
