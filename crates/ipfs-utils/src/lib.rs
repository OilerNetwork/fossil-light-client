use anyhow::Result;
use futures_util::StreamExt;
use ipfs_api::{IpfsApi, IpfsClient};
use std::io::Cursor;
use std::path::Path;

pub struct IpfsManager {
    client: IpfsClient,
}

impl IpfsManager {
    pub fn new() -> Self {
        let client = IpfsClient::default();
        Self { client }
    }

    /// Upload a .db file to IPFS
    pub async fn upload_db(&self, file_path: &Path) -> Result<String> {
        let data = std::fs::read(file_path)?;
        // Convert Vec<u8> to Cursor<Vec<u8>> which implements Read
        let cursor = Cursor::new(data);
        let res = self.client.add(cursor).await?;
        Ok(res.hash)
    }

    /// Fetch a .db file from IPFS using its hash
    pub async fn fetch_db(&self, hash: &str, output_path: &Path) -> Result<()> {
        // Collect all chunks from the stream
        let mut stream = self.client.cat(hash);
        let mut bytes = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            bytes.extend_from_slice(&chunk);
        }

        std::fs::write(output_path, bytes)?;
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

        // Try to fetch to an invalid path (directory that doesn't exist)
        let invalid_path = PathBuf::from("nonexistent_dir/test.db");
        let result = ipfs.fetch_db(&hash, &invalid_path).await;
        assert!(result.is_err());

        // Cleanup
        std::fs::remove_file("test_invalid_path.db").unwrap();
    }
}
