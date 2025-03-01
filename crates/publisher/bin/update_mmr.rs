use clap::Parser;
use common::initialize_logger;
use publisher::cli::update_mmr::{run, Args, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_logger()?;

    let config = Config::from_env()?;
    let args = Args::parse();

    run(config, args).await
}
