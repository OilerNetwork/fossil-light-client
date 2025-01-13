use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use state_proof_api::*; // You'll need to make necessary items public in lib.rs
use std::sync::Arc;
use tower::ServiceExt;

// Mock state with a very short timeout
fn create_test_state() -> Arc<AppState> {
    Arc::new(AppState {
        rpc_url: "http://localhost:1234".to_string(),
        l2_store_address: "0x123".to_string(),
        chain_id: 1,
        skip_proof_verification: false,
        batch_size: 1024,
    })
}

#[tokio::test]
async fn test_verify_blocks_endpoint() {
    let app = Router::new()
        .route("/verify-blocks", get(verify_blocks))
        .with_state(create_test_state());

    // Test cases with their expected status codes
    let test_cases = vec![
        // Valid cases - will fail fast due to connection refused
        (
            "/verify-blocks?from_block=1&to_block=10",
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        // Missing parameter cases
        ("/verify-blocks", StatusCode::BAD_REQUEST),
        ("/verify-blocks?from_block=1", StatusCode::BAD_REQUEST),
        ("/verify-blocks?to_block=10", StatusCode::BAD_REQUEST),
        // Invalid parameter cases
        (
            "/verify-blocks?from_block=abc&to_block=10",
            StatusCode::BAD_REQUEST,
        ),
        (
            "/verify-blocks?from_block=1&to_block=def",
            StatusCode::BAD_REQUEST,
        ),
    ];

    for (uri, expected_status) in test_cases {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            expected_status,
            "URI '{}' should return status {}",
            uri,
            expected_status
        );
    }
}

#[tokio::test]
async fn test_skip_proof_verification_combinations() {
    let app = Router::new()
        .route("/verify-blocks", get(verify_blocks))
        .with_state(create_test_state());

    let test_cases = vec![
        // Basic override cases - will fail fast due to connection refused
        (
            "/verify-blocks?from_block=1&to_block=10&skip_proof_verification=true",
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            "/verify-blocks?from_block=1&to_block=10&skip_proof_verification=false",
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        // Invalid skip_proof_verification values
        (
            "/verify-blocks?from_block=1&to_block=10&skip_proof_verification=invalid",
            StatusCode::BAD_REQUEST,
        ),
        (
            "/verify-blocks?from_block=1&to_block=10&skip_proof_verification=",
            StatusCode::BAD_REQUEST,
        ),
        (
            "/verify-blocks?from_block=1&to_block=10&skip_proof_verification",
            StatusCode::BAD_REQUEST,
        ),
    ];

    for (uri, expected_status) in test_cases {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            expected_status,
            "URI '{}' should return status {}",
            uri,
            expected_status
        );
    }
}

#[tokio::test]
async fn test_batch_size_boundaries() {
    let test_cases = vec![
        (0, false),    // Invalid batch size
        (1, true),     // Minimum valid batch size
        (1024, true),  // Default batch size
        (10000, true), // Large batch size
    ];

    for (batch_size, _) in test_cases {
        let state = Arc::new(AppState {
            rpc_url: "http://localhost:1234".to_string(),
            l2_store_address: "0x123".to_string(),
            chain_id: 1,
            skip_proof_verification: false,
            batch_size,
        });

        let app = Router::new()
            .route("/verify-blocks", get(verify_blocks))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/verify-blocks?from_block=1&to_block=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "Batch size {} test",
            batch_size
        );
    }
}

#[test]
fn test_app_state_creation() {
    let state = AppState {
        rpc_url: "http://localhost:8545".to_string(),
        l2_store_address: "0x123".to_string(),
        chain_id: 1,
        skip_proof_verification: false,
        batch_size: 1024,
    };

    assert_eq!(state.rpc_url, "http://localhost:8545");
    assert_eq!(state.l2_store_address, "0x123");
    assert_eq!(state.chain_id, 1);
    assert_eq!(state.skip_proof_verification, false);
    assert_eq!(state.batch_size, 1024);
}
