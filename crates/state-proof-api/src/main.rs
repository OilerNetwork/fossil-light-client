use axum_server::Server;
use clap::Parser;
use common::{get_env_var, initialize_logger_and_env};
use state_proof_api::api::{verify_blocks, AppState};
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::get, Router};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Environment file to use (.env.local, .env.docker, etc)
    #[arg(short, long, default_value = ".env.local")]
    env_file: String,

    /// Batch size to use
    #[arg(short, long, default_value_t = 1024)]
    batch_size: u64,

    /// Skip proof verification by default
    #[arg(long, default_value = "false")]
    skip_proof_verification: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize environment with specified file
    dotenv::from_path(&args.env_file)?;
    initialize_logger_and_env()?;

    // Get required environment variables
    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let l2_store_address = get_env_var("FOSSIL_STORE")?;
    let chain_id = get_env_var("CHAIN_ID")?
        .parse::<u64>()
        .expect("CHAIN_ID must be a valid number");

    let state = AppState {
        rpc_url,
        l2_store_address,
        chain_id,
        skip_proof_verification: args.skip_proof_verification,
        batch_size: args.batch_size,
    };

    let app = Router::new()
        .route("/verify-blocks", get(verify_blocks))
        .with_state(Arc::new(state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    tracing::info!("using environment file: {}", args.env_file);
    tracing::info!(
        "default skip_proof_verification: {}",
        args.skip_proof_verification
    );

    Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
