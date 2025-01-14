use crate::core::{MMRStateManager, ProofGenerator};
use crate::db::DbConnection;
use crate::errors::AccumulatorError;
use crate::utils::BatchResult;
use common::get_or_create_db_path;
use guest_types::{CombinedInput, GuestOutput, MMRInput};
use ipfs_utils::IpfsManager;
use mmr::PeaksOptions;
use mmr_utils::initialize_mmr;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

pub struct BatchProcessor<'a> {
    batch_size: u64,
    proof_generator: ProofGenerator<CombinedInput>,
    mmr_state_manager: MMRStateManager<'a>,
    skip_proof_verification: bool,
    ipfs_manager: IpfsManager,
}

impl<'a> BatchProcessor<'a> {
    pub fn new(
        batch_size: u64,
        proof_generator: ProofGenerator<CombinedInput>,
        skip_proof_verification: bool,
        mmr_state_manager: MMRStateManager<'a>,
    ) -> Result<Self, AccumulatorError> {
        if batch_size == 0 {
            return Err(AccumulatorError::InvalidInput(
                "Batch size must be greater than 0",
            ));
        }

        let ipfs_manager = IpfsManager::new();

        Ok(Self {
            batch_size,
            proof_generator,
            skip_proof_verification,
            mmr_state_manager,
            ipfs_manager,
        })
    }

    pub fn mmr_state_manager(&self) -> &MMRStateManager<'a> {
        &self.mmr_state_manager
    }

    pub fn proof_generator(&self) -> &ProofGenerator<CombinedInput> {
        &self.proof_generator
    }

    pub fn batch_size(&self) -> u64 {
        self.batch_size
    }

    pub fn skip_proof_verification(&self) -> bool {
        self.skip_proof_verification
    }

    pub async fn process_batch(
        &self,
        chain_id: u64,
        start_block: u64,
        end_block: u64,
    ) -> Result<Option<BatchResult>, AccumulatorError> {
        if end_block < start_block {
            return Err(AccumulatorError::InvalidInput(
                "End block cannot be less than start block",
            ));
        }

        let batch_index = start_block / self.batch_size;
        let (batch_start, batch_end) = self.calculate_batch_bounds(batch_index)?;

        if start_block < batch_start {
            return Err(AccumulatorError::InvalidInput(
                "Start block is before batch start",
            ));
        }

        let adjusted_end_block = std::cmp::min(end_block, batch_end);

        info!(
            batch_index,
            num_blocks = adjusted_end_block - start_block + 1,
            start_block,
            end_block,
            "Processing batch"
        );

        let batch_file_name = format!("batch_{}.db", batch_index);
        let temp_file_path =
            PathBuf::from(get_or_create_db_path(&batch_file_name).map_err(|e| {
                error!(error = %e, "Failed to get or create DB path");
                e
            })?);
        debug!("Using temporary batch file: {}", temp_file_path.display());

        let (store_manager, mut mmr, pool) = initialize_mmr(temp_file_path.to_str().unwrap())
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to initialize MMR");
                e
            })?;

        let current_leaves_count = mmr.leaves_count.get().await.map_err(|e| {
            error!(error = %e, "Failed to get current leaves count");
            e
        })?;
        if current_leaves_count as u64 >= self.batch_size {
            debug!("Batch {} is already complete", batch_index);
            return Ok(None);
        }

        let db_connection = DbConnection::new().await.map_err(|e| {
            error!(error = %e, "Failed to create DB connection");
            e
        })?;
        let headers = db_connection
            .get_block_headers_by_block_range(start_block, adjusted_end_block)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch block headers");
                e
            })?;

        if headers.is_empty() {
            warn!(
                "No headers found for block range {} to {}",
                start_block, adjusted_end_block
            );
            return Err(AccumulatorError::EmptyHeaders {
                start_block,
                end_block: adjusted_end_block,
            });
        }

        let current_peaks = mmr.get_peaks(PeaksOptions::default()).await.map_err(|e| {
            error!(error = %e, "Failed to get current peaks");
            e
        })?;
        let current_elements_count = mmr.elements_count.get().await.map_err(|e| {
            error!(error = %e, "Failed to get current elements count");
            e
        })?;
        let current_leaves_count = mmr.leaves_count.get().await.map_err(|e| {
            error!(error = %e, "Failed to get current leaves count");
            e
        })?;

        let new_headers: Vec<String> = headers.iter().map(|h| h.block_hash.clone()).collect();

        let mmr_input = MMRInput::new(
            current_peaks,
            current_elements_count,
            current_leaves_count,
            new_headers.clone(),
        );

        let batch_link: Option<String> = if batch_index > 0 {
            Some(
                db_connection
                    .get_block_header_by_number(start_block - 1)
                    .await?
                    .ok_or_else(|| {
                        AccumulatorError::InvalidInput("Previous block header not found")
                    })?
                    .block_hash,
            )
        } else {
            None
        };

        let next_batch_link = db_connection
            .get_block_header_by_number(adjusted_end_block + 1)
            .await?
            .map(|header| header.parent_hash)
            .flatten();

        let combined_input = CombinedInput::new(
            chain_id,
            self.batch_size,
            headers.clone(),
            mmr_input,
            batch_link,
            next_batch_link,
            self.skip_proof_verification,
        );

        let (guest_output, proof) = if self.skip_proof_verification {
            info!("Skipping proof generation and verification");
            (None, None)
        } else {
            let proof = self
                .proof_generator
                .generate_groth16_proof(combined_input)
                .await
                .map_err(|e| {
                    error!(error = %e, "Failed to generate proof");
                    e
                })?;

            debug!("Generated proof with {} elements", proof.calldata().len());

            let guest_output: GuestOutput =
                self.proof_generator.decode_journal(&proof).map_err(|e| {
                    error!(error = %e, "Failed to decode guest output");
                    e
                })?;

            debug!(
                "Guest output - root_hash: {}, leaves_count: {}",
                guest_output.root_hash(),
                guest_output.leaves_count()
            );

            (Some(guest_output), Some(proof))
        };

        let new_mmr_state = self
            .mmr_state_manager
            .update_state(
                store_manager,
                &mut mmr,
                &pool,
                adjusted_end_block,
                guest_output.as_ref(),
                &new_headers,
            )
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to update MMR state");
                e
            })?;

        // Save to permanent path directly instead of using a temp file
        let permanent_path =
            PathBuf::from(get_or_create_db_path(&batch_file_name).map_err(|e| {
                error!(error = %e, "Failed to get DB path");
                e
            })?);

        // Upload current state to IPFS
        let ipfs_hash = self
            .ipfs_manager
            .upload_db(&permanent_path)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to upload batch file to IPFS");
                AccumulatorError::StorageError(format!("Failed to upload to IPFS: {}", e))
            })?;

        Ok(Some(BatchResult::new(
            start_block,
            adjusted_end_block,
            new_mmr_state,
            proof,
            ipfs_hash.to_string(),
        )))
    }

    pub fn calculate_batch_bounds(&self, batch_index: u64) -> Result<(u64, u64), AccumulatorError> {
        let batch_start = batch_index
            .checked_mul(self.batch_size)
            .ok_or(AccumulatorError::InvalidInput("Batch index too large"))?;

        let batch_end = batch_start
            .checked_add(self.batch_size)
            .ok_or(AccumulatorError::InvalidInput(
                "Batch end calculation overflow",
            ))?
            .saturating_sub(1);

        Ok((batch_start, batch_end))
    }

    pub fn calculate_start_block(&self, current_end: u64) -> Result<u64, AccumulatorError> {
        if current_end == 0 {
            return Err(AccumulatorError::InvalidInput(
                "Current end block cannot be 0",
            ));
        }

        Ok(current_end.saturating_sub(current_end % self.batch_size))
    }

    pub fn calculate_batch_range(
        &self,
        current_end: u64,
        start_block: u64,
    ) -> Result<BatchRange, AccumulatorError> {
        if current_end < start_block {
            return Err(AccumulatorError::InvalidInput(
                "Current end block cannot be less than start block",
            ));
        }

        if current_end == 0 {
            return Err(AccumulatorError::InvalidInput(
                "Current end block cannot be 0",
            ));
        }

        let batch_start = current_end.saturating_sub(current_end % self.batch_size);
        let effective_start = batch_start.max(start_block);

        let batch_size_minus_one = self
            .batch_size
            .checked_sub(1)
            .ok_or(AccumulatorError::InvalidInput("Invalid batch size"))?;

        let max_end =
            batch_start
                .checked_add(batch_size_minus_one)
                .ok_or(AccumulatorError::InvalidInput(
                    "Batch end calculation overflow",
                ))?;

        let effective_end = std::cmp::min(current_end, max_end);

        Ok(BatchRange {
            start: effective_start,
            end: effective_end,
        })
    }
}

pub struct BatchRange {
    pub start: u64,
    pub end: u64,
}

impl BatchRange {
    pub fn new(start: u64, end: u64) -> Result<Self, AccumulatorError> {
        if end < start {
            return Err(AccumulatorError::InvalidInput(
                "End block cannot be less than start block",
            ));
        }
        Ok(Self { start, end })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::automock;
    use serde::Serialize;
    use starknet::{
        core::types::U256,
        providers::{jsonrpc::HttpTransport, JsonRpcClient, Url},
    };
    use starknet_handler::account::StarknetAccount;
    use starknet_handler::MmrState;
    use std::sync::Arc;

    // Create traits that match the structs we want to mock
    #[automock]
    #[allow(dead_code)]
    pub trait ProofGeneratorTrait {
        fn generate_groth16_proof(
            &self,
            input: CombinedInput,
        ) -> Result<mmr::Proof, AccumulatorError>;
        fn decode_journal(&self, proof: &mmr::Proof) -> Result<GuestOutput, AccumulatorError>;
    }

    // Mock implementation that doesn't need real ELF data
    #[allow(dead_code)]
    struct MockProofGen;
    impl<T: Serialize + Clone + Send + 'static> ProofGenerator<T> {
        fn mock() -> Self {
            // Use a static array instead of vec for 'static lifetime
            let method_elf: &'static [u8] = &[1, 2, 3, 4]; // Non-empty ELF data
            let method_id = [1u32; 8]; // Non-zero method ID

            ProofGenerator::new(method_elf, method_id)
                .expect("Failed to create mock ProofGenerator")
        }
    }

    // Create a trait without lifetime parameter for automock
    #[automock]
    #[allow(dead_code)]
    pub trait MMRStateManagerTrait {
        fn update_state<'a>(
            &self,
            store_manager: mmr_utils::StoreManager,
            mmr: &mut mmr::MMR,
            pool: &sqlx::Pool<sqlx::Sqlite>,
            end_block: u64,
            guest_output: Option<&'a GuestOutput>,
            new_headers: &Vec<String>,
        ) -> Result<MmrState, AccumulatorError>;
    }

    // Mock implementation that doesn't need real Starknet connection
    impl<'a> MMRStateManager<'a> {
        fn mock() -> Self {
            let provider = Arc::new(JsonRpcClient::new(HttpTransport::new(
                Url::parse("http://localhost:5050").expect("Invalid URL"),
            )));
            let account = StarknetAccount::new(
                provider, "0x0", "0x0", // private key as &str
            )
            .expect("Failed to create StarknetAccount");

            MMRStateManager::new(
                account, "0x0", // store_address as &str
            )
        }
    }

    // Helper function to create test instances
    fn create_test_processor() -> BatchProcessor<'static> {
        let proof_gen = ProofGenerator::mock();
        let mmr_state_mgr = MMRStateManager::mock();

        BatchProcessor::new(100, proof_gen, false, mmr_state_mgr).unwrap()
    }

    #[tokio::test]
    async fn test_calculate_batch_bounds() {
        let processor = create_test_processor();

        // Test normal case
        let (start, end) = processor.calculate_batch_bounds(1).unwrap();
        assert_eq!(start, 100);
        assert_eq!(end, 199);

        // Test batch 0
        let (start, end) = processor.calculate_batch_bounds(0).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 99);
    }

    #[tokio::test]
    async fn test_calculate_batch_range() {
        let processor = create_test_processor();

        // Test normal case
        let range = processor.calculate_batch_range(150, 100).unwrap();
        assert_eq!(range.start, 100);
        assert_eq!(range.end, 150);

        // Test when current_end is at batch boundary
        let range = processor.calculate_batch_range(200, 150).unwrap();
        assert_eq!(range.start, 200);
        assert_eq!(range.end, 200);

        // Test error case: current_end < start_block
        let result = processor.calculate_batch_range(100, 150);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_calculate_start_block() {
        let processor = create_test_processor();

        // Test normal case
        let start = processor.calculate_start_block(150).unwrap();
        assert_eq!(start, 100);

        // Test at batch boundary
        let start = processor.calculate_start_block(200).unwrap();
        assert_eq!(start, 200);

        // Test error case: current_end = 0
        let result = processor.calculate_start_block(0);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_processor_new() {
        let proof_gen = ProofGenerator::mock();
        let mmr_state_mgr = MMRStateManager::mock();

        // Test valid creation
        let result = BatchProcessor::new(100, proof_gen, false, mmr_state_mgr);
        assert!(result.is_ok());

        // Test invalid batch size
        let result = BatchProcessor::new(0, ProofGenerator::mock(), false, MMRStateManager::mock());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_range_new() {
        // Test valid range
        let result = BatchRange::new(100, 200);
        assert!(result.is_ok());
        let range = result.unwrap();
        assert_eq!(range.start, 100);
        assert_eq!(range.end, 200);

        // Test invalid range
        let result = BatchRange::new(200, 100);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_process_batch_invalid_inputs() {
        let processor = create_test_processor();

        // Test end_block < start_block
        let result = processor.process_batch(1, 150, 100).await;
        assert!(result.is_err());

        // Test start_block before batch start
        let result = processor.process_batch(1, 50, 199).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_getters() {
        let processor = create_test_processor();

        assert_eq!(processor.batch_size(), 100);
        assert!(!processor.skip_proof_verification());
    }

    // Add test that uses the mock traits to satisfy dead code warnings
    #[test]
    fn test_mock_traits() {
        let mut mock_proof_gen = MockProofGeneratorTrait::new();
        let mut mock_mmr_mgr = MockMMRStateManagerTrait::new();

        // Set up expectations
        mock_proof_gen
            .expect_generate_groth16_proof()
            .returning(|_| {
                Ok(mmr::Proof {
                    element_index: 0,
                    element_hash: "".to_string(),
                    siblings_hashes: vec!["".to_string()],
                    peaks_hashes: vec!["".to_string()],
                    elements_count: 0,
                })
            });

        mock_mmr_mgr
            .expect_update_state()
            .returning(|_, _, _, _, _, _| {
                Ok(MmrState::new(
                    0,
                    U256::from(0_u64),
                    U256::from(0_u64),
                    0,
                    None,
                ))
            });

        // Verify mocks exist
        mock_proof_gen.checkpoint();
        mock_mmr_mgr.checkpoint();
    }
}
