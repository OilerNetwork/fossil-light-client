use anyhow::{Context, Result};
use futures_util::StreamExt;
use ipfs_api::{IpfsApi, IpfsClient};
use std::io::Cursor;
use std::path::Path;
use thiserror::Error;
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
        info!("Uploading database file to IPFS: {:?}", file_path);

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

        let mut stream = self.client.cat(hash);
        let mut bytes = Vec::with_capacity(1024 * 1024); // Preallocate 1MB
        let mut total_size = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read chunk from IPFS")?;
            total_size += chunk.len();

            if total_size > self.max_file_size {
                error!("Downloaded file size exceeds maximum allowed size");
                return Err(anyhow::anyhow!(
                    "Downloaded file size exceeds maximum allowed size"
                ));
            }

            bytes.extend_from_slice(&chunk);
        }

        // Atomic write using temporary file
        let temp_path = output_path.with_extension("tmp");
        std::fs::write(&temp_path, &bytes).context("Failed to write temporary file")?;

        std::fs::rename(temp_path, output_path).context("Failed to rename temporary file")?;

        info!("Successfully fetched file to {:?}", output_path);
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
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_upload_and_fetch() {
        let ipfs = IpfsManager::new();

        // Create a test db file
        let test_data = b"test database content";
        let test_file = PathBuf::from("test.db");
        std::fs::write(&test_file, test_data).unwrap();

        // Upload the file and print the CID
        let hash = ipfs.upload_db(&test_file).await.unwrap();
        println!("File CID: {}", hash);

        // Fetch the file
        let output_file = PathBuf::from("fetched_test.db");
        ipfs.fetch_db(&hash, &output_file).await.unwrap();

        // Verify contents
        let fetched_data = std::fs::read(output_file).unwrap();
        assert_eq!(fetched_data, test_data);

        // Cleanup
        std::fs::remove_file("test.db").unwrap();
        std::fs::remove_file("fetched_test.db").unwrap();
    }

    #[tokio::test]
    async fn test_upload_nonexistent_file() {
        let ipfs = IpfsManager::new();
        let nonexistent_file = PathBuf::from("does_not_exist.db");

        let result = ipfs.upload_db(&nonexistent_file).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_upload_and_fetch_empty_file() {
        let ipfs = IpfsManager::new();

        // Create an empty test file
        let test_file = PathBuf::from("empty.db");
        std::fs::write(&test_file, b"").unwrap();

        // Upload empty file
        let hash = ipfs.upload_db(&test_file).await.unwrap();
        println!("Empty file CID: {}", hash);

        // Fetch the empty file
        let output_file = PathBuf::from("fetched_empty.db");
        ipfs.fetch_db(&hash, &output_file).await.unwrap();

        // Verify contents are empty
        let fetched_data = std::fs::read(&output_file).unwrap();
        assert!(fetched_data.is_empty());

        // Cleanup
        std::fs::remove_file("empty.db").unwrap();
        std::fs::remove_file("fetched_empty.db").unwrap();
    }

    #[tokio::test]
    async fn test_fetch_invalid_hash() {
        let ipfs = IpfsManager::new();
        let output_file = PathBuf::from("should_not_exist.db");

        // Try to fetch with invalid hash
        let result = ipfs.fetch_db("QminvalidHashValue", &output_file).await;
        assert!(result.is_err());

        // Verify the output file wasn't created
        assert!(!output_file.exists());
    }

    #[tokio::test]
    async fn test_fetch_to_invalid_path() {
        let ipfs = IpfsManager::new();

        // Create and upload a test file first
        let test_file = PathBuf::from("test_invalid_path.db");
        std::fs::write(&test_file, b"test content").unwrap();
        let hash = ipfs.upload_db(&test_file).await.unwrap();

        // Try to fetch to a path with invalid permissions
        let invalid_path = PathBuf::from("/root/test.db");
        let result = ipfs.fetch_db(&hash, &invalid_path).await;
        assert!(result.is_err());

        // Cleanup
        std::fs::remove_file("test_invalid_path.db").unwrap();
    }
}
