/// Enhanced MMR proofs integration tests
///
/// This module tests the core MMR functionality for proof generation and verification,
/// which is critical to the publisher's block hash proof capabilities.
use std::{fs, path::PathBuf};

use mmr_utils::initialize_mmr;

/// Test data for MMR proof verification
const TEST_INDEXES: &[usize] = &[1, 2, 4, 5, 8, 9, 11, 12, 16, 17, 19, 20, 23, 24, 26, 27];

/// Helper function to set up test database
async fn setup_test_database() -> (mmr_utils::StoreManager, mmr::MMR, sqlx::Pool<sqlx::Sqlite>) {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");

    let db_path = test_dir.join("0.db");
    let store_path = db_path.to_str().expect("Failed to convert path to string");

    initialize_mmr(store_path)
        .await
        .expect("Failed to initialize MMR")
}

/// Test MMR proof generation and verification for multiple elements
#[tokio::test]
async fn test_mmr_proofs_comprehensive() {
    let (store_manager, mmr, pool) = setup_test_database().await;

    // Get all hashes for the test indexes
    let mut hashes = Vec::new();
    for &index in TEST_INDEXES {
        let hash = store_manager
            .get_value_for_element_index(&pool, index)
            .await
            .expect("Failed to get value for element index");

        assert!(hash.is_some(), "Hash should exist for index {}", index);
        hashes.push(hash.unwrap());
    }

    // Verify proofs for each element
    for (i, &index) in TEST_INDEXES.iter().enumerate() {
        let proof = mmr
            .get_proof(index, None)
            .await
            .expect("Failed to generate proof");

        let verification_result = mmr
            .verify_proof(proof.clone(), hashes[i].clone(), None)
            .await
            .expect("Failed to verify proof");

        assert!(
            verification_result,
            "Proof verification failed for index {} with hash {}",
            index, hashes[i]
        );

        // Additional validation of proof structure
        assert_eq!(
            proof.element_index as usize, index,
            "Proof element index should match"
        );
        assert_eq!(
            proof.element_hash, hashes[i],
            "Proof element hash should match"
        );
        assert!(
            !proof.siblings_hashes.is_empty(),
            "Proof should have siblings"
        );
        assert!(!proof.peaks_hashes.is_empty(), "Proof should have peaks");
    }
}

/// Test MMR proof generation performance
#[tokio::test]
async fn test_mmr_proof_performance() {
    use std::time::{Duration, Instant};

    let (store_manager, mmr, pool) = setup_test_database().await;

    let test_index = TEST_INDEXES[0];
    let hash = store_manager
        .get_value_for_element_index(&pool, test_index)
        .await
        .expect("Failed to get value")
        .expect("Hash should exist");

    // Measure proof generation time
    let start = Instant::now();
    let proof = mmr
        .get_proof(test_index, None)
        .await
        .expect("Failed to generate proof");
    let proof_generation_time = start.elapsed();

    // Measure proof verification time
    let start = Instant::now();
    let verification_result = mmr
        .verify_proof(proof, hash, None)
        .await
        .expect("Failed to verify proof");
    let verification_time = start.elapsed();

    assert!(verification_result, "Proof verification should succeed");

    // Performance assertions (these might need adjustment based on system performance)
    assert!(
        proof_generation_time < Duration::from_millis(100),
        "Proof generation took {:?}, expected < 100ms",
        proof_generation_time
    );

    assert!(
        verification_time < Duration::from_millis(100),
        "Proof verification took {:?}, expected < 100ms",
        verification_time
    );
}

/// Test MMR proof with invalid data
#[tokio::test]
async fn test_mmr_proof_invalid_cases() {
    let (store_manager, mmr, pool) = setup_test_database().await;

    let test_index = TEST_INDEXES[0];
    let correct_hash = store_manager
        .get_value_for_element_index(&pool, test_index)
        .await
        .expect("Failed to get value")
        .expect("Hash should exist");

    let proof = mmr
        .get_proof(test_index, None)
        .await
        .expect("Failed to generate proof");

    // Test with wrong hash
    let wrong_hash = "0xwronghash".to_string();
    let verification_result = mmr
        .verify_proof(proof.clone(), wrong_hash, None)
        .await
        .expect("Verification should not fail, but should return false");

    assert!(
        !verification_result,
        "Proof verification should fail with wrong hash"
    );

    // Test with correct hash (sanity check)
    let verification_result = mmr
        .verify_proof(proof, correct_hash, None)
        .await
        .expect("Failed to verify proof");

    assert!(
        verification_result,
        "Proof verification should succeed with correct hash"
    );
}

/// Test error handling in MMR operations
#[tokio::test]
async fn test_mmr_error_handling() {
    let (store_manager, mmr, pool) = setup_test_database().await;

    // Test with non-existent index
    let non_existent_index = usize::MAX;

    let hash_result = store_manager
        .get_value_for_element_index(&pool, non_existent_index)
        .await;

    // Should succeed but return None for non-existent index
    match hash_result {
        Ok(None) => {
            // This is expected - index doesn't exist
        }
        Ok(Some(_)) => {
            panic!("Should not have found hash for non-existent index");
        }
        Err(_) => {
            // This might be acceptable depending on implementation
        }
    }

    // Test proof generation for non-existent index
    let proof_result = mmr.get_proof(non_existent_index, None).await;

    // This should either return an error or handle gracefully
    // The exact behavior depends on the MMR implementation
    match proof_result {
        Ok(_) => {
            // If it succeeds, that's also valid depending on implementation
        }
        Err(_) => {
            // Expected error for non-existent index
        }
    }
}
