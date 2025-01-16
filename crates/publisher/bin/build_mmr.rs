use publisher::cli::build_mmr::{Args, run};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    run(args).await
}
