use clap::Parser;
use common::{get_env_var, initialize_logger};
use guest_types::CombinedInput;
use methods::MMR_BENCHMARK_ELF;
use publisher::{core::group_headers_by_hour, db::DbConnection};
use risc0_zkvm::{default_executor, ExecutorEnv};
use tracing::error;

#[derive(Parser)]
#[command(name = "mmr_benchmark")]
struct Args {
    #[arg(long, default_value = "7000000")]
    start_block: u64,

    #[arg(long, default_value = "7001023")]
    end_block: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::from_path(".env")?;
    initialize_logger()?;

    let args = Args::parse();

    let db_connection = DbConnection::new().await.map_err(|e| {
        error!(error = %e, "Failed to create DB connection");
        e
    })?;

    let block_headers = db_connection
        .get_block_headers_by_block_range(args.start_block, args.end_block)
        .await?;
    println!("block_headers 0: {:?}", block_headers[0]);

    let headers_by_hour = group_headers_by_hour(block_headers);

    let mut combined_input: CombinedInput = Default::default();
    combined_input.headers = headers_by_hour;
    combined_input.chain_id = 11155111;
    combined_input.batch_size = args.end_block - args.start_block + 1;

    // Execute the guest code.
    let env = ExecutorEnv::builder().write(&combined_input)?.build()?;
    let exec = default_executor();
    exec.execute(env, MMR_BENCHMARK_ELF)?;

    Ok(())
}
