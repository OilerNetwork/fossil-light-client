#![deny(unused_crate_dependencies)]

use dotenv::dotenv;
use serde_json;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::str;
use thiserror::Error;
use tokio::task;
use tracing::{info, warn};

// Define constant for max file size (50MB)
pub const DEFAULT_MAX_FILE_SIZE: usize = 50 * 1024 * 1024;

#[derive(Error, Debug)]
pub enum IpfsError {
    #[error("Failed to connect to IPFS node: {0}")]
    ConnectionError(String),
    #[error("File operation failed: {0}")]
    FileError(#[from] std::io::Error),
    #[error("Backend operation failed: {0}")]
    BackendError(String),
    #[error("Invalid IPFS hash: {0}")]
    InvalidHash(String),
    #[error("Response error: {0}")]
    ResponseError(String),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Curl operation failed: {0}")]
    CurlError(#[from] curl::Error),
    #[error("Environment variable not found: {0}")]
    EnvError(String),
}

#[derive(Clone)]
pub struct IpfsManager {
    add_url: String,
    fetch_base_url: String,
    token: String,
    pub max_file_size: usize,
}

impl IpfsManager {
    pub fn with_endpoint() -> Result<Self, IpfsError> {
        // Load environment variables
        dotenv().ok();

        // Get IPFS configuration from environment variables
        let add_url = env::var("IPFS_ADD_URL").map_err(|_| {
            IpfsError::EnvError("IPFS_ADD_URL not found in environment".to_string())
        })?;

        let fetch_base_url = env::var("IPFS_FETCH_BASE_URL").map_err(|_| {
            IpfsError::EnvError("IPFS_FETCH_BASE_URL not found in environment".to_string())
        })?;

        let token = env::var("IPFS_TOKEN")
            .map_err(|_| IpfsError::EnvError("IPFS_TOKEN not found in environment".to_string()))?;

        // Use default max file size
        Ok(IpfsManager {
            add_url,
            fetch_base_url,
            token,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
        })
    }

    pub async fn upload_db(&self, file_path: &Path) -> Result<String, IpfsError> {
        let metadata = fs::metadata(file_path).map_err(IpfsError::FileError)?;

        let contents = fs::read(&file_path)?;
        if !contents.starts_with(b"SQLite format 3\0") {
            return Err(IpfsError::FileError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "File is not a valid SQLite database",
            )));
        }

        if metadata.len() as usize > self.max_file_size {
            warn!(
                "File size exceeds limit: {} bytes > {} bytes",
                metadata.len(),
                self.max_file_size
            );
            return Err(IpfsError::BackendError(format!(
                "File size {} bytes exceeds maximum allowed size {} bytes",
                metadata.len(),
                self.max_file_size,
            )));
        }

        let mut easy = curl::easy::Easy::new();
        easy.url(&self.add_url)?;

        // Set up authorization header exactly like the curl command
        let mut list = curl::easy::List::new();
        list.append(&format!("Authorization: Bearer {}", self.token))?;
        easy.http_headers(list)?;

        // Set POST method
        easy.post(true)?;

        // Set up the form data
        let mut form = curl::easy::Form::new();
        form.part("file")
            .file(file_path.to_str().ok_or_else(|| {
                IpfsError::BackendError("Invalid file path (non-UTF8)".to_string())
            })?)
            .add()
            .map_err(|e| IpfsError::BackendError(e.to_string()))?;
        easy.httppost(form)?;

        // Capture the response
        let mut response_data = Vec::new();
        {
            let mut transfer = easy.transfer();
            transfer.write_function(|data| {
                response_data.extend_from_slice(data);
                Ok(data.len())
            })?;
            transfer.perform()?;
        }

        // Check response code
        let response_code = easy.response_code()?;
        if response_code != 200 {
            return Err(IpfsError::ResponseError(format!(
                "HTTP error: {} for URL: {}",
                response_code, self.add_url
            )));
        }

        // Parse response
        let response_str = str::from_utf8(&response_data)
            .map_err(|e| IpfsError::ResponseError(format!("Invalid UTF-8 sequence: {}", e)))?;
        let response_json: serde_json::Value = serde_json::from_str(response_str)?;

        let hash = response_json["Hash"]
            .as_str()
            .ok_or_else(|| IpfsError::BackendError("No hash in response".to_string()))?
            .to_string();

        info!("IPFS upload completed successfully, CID: {}", hash);
        Ok(hash)
    }

    pub async fn fetch_db(&self, hash: &str, output_path: &Path) -> Result<(), IpfsError> {
        let fetch_url = format!("{}{}", self.fetch_base_url, hash);

        let mut easy = curl::easy::Easy::new();
        easy.url(&fetch_url)?;
        easy.follow_location(true)?;

        // Set up authorization header exactly like the curl command
        let mut list = curl::easy::List::new();
        list.append(&format!("Authorization: Bearer {}", self.token))?;
        easy.http_headers(list)?;

        // Create output file
        let mut file = std::fs::File::create(output_path)?;

        // Set up the transfer to write directly to file
        {
            let mut transfer = easy.transfer();
            transfer.write_function(|data| {
                file.write_all(data).unwrap();
                Ok(data.len())
            })?;
            transfer.perform()?;
        }

        // Check response code after transfer
        let response_code = easy.response_code()?;
        if response_code != 200 {
            // Clean up failed download
            std::fs::remove_file(&output_path)?;
            return Err(IpfsError::ResponseError(format!(
                "HTTP error: {} for URL: {}",
                response_code, fetch_url
            )));
        }

        Ok(())
    }

    pub async fn check_connection(&self) -> Result<(), IpfsError> {
        let version_url = "http://100.25.44.230:5001/api/v0/version".to_string();
        let token = self.token.clone();
        task::spawn_blocking(move || -> Result<(), IpfsError> {
            let mut easy = curl::easy::Easy::new();
            easy.url(&version_url)
                .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            let header_value = format!("Authorization: Bearer {}", token);
            let mut list = curl::easy::List::new();
            list.append(&header_value)
                .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            easy.http_headers(list)
                .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            let mut response_data = Vec::new();
            {
                let mut transfer = easy.transfer();
                transfer
                    .write_function(|data| {
                        response_data.extend_from_slice(data);
                        Ok(data.len())
                    })
                    .map_err(|e| IpfsError::BackendError(e.to_string()))?;
                transfer
                    .perform()
                    .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            }
            Ok(())
        })
        .await
        .map_err(|e| IpfsError::BackendError(e.to_string()))??;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::NamedTempFile;
    use tokio::fs;
    use tokio::io::AsyncWriteExt;

    // Setup environment variables for tests
    fn setup_test_env() {
        env::set_var("IPFS_ADD_URL", "http://100.25.44.230:5001/api/v0/add");
        env::set_var("IPFS_FETCH_BASE_URL", "http://100.25.44.230/ipfs/");
        env::set_var("IPFS_TOKEN", "YgkUzg1TQGWZvb0QwrkPoO2TIgkwEuE9MVWwJuNZ4pk=");
    }

    // Helper function to create a temporary SQLite database file
    async fn create_temp_db_file() -> Result<NamedTempFile, std::io::Error> {
        let temp_file = NamedTempFile::new()?;
        let mut file = tokio::fs::File::create(temp_file.path()).await?;

        // Write SQLite header to make it a valid SQLite file
        file.write_all(b"SQLite format 3\0").await?;
        // Add some dummy data
        file.write_all(&[0; 1024]).await?;

        Ok(temp_file)
    }

    #[tokio::test]
    async fn test_ipfs_manager_creation() {
        setup_test_env();
        let result = IpfsManager::with_endpoint();
        assert!(result.is_ok());

        let manager = result.unwrap();
        assert_eq!(manager.max_file_size, DEFAULT_MAX_FILE_SIZE);
    }

    #[tokio::test]
    async fn test_upload_invalid_file_type() {
        let manager = IpfsManager::with_endpoint().unwrap();

        // Create a temporary file that is not a SQLite database
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let result = manager.upload_db(path).await;
        assert!(result.is_err());

        match result {
            Err(IpfsError::FileError(e)) => {
                assert_eq!(e.kind(), std::io::ErrorKind::InvalidData);
            }
            _ => panic!("Expected FileError with InvalidData kind"),
        }
    }

    #[tokio::test]
    async fn test_upload_file_too_large() {
        let mut manager = IpfsManager::with_endpoint().unwrap();
        // Set a very small max file size for testing
        manager.max_file_size = 10; // 10 bytes

        // Create a temporary SQLite database file
        let temp_file = create_temp_db_file().await.unwrap();
        let path = temp_file.path();

        let result = manager.upload_db(path).await;
        assert!(result.is_err());

        match result {
            Err(IpfsError::BackendError(msg)) => {
                assert!(msg.contains("exceeds maximum allowed size"));
            }
            _ => panic!("Expected BackendError about file size"),
        }
    }

    #[tokio::test]
    #[ignore] // Ignore by default as it requires a real IPFS node
    async fn test_upload_and_fetch_integration() {
        let manager = IpfsManager::with_endpoint().unwrap();

        // Create a temporary SQLite database file
        let temp_file = create_temp_db_file().await.unwrap();
        let upload_path = temp_file.path();

        // Upload the file
        let hash = manager
            .upload_db(upload_path)
            .await
            .expect("Failed to upload file");
        assert!(!hash.is_empty(), "Hash should not be empty");

        // Create a temporary file for download
        let download_file = NamedTempFile::new().unwrap();
        let download_path = download_file.path();

        // Fetch the file
        manager
            .fetch_db(&hash, download_path)
            .await
            .expect("Failed to fetch file");

        // Verify file exists and has content
        let metadata = fs::metadata(download_path)
            .await
            .expect("Failed to get metadata");
        assert!(metadata.len() > 0, "Downloaded file should not be empty");

        // Verify it's a valid SQLite file
        let content = fs::read(download_path).await.expect("Failed to read file");
        assert!(
            content.starts_with(b"SQLite format 3\0"),
            "Not a valid SQLite file"
        );
    }

    #[tokio::test]
    async fn test_check_connection() {
        let manager = IpfsManager::with_endpoint().unwrap();

        // This test is marked as ignore because it requires a real IPFS node
        // But we can still write the test and run it manually when needed
        let result = manager.check_connection().await;

        // We don't assert the result since it depends on external service
        // Just ensure the function doesn't panic
        match result {
            Ok(_) => println!("Connection successful"),
            Err(e) => println!("Connection failed: {}", e),
        }
    }

    #[tokio::test]
    async fn test_fetch_nonexistent_hash() {
        let manager = IpfsManager::with_endpoint().unwrap();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Try to fetch a file with a non-existent hash
        let result = manager
            .fetch_db("QmInvalidHashThatDoesNotExist", path)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_error_handling() {
        // Test IpfsError Display implementation
        let error = IpfsError::ConnectionError("test error".to_string());
        assert_eq!(
            error.to_string(),
            "Failed to connect to IPFS node: test error"
        );

        let error = IpfsError::FileError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert_eq!(error.to_string(), "File operation failed: file not found");

        let error = IpfsError::InvalidHash("invalid hash".to_string());
        assert_eq!(error.to_string(), "Invalid IPFS hash: invalid hash");
    }
}
