use clap::Parser;
use methods::MMR_BENCHMARK_ELF;
use publisher::db::DbConnection;
use risc0_zkvm::{default_executor, ExecutorEnv};
use tracing::error;

#[derive(Parser)]
#[command(name = "mmr_benchmark")]
struct Args {
    #[arg(long, default_value = "0")]
    start_block: u64,

    #[arg(long, default_value = "1000")]
    end_block: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let db_connection = DbConnection::new().await.map_err(|e| {
        error!(error = %e, "Failed to create DB connection");
        e
    })?;

    let block_headers = db_connection
        .get_block_headers_by_block_range(args.start_block, args.end_block)
        .await?;

    // Execute the guest code.
    let env = ExecutorEnv::builder().write(&block_headers)?.build()?;
    let exec = default_executor();
    exec.execute(env, MMR_BENCHMARK_ELF)?;

    Ok(())
}
