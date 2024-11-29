// use mmr::MMR;
use mmr_utils::initialize_mmr;
use std::path::PathBuf;

#[tokio::test]
async fn test_mmr_proofs() {
    // Get path to the db-instances directory relative to the test file
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("db-instances");

    let binding = test_dir.join("0.db");
    let store_path = binding.to_str().unwrap();

    let (store_manager, mmr, pool) = initialize_mmr(store_path).await.unwrap();

    let indices = vec![1, 2, 4, 5, 8, 9, 11, 12];

    let mut hashes = vec![];
    for index in indices.iter() {
        let hash = store_manager
            .get_value_for_element_index(&pool, *index)
            .await
            .unwrap();
        hashes.push(hash.unwrap());
    }

    for (i, index) in indices.iter().enumerate() {
        let proof = mmr.get_proof(*index, None).await.unwrap();
        assert!(mmr
            .verify_proof(proof, hashes[i].clone(), None)
            .await
            .unwrap());
    }
}
