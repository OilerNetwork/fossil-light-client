use axum::{
    extract::Query,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_server::Server;
use clap::Parser;
use common::get_env_var;
use publisher::api::operations::prove_headers_integrity_and_inclusion;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Environment file to use (.env.local, .env.docker, etc)
    #[arg(short, long, default_value = ".env.local")]
    env_file: String,

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    // Get required environment variables
    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let l2_store_address = get_env_var("FOSSIL_STORE")?;
    let chain_id = get_env_var("CHAIN_ID")?
        .parse::<u64>()
        .expect("CHAIN_ID must be a valid number");

    let app = Router::new()
        .route("/verify-blocks", get(verify_blocks))
        .with_state((
            rpc_url,
            l2_store_address,
            chain_id,
            args.skip_proof_verification,
        ));

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
    axum::extract::State((rpc_url, l2_store_address, chain_id, default_skip_proof)): axum::extract::State<(String, String, u64, bool)>,
    Query(params): Query<BlockRangeParams>,
) -> Response {
    // Use query parameter if provided, otherwise use CLI default
    let skip_proof = params.skip_proof_verification.unwrap_or(default_skip_proof);

    match prove_headers_integrity_and_inclusion(
        &rpc_url,
        &l2_store_address,
        chain_id,
        params.from_block,
        params.to_block,
        Some(skip_proof),
    )
    .await
    {
        Ok(result) => {
            let bytes = bincode::serialize(&result).unwrap();
            (
                [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
                bytes,
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Error verifying blocks: {}", e);
            (
                [(axum::http::header::CONTENT_TYPE, "text/plain")],
                format!("error: {}", e),
            )
                .into_response()
        }
    }
}
