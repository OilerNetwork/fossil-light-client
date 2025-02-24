#![deny(unused_crate_dependencies)]

use std::fs;
use std::io::Write;
use std::path::Path;
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
            token: "YgkUzg1TQGWZvb0QwrkPoO2TIgkwEuE9MVWwJuNZ4pk".to_string(),
            max_file_size: 50 * 1024 * 1024, // 50MB
        })
    }

    pub async fn upload_db(&self, file_path: &Path) -> Result<String, IpfsError> {
        let metadata = fs::metadata(file_path).map_err(IpfsError::FileError)?;
        info!("Starting IPFS upload, size: {} bytes", metadata.len());

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

        let file_path = file_path.to_owned();
        let add_url = self.add_url.clone();
        let token = self.token.clone();
        let result = task::spawn_blocking(move || -> Result<String, IpfsError> {
            let mut easy = curl::easy::Easy::new();
            easy.url(&add_url)
                .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            let header_value = format!("Authorization: Bearer {}", token);
            let mut list = curl::easy::List::new();
            list.append(&header_value)
                .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            easy.http_headers(list)
                .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            let mut form = curl::easy::Form::new();
            let file_str = file_path.to_str().ok_or_else(|| {
                IpfsError::BackendError("Invalid file path (non-UTF8)".to_string())
            })?;
            form.part("file")
                .file(file_str)
                .add()
                .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            easy.httppost(form)
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
            String::from_utf8(response_data).map_err(|e| IpfsError::BackendError(e.to_string()))
        })
        .await
        .map_err(|e| IpfsError::BackendError(e.to_string()))??;

        info!("IPFS upload completed successfully, CID: {}", result);
        Ok(result)
    }

    pub async fn fetch_db(&self, hash: &str, output_path: &Path) -> Result<(), IpfsError> {
        info!("Starting IPFS download, CID: {}", hash);

        if !hash.starts_with("Qm") {
            warn!("Invalid IPFS hash format: {}", hash);
            return Err(IpfsError::InvalidHash(hash.to_string()));
        }

        let fetch_url = format!("{}{}", self.fetch_base_url, hash);
        let output_path = output_path.to_owned();
        let token = self.token.clone();
        task::spawn_blocking(move || -> Result<(), IpfsError> {
            let mut easy = curl::easy::Easy::new();
            easy.url(&fetch_url)
                .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            let header_value = format!("Authorization: Bearer {}", token);
            let mut list = curl::easy::List::new();
            list.append(&header_value)
                .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            easy.http_headers(list)
                .map_err(|e| IpfsError::BackendError(e.to_string()))?;
            let mut file = fs::File::create(&output_path).map_err(IpfsError::FileError)?;
            {
                let mut transfer = easy.transfer();
                transfer
                    .write_function(|data| {
                        file.write_all(data)
                            .map(|_| data.len())
                            .map_err(|_e| curl::easy::WriteError::Pause)
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
