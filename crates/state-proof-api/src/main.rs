use axum::{
    extract::Query,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_server::Server;
use publisher::api::operations::prove_headers_integrity_and_inclusion;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize)]
struct BlockRangeParams {
    from_block: u64,
    to_block: u64,
    rpc_url: String,
    l2_store_address: String,
    chain_id: u64,
    skip_proof_verification: Option<bool>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/verify-blocks", get(verify_blocks));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn verify_blocks(Query(params): Query<BlockRangeParams>) -> Response {
    match prove_headers_integrity_and_inclusion(
        &params.rpc_url,
        &params.l2_store_address,
        params.chain_id,
        params.from_block,
        params.to_block,
        params.skip_proof_verification,
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
