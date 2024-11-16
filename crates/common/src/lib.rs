use eyre::Result;
use std::str::FromStr;

/// Retrieves an environment variable or returns an error if not set.
pub fn get_env_var(key: &str) -> Result<String> {
    dotenv::var(key).map_err(|_| eyre::eyre!("Environment variable {} not set", key))
}

/// Parses an environment variable into the desired type or returns an error.
pub fn get_var<T: FromStr>(name: &str) -> Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let var_value = get_env_var(name)?;
    var_value
        .parse::<T>()
        .map_err(|e| eyre::eyre!("Unable to parse {} environment variable: {}", name, e))
}

/// Function to initialize logging and environment variables
pub fn initialize_logger_and_env() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt().init();

    Ok(())
}
