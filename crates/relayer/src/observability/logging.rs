use std::env;

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize structured logging for the relayer
///
/// This sets up tracing with configurable output format and log levels.
/// The log level can be controlled via the `RUST_LOG` environment variable.
///
/// # Examples
///
/// ```
/// use relayer::observability::init_logging;
///
/// // Initialize with default settings (may fail if already initialized)
/// let _ = init_logging();
///
/// // Set log level via environment
/// std::env::set_var("RUST_LOG", "relayer=debug");
/// let _ = init_logging(); // May fail if already initialized, which is ok
/// ```
pub fn init_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let format = env::var("LOG_FORMAT").unwrap_or_else(|_| "text".to_string());

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("relayer=info"));

    match format.as_str() {
        "json" => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().json())
                .try_init()?;
        }
        _ => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().with_target(true).with_thread_ids(true))
                .try_init()?;
        }
    }

    tracing::info!(
        log_format = %format,
        "Structured logging initialized"
    );

    Ok(())
}

/// Initialize logging with custom configuration
///
/// # Arguments
///
/// * `json_format` - Whether to use JSON output format
/// * `log_level` - Custom log level filter
pub fn init_logging_with_config(
    json_format: bool,
    log_level: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env_filter = EnvFilter::new(log_level);

    if json_format {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json())
            .try_init()?;
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_target(true).with_thread_ids(true))
            .try_init()?;
    }

    tracing::info!(
        json_format = json_format,
        log_level = %log_level,
        "Structured logging initialized with custom config"
    );

    Ok(())
}

/// Logging configuration for the relayer
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Whether to use JSON format for logs
    pub json_format: bool,
    /// Log level filter string
    pub log_level: String,
    /// Whether to include thread IDs in logs
    pub include_thread_ids: bool,
    /// Whether to include targets in logs
    pub include_targets: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            json_format: false,
            log_level: "relayer=info".to_string(),
            include_thread_ids: true,
            include_targets: true,
        }
    }
}

impl LoggingConfig {
    /// Create a new logging configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set JSON format
    pub const fn with_json_format(mut self, json_format: bool) -> Self {
        self.json_format = json_format;
        self
    }

    /// Set log level
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = level.into();
        self
    }

    /// Set whether to include thread IDs
    pub const fn with_thread_ids(mut self, include: bool) -> Self {
        self.include_thread_ids = include;
        self
    }

    /// Set whether to include targets
    pub const fn with_targets(mut self, include: bool) -> Self {
        self.include_targets = include;
        self
    }

    /// Initialize logging with this configuration
    pub fn init(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let env_filter = EnvFilter::new(&self.log_level);

        if self.json_format {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().json())
                .try_init()?;
        } else {
            let fmt_layer = fmt::layer()
                .with_target(self.include_targets)
                .with_thread_ids(self.include_thread_ids);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()?;
        }

        tracing::info!(
            json_format = self.json_format,
            log_level = %self.log_level,
            include_thread_ids = self.include_thread_ids,
            include_targets = self.include_targets,
            "Structured logging initialized"
        );

        Ok(())
    }
}
