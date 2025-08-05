//! Builder pattern implementation for the Fossil Light Client.
//!
//! This module provides a fluent API for constructing LightClient instances
//! with optional parameters and clear configuration steps.

use crate::{
    client::LightClient,
    error::Result,
    types::{BatchSize, PollingInterval},
};

/// Builder for creating LightClient instances with a fluent API.
///
/// This builder provides a more ergonomic way to construct LightClient instances,
/// especially when dealing with optional parameters or when you want to clearly
/// express the configuration steps.
///
/// # Example
///
/// ```rust,no_run
/// use client::LightClientBuilder;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = LightClientBuilder::new()
///         .polling_interval_secs(10)
///         .batch_size(2048)
///         .start_block(1000)
///         .blocks_per_run(50)
///         .build()
///         .await?;
///     
///     Ok(())
/// }
/// ```
#[derive(Debug, Default)]
pub struct LightClientBuilder {
    polling_interval_secs: Option<u64>,
    batch_size: Option<u64>,
    start_block: Option<u64>,
    blocks_per_run: Option<u64>,
}

impl LightClientBuilder {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the polling interval in seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - Polling interval in seconds (must be > 0)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::LightClientBuilder;
    /// let builder = LightClientBuilder::new().polling_interval_secs(5);
    /// ```
    pub fn polling_interval_secs(mut self, seconds: u64) -> Self {
        self.polling_interval_secs = Some(seconds);
        self
    }

    /// Sets the batch size for processing.
    ///
    /// # Arguments
    ///
    /// * `size` - Number of blocks to process in each batch (must be > 0)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::LightClientBuilder;
    /// let builder = LightClientBuilder::new().batch_size(1024);
    /// ```
    pub fn batch_size(mut self, size: u64) -> Self {
        self.batch_size = Some(size);
        self
    }

    /// Sets the starting block number.
    ///
    /// # Arguments
    ///
    /// * `block` - The block number to start processing from
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::LightClientBuilder;
    /// let builder = LightClientBuilder::new().start_block(1000);
    /// ```
    pub fn start_block(mut self, block: u64) -> Self {
        self.start_block = Some(block);
        self
    }

    /// Sets the maximum number of blocks to process per run.
    ///
    /// # Arguments
    ///
    /// * `blocks` - Maximum blocks per run (0 for unlimited)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::LightClientBuilder;
    /// let builder = LightClientBuilder::new().blocks_per_run(100);
    /// ```
    pub fn blocks_per_run(mut self, blocks: u64) -> Self {
        self.blocks_per_run = Some(blocks);
        self
    }

    /// Sets polling interval using the PollingInterval type.
    ///
    /// # Arguments
    ///
    /// * `interval` - Typed polling interval
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::{LightClientBuilder, types::PollingInterval};
    /// let interval = PollingInterval::new(5).unwrap();
    /// let builder = LightClientBuilder::new().polling_interval(interval);
    /// ```
    pub fn polling_interval(mut self, interval: PollingInterval) -> Self {
        self.polling_interval_secs = Some(interval.seconds());
        self
    }

    /// Sets batch size using the BatchSize type.
    ///
    /// # Arguments
    ///
    /// * `size` - Typed batch size
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::{LightClientBuilder, types::BatchSize};
    /// let size = BatchSize::new(1024).unwrap();
    /// let builder = LightClientBuilder::new().batch_size_typed(size);
    /// ```
    pub fn batch_size_typed(mut self, size: BatchSize) -> Self {
        self.batch_size = Some(size.value());
        self
    }

    /// Uses default values for common configurations.
    ///
    /// This sets reasonable defaults:
    /// - Polling interval: 5 seconds
    /// - Batch size: 1024 blocks
    /// - Blocks per run: 100 blocks
    ///
    /// The start block must still be specified or use `build_with_auto_start()`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::LightClientBuilder;
    /// let builder = LightClientBuilder::new()
    ///     .with_defaults()
    ///     .start_block(1000);
    /// ```
    pub fn with_defaults(mut self) -> Self {
        self.polling_interval_secs = Some(PollingInterval::DEFAULT.seconds());
        self.batch_size = Some(BatchSize::DEFAULT.value());
        self.blocks_per_run = Some(100);
        self
    }

    /// Configures for a fast polling scenario.
    ///
    /// This sets values optimized for fast processing:
    /// - Polling interval: 1 second
    /// - Batch size: 100 blocks (smaller for faster processing)
    /// - Blocks per run: 50 blocks
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::LightClientBuilder;
    /// let builder = LightClientBuilder::new()
    ///     .fast_mode()
    ///     .start_block(1000);
    /// ```
    pub fn fast_mode(mut self) -> Self {
        self.polling_interval_secs = Some(PollingInterval::FAST.seconds());
        self.batch_size = Some(BatchSize::SMALL.value());
        self.blocks_per_run = Some(50);
        self
    }

    /// Configures for a high-throughput scenario.
    ///
    /// This sets values optimized for processing large volumes:
    /// - Polling interval: 30 seconds
    /// - Batch size: 10000 blocks
    /// - Blocks per run: 1000 blocks
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::LightClientBuilder;
    /// let builder = LightClientBuilder::new()
    ///     .high_throughput_mode()
    ///     .start_block(1000);
    /// ```
    pub fn high_throughput_mode(mut self) -> Self {
        self.polling_interval_secs = Some(PollingInterval::SLOW.seconds());
        self.batch_size = Some(BatchSize::LARGE.value());
        self.blocks_per_run = Some(1000);
        self
    }

    /// Builds the LightClient with the configured parameters.
    ///
    /// Uses default values for any parameters not explicitly set:
    /// - Polling interval: 5 seconds
    /// - Batch size: 1024 blocks
    /// - Start block: 0
    /// - Blocks per run: 100 blocks
    ///
    /// # Returns
    ///
    /// Returns a configured LightClient instance.
    ///
    /// # Errors
    ///
    /// This method can fail for the same reasons as `LightClient::new()`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::LightClientBuilder;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LightClientBuilder::new()
    ///     .polling_interval_secs(10)
    ///     .batch_size(2048)
    ///     .start_block(1000)
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(self) -> Result<LightClient> {
        let polling_interval = self.polling_interval_secs.unwrap_or(5);
        let batch_size = self.batch_size.unwrap_or(1024);
        let start_block = self.start_block.unwrap_or(0);
        let blocks_per_run = self.blocks_per_run.unwrap_or(100);

        LightClient::new(polling_interval, batch_size, start_block, blocks_per_run).await
    }

    /// Builds the LightClient with automatic start block detection.
    ///
    /// This is equivalent to calling `build()` and then using the client to
    /// determine the latest relayed block, but it's done automatically.
    ///
    /// # Returns
    ///
    /// Returns a configured LightClient instance with the start block set to
    /// the latest relayed block + 1.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use client::LightClientBuilder;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LightClientBuilder::new()
    ///     .polling_interval_secs(10)
    ///     .batch_size(2048)
    ///     .build_with_auto_start()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build_with_auto_start(self) -> Result<LightClient> {
        let polling_interval = self.polling_interval_secs.unwrap_or(5);
        let batch_size = self.batch_size.unwrap_or(1024);
        let blocks_per_run = self.blocks_per_run.unwrap_or(100);

        LightClient::new_with_default_start(polling_interval, batch_size, blocks_per_run).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let builder = LightClientBuilder::new();
        assert!(builder.polling_interval_secs.is_none());
        assert!(builder.batch_size.is_none());
        assert!(builder.start_block.is_none());
        assert!(builder.blocks_per_run.is_none());
    }

    #[test]
    fn test_builder_chaining() {
        let builder = LightClientBuilder::new()
            .polling_interval_secs(10)
            .batch_size(2048)
            .start_block(1000)
            .blocks_per_run(50);

        assert_eq!(builder.polling_interval_secs, Some(10));
        assert_eq!(builder.batch_size, Some(2048));
        assert_eq!(builder.start_block, Some(1000));
        assert_eq!(builder.blocks_per_run, Some(50));
    }

    #[test]
    fn test_typed_setters() {
        let interval = PollingInterval::new(5).unwrap();
        let batch_size = BatchSize::new(1024).unwrap();

        let builder = LightClientBuilder::new()
            .polling_interval(interval)
            .batch_size_typed(batch_size);

        assert_eq!(builder.polling_interval_secs, Some(5));
        assert_eq!(builder.batch_size, Some(1024));
    }

    #[test]
    fn test_preset_configurations() {
        // Test defaults
        let builder = LightClientBuilder::new().with_defaults();
        assert_eq!(builder.polling_interval_secs, Some(5));
        assert_eq!(builder.batch_size, Some(1024));
        assert_eq!(builder.blocks_per_run, Some(100));

        // Test fast mode
        let builder = LightClientBuilder::new().fast_mode();
        assert_eq!(builder.polling_interval_secs, Some(1));
        assert_eq!(builder.batch_size, Some(100));
        assert_eq!(builder.blocks_per_run, Some(50));

        // Test high throughput mode
        let builder = LightClientBuilder::new().high_throughput_mode();
        assert_eq!(builder.polling_interval_secs, Some(30));
        assert_eq!(builder.batch_size, Some(10000));
        assert_eq!(builder.blocks_per_run, Some(1000));
    }
}
