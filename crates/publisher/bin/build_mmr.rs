use clap::Parser;
use publisher::cli::build_mmr::{run, Args};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    run(args).await
}
