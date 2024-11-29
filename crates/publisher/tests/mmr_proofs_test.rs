// use mmr::MMR;
use mmr_utils::initialize_mmr;
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn test_mmr_proofs() {
    // Get path to the test file's directory
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");

    let binding = test_dir.join("0.db");
    let store_path = binding.to_str().unwrap();
    let (store_manager, mmr, pool) = initialize_mmr(store_path).await.unwrap();

    let indexs = vec![1, 2, 4, 5, 8, 9, 11, 12, 16, 17, 19, 20, 23, 24, 26, 27];

    let mut hashes = vec![];
    for index in indexs.iter() {
        let hash = store_manager
            .get_value_for_element_index(&pool, *index)
            .await
            .unwrap();
        hashes.push(hash.unwrap());
    }

    for (i, index) in indexs.iter().enumerate() {
        let proof = mmr.get_proof(*index, None).await.unwrap();
        let result = mmr
            .verify_proof(proof, hashes[i].clone(), None)
            .await
            .unwrap();
        assert!(result);
    }
}
