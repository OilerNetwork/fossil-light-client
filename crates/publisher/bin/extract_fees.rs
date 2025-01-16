use clap::Parser;
use common::initialize_logger_and_env;
use publisher::cli::extract_fees::{run, Args, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_logger_and_env()?;

    let config = Config::from_env()?;
    let args = Args::parse();

    run(config, args).await
}
