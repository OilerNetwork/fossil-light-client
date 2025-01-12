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
