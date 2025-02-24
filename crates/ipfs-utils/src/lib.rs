#![deny(unused_crate_dependencies)]

use serde_json;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::str;
use thiserror::Error;
use tokio::task;
use tracing::{info, warn};

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
        Ok(IpfsManager {
            add_url: "http://100.25.44.230:5001/api/v0/add".to_string(),
            fetch_base_url: "http://100.25.44.230/ipfs/".to_string(),
            token: "YgkUzg1TQGWZvb0QwrkPoO2TIgkwEuE9MVWwJuNZ4pk=".to_string(),
            max_file_size: 50 * 1024 * 1024, // 50MB
        })
    }

    pub async fn upload_db(&self, file_path: &Path) -> Result<String, IpfsError> {
        info!("Starting IPFS upload for file: {}", file_path.display());

        let metadata = fs::metadata(file_path).map_err(IpfsError::FileError)?;
        info!("File size: {} bytes", metadata.len());

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
        info!("Starting IPFS download, CID: {}", hash);

        let fetch_url = format!("{}{}", self.fetch_base_url, hash);
        info!("Fetching from IPFS URL: {}", fetch_url);

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

        info!("IPFS download completed successfully, CID: {}", hash);
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
