#![deny(unused_crate_dependencies)]
use methods as _;
use reqwest as _;
use risc0_zkvm as _;

use axum::body::Body;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_server::Server;
use clap::Parser;
use common::{get_env_var, initialize_logger_and_env};
use publisher::api::operations::extract_fees;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

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

#[derive(Deserialize)]
struct BlockRangeParams {
    from_block: u64,
    to_block: u64,
    /// Optional override for skip_proof_verification from CLI
    skip_proof_verification: Option<bool>,
}

#[derive(Clone)]
struct AppState {
    rpc_url: String,
    l2_store_address: String,
    chain_id: u64,
    skip_proof_verification: bool,
    batch_size: u64,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
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

async fn verify_blocks(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BlockRangeParams>,
) -> impl IntoResponse {
    // Use query parameter if provided, otherwise use CLI default
    let skip_proof = params
        .skip_proof_verification
        .unwrap_or(state.skip_proof_verification);

    let result = match extract_fees(
        &state.rpc_url,
        &state.l2_store_address,
        state.chain_id,
        state.batch_size,
        params.from_block,
        params.to_block,
        Some(skip_proof),
    )
    .await
    {
        Ok(result) => match bincode::serialize(&result) {
            Ok(bytes) => {
                let mut headers = HeaderMap::new();
                headers.insert(
                    header::CONTENT_TYPE,
                    "application/octet-stream".parse().unwrap(),
                );
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/octet-stream")
                    .body(Body::from(bytes))
                    .unwrap()
            }
            Err(e) => {
                tracing::error!("Failed to serialize response: {}", e);
                let error_json = serde_json::to_vec(&ErrorResponse {
                    error: format!("Failed to serialize response: {}", e),
                })
                .unwrap_or_default();
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(error_json.into())
                    .unwrap()
            }
        },
        Err(e) => {
            tracing::error!("Error verifying blocks: {}", e);
            let error_json = serde_json::to_vec(&ErrorResponse {
                error: e.to_string(),
            })
            .unwrap_or_default();
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "application/json")
                .body(error_json.into())
                .unwrap()
        }
    };

    result
}
