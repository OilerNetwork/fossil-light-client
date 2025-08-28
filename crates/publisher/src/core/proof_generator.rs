use garaga_rs::{
    calldata::full_proof_with_hints::groth16::{
        get_groth16_calldata_felt, risc0_utils::get_risc0_vk, Groth16Proof,
    },
    definitions::CurveID,
};
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{compute_image_id, default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use serde::Deserialize;
use starknet_crypto::Felt;
use tokio::{
    task,
    time::{sleep, Duration},
};
use tracing::{debug, error, info};

use crate::{
    error::{PublisherError, PublisherResult},
    utils::{Groth16, Stark},
};

const MAX_RETRIES: u32 = 10;
const INITIAL_RETRY_DELAY_MS: u64 = 5000;

#[derive(Debug)]
/// Generates zero-knowledge proofs for MMR operations using RISC Zero
pub struct ProofGenerator<T> {
    method_elf: &'static [u8],
    method_id: [u32; 8],
    _phantom: std::marker::PhantomData<T>,
}

// Explicitly implement Send and Sync since all fields are Send + Sync
unsafe impl<T> Send for ProofGenerator<T> where T: Send {}
unsafe impl<T> Sync for ProofGenerator<T> where T: Sync {}

impl<T> ProofGenerator<T>
where
    T: serde::Serialize + Clone + Send + Sync + 'static + std::fmt::Debug,
{
    /// Create a new proof generator with method ELF and ID
    pub fn new(method_elf: &'static [u8], method_id: [u32; 8]) -> PublisherResult<Self> {
        if method_elf.is_empty() {
            error!("Method ELF cannot be empty");
            return Err(PublisherError::proof_generation(format!(
                "Method ELF cannot be empty: {method_elf:?}"
            )));
        }

        if method_id.iter().all(|&x| x == 0) {
            error!(method_id = ?method_id, "Method ID cannot be all zeros");
            return Err(PublisherError::proof_generation(format!(
                "Method ID cannot be all zeros: {method_id:?}"
            )));
        }

        // Warn about very small ELF files that are likely mock data
        if method_elf.len() < 1024 {
            tracing::warn!(
                "Method ELF is very small ({} bytes), likely mock data for testing",
                method_elf.len()
            );
        }

        Ok(Self {
            method_elf,
            method_id,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Validate input data before proof generation
    fn validate_input(input: &T) -> PublisherResult<()> {
        let input_size = std::mem::size_of_val(input);
        let debug_str = format!("{input:?}");
        let type_name = std::any::type_name::<T>();

        Self::validate_memory_size(input_size)?;
        Self::validate_debug_content(&debug_str)?;
        Self::validate_large_collections(input_size, &debug_str, type_name)?;

        debug!(
            "Input validation passed for type {}: {} bytes, content: {}",
            type_name, input_size, debug_str
        );
        Ok(())
    }

    /// Validates that input has non-zero memory size
    fn validate_memory_size(input_size: usize) -> PublisherResult<()> {
        if input_size == 0 {
            error!("Input has zero memory size");
            return Err(PublisherError::proof_generation(
                "Input cannot have zero memory size",
            ));
        }
        Ok(())
    }

    /// Validates debug string content for empty patterns and minimal content
    fn validate_debug_content(debug_str: &str) -> PublisherResult<()> {
        if Self::is_empty_pattern(debug_str) {
            error!("Input appears to be empty: {}", debug_str.trim());
            return Err(PublisherError::proof_generation(format!(
                "Input appears to be empty: {}",
                debug_str.trim()
            )));
        }

        if Self::has_minimal_content(debug_str) {
            error!("Input appears to have minimal content: {}", debug_str);
            return Err(PublisherError::proof_generation(format!(
                "Input appears to have minimal content: {debug_str}"
            )));
        }

        Ok(())
    }

    /// Checks for common empty patterns in debug output
    fn is_empty_pattern(debug_str: &str) -> bool {
        let trimmed = debug_str.trim();
        trimmed == "[]" || trimmed == "{}" || trimmed == "()"
    }

    /// Checks if debug string indicates minimal content
    const fn has_minimal_content(debug_str: &str) -> bool {
        debug_str.len() <= 4
    }

    /// Validates large collections to ensure they have meaningful content
    fn validate_large_collections(
        input_size: usize,
        debug_str: &str,
        type_name: &str,
    ) -> PublisherResult<()> {
        // For types with larger memory footprint, ensure they have meaningful size
        // Vec<u8> with empty content still has Vec metadata (24 bytes on 64-bit)
        if input_size >= 24 && debug_str.trim() == "[]" {
            error!(
                "Large empty collection detected for type {}: {}",
                type_name, debug_str
            );
            return Err(PublisherError::proof_generation(format!(
                "Large empty collection detected for type {type_name}"
            )));
        }
        Ok(())
    }

    /// Generate a standard Stark proof for intermediate batches
    pub async fn generate_stark_proof(&self, input: T) -> PublisherResult<Stark> {
        // Validate input before attempting proof generation
        Self::validate_input(&input)?;

        // Additional validation for mock ELF data in tests
        if self.method_elf.len() < 1024 {
            error!(
                "Method ELF too small for real proof generation: {} bytes",
                self.method_elf.len()
            );
            return Err(PublisherError::proof_generation(format!(
                "Method ELF too small for proof generation: {} bytes (minimum 1024 bytes required)",
                self.method_elf.len()
            )));
        }

        let input_size = std::mem::size_of_val(&input);
        info!("Generating STARK proof...");
        debug!("Input size: {} bytes", input_size);

        let proof = task::spawn_blocking({
            let method_elf = self.method_elf;
            let method_id = self.method_id;
            let input = input.clone();

            #[allow(clippy::cognitive_complexity)]
            move || -> PublisherResult<Stark> {
                debug!("Building executor environment");
                let env = ExecutorEnv::builder()
                    .write(&input)
                    .map_err(|e| {
                        PublisherError::proof_generation(format!(
                            "Failed to write input to executor env: {e}"
                        ))
                    })?
                    .build()
                    .map_err(|e| {
                        PublisherError::proof_generation(format!(
                            "Failed to build executor env: {e}"
                        ))
                    })?;

                debug!("Generating STARK proof with default prover");
                let receipt = default_prover()
                    .prove(env, method_elf)
                    .map_err(|e| {
                        PublisherError::proof_generation(format!(
                            "Failed to generate STARK proof: {e}"
                        ))
                    })?
                    .receipt;

                debug!("Computing image ID");
                let image_id = compute_image_id(method_elf).map_err(|e| {
                    PublisherError::proof_generation(format!("Failed to compute image ID: {e}"))
                })?;

                info!("Successfully generated STARK proof");
                Ok(Stark::new(receipt, image_id.as_bytes().to_vec(), method_id))
            }
        })
        .await?
        .map_err(|e| {
            PublisherError::proof_generation(format!("Failed to spawn blocking task: {e}"))
        })?;

        Ok(proof)
    }

    /// Generate a Groth16 proof for the final batch
    pub async fn generate_groth16_proof(&self, input: T) -> PublisherResult<Groth16> {
        self.generate_groth16_proof_with_retry(input).await
    }

    /// Decode the journal data from a Groth16 proof
    pub fn decode_journal<U: for<'a> Deserialize<'a>>(
        &self,
        proof: &Groth16,
    ) -> PublisherResult<U> {
        if proof.receipt().journal.bytes.is_empty() {
            error!("Proof journal cannot be empty for decoding");
            return Err(PublisherError::proof_generation(format!(
                "Proof journal cannot be empty: {:?}",
                proof.receipt().journal.bytes
            )));
        }

        let receipt = proof.receipt();
        Ok(receipt.journal.decode()?)
    }

    async fn generate_groth16_proof_with_retry(&self, input: T) -> PublisherResult<Groth16> {
        let mut retries = 0;
        let mut last_error = None;

        while retries < MAX_RETRIES {
            match self.generate_groth16_proof_internal(input.clone()).await {
                Ok(proof) => return Ok(proof),
                Err(e) => {
                    retries += 1;

                    if retries < MAX_RETRIES {
                        let delay = INITIAL_RETRY_DELAY_MS * (2_u64.pow(retries - 1));
                        tracing::warn!(
                            "Failed to generate Groth16 proof: {}, retrying in {}ms (attempt {}/{})",
                            e,
                            delay,
                            retries,
                            MAX_RETRIES
                        );
                        sleep(Duration::from_millis(delay)).await;
                    }

                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            PublisherError::proof_generation(format!(
                "Failed to generate Groth16 proof after {MAX_RETRIES} attempts"
            ))
        }))
    }

    async fn generate_groth16_proof_internal(&self, input: T) -> PublisherResult<Groth16> {
        // Validate input before attempting proof generation
        Self::validate_input(&input)?;

        // Additional validation for mock ELF data in tests
        if self.method_elf.len() < 1024 {
            error!(
                "Method ELF too small for real proof generation: {} bytes",
                self.method_elf.len()
            );
            return Err(PublisherError::proof_generation(format!(
                "Method ELF too small for proof generation: {} bytes (minimum 1024 bytes required)",
                self.method_elf.len()
            )));
        }

        let input_size = std::mem::size_of_val(&input);
        debug!("Input size: {} bytes", input_size);

        let method_elf = self.method_elf;
        let input = input.clone();

        let proof = task::spawn_blocking(move || -> PublisherResult<Groth16> {
            Self::generate_groth16_proof_internal_blocking(input, method_elf)
        })
        .await?
        .map_err(|e| {
            PublisherError::proof_generation(format!("Failed to spawn blocking task: {e}"))
        })?;

        Ok(proof)
    }

    fn generate_groth16_proof_internal_blocking(
        input: T,
        method_elf: &[u8],
    ) -> PublisherResult<Groth16> {
        let env = Self::build_executor_environment(&input)?;
        let receipt = Self::generate_proof_receipt(env, method_elf)?;
        let proof_components = Self::process_proof_receipt(&receipt, method_elf)?;
        let calldata = Self::generate_proof_calldata(&proof_components)?;

        debug!("Successfully generated Groth16 proof and calldata.");
        Ok(Groth16::new(receipt, calldata))
    }

    fn build_executor_environment(input: &T) -> PublisherResult<ExecutorEnv<'static>> {
        debug!("Building executor environment");
        ExecutorEnv::builder()
            .write(input)
            .map_err(|e| {
                PublisherError::proof_generation(format!(
                    "Failed to write input to executor env: {e}"
                ))
            })?
            .build()
            .map_err(|e| {
                PublisherError::proof_generation(format!("Failed to build executor env: {e}"))
            })
    }

    fn generate_proof_receipt(
        env: ExecutorEnv<'_>,
        method_elf: &[u8],
    ) -> PublisherResult<risc0_zkvm::Receipt> {
        debug!("Generating proof with Groth16 options");
        default_prover()
            .prove_with_ctx(
                env,
                &VerifierContext::default(),
                method_elf,
                &ProverOpts::groth16(),
            )
            .map_err(|e| {
                PublisherError::proof_generation(format!("Failed to generate Groth16 proof: {e}"))
            })
            .map(|prove_info| prove_info.receipt)
    }

    fn process_proof_receipt(
        receipt: &risc0_zkvm::Receipt,
        method_elf: &[u8],
    ) -> PublisherResult<Groth16Proof> {
        debug!("Encoding seal");
        let encoded_seal = encode_seal(receipt)
            .map_err(|e| PublisherError::proof_generation(format!("Failed to encode seal: {e}")))?;

        debug!("Computing image ID");
        let image_id = compute_image_id(method_elf).map_err(|e| {
            PublisherError::proof_generation(format!("Failed to compute image ID: {e}"))
        })?;

        let journal = receipt.journal.bytes.clone();

        debug!("Converting to Groth16 proof");
        Ok(Groth16Proof::from_risc0(
            encoded_seal,
            image_id.as_bytes().to_vec(),
            journal,
        ))
    }

    fn generate_proof_calldata(groth16_proof: &Groth16Proof) -> PublisherResult<Vec<Felt>> {
        debug!("Generating calldata");
        get_groth16_calldata_felt(groth16_proof, &get_risc0_vk(), CurveID::BN254).map_err(|e| {
            PublisherError::proof_generation(format!("Failed to generate calldata: {e}"))
        })
    }

    #[cfg(test)]
    /// Creates a mock proof generator for testing purposes
    pub fn mock_for_tests() -> Self {
        Self {
            method_elf: &[],
            method_id: [0; 8],
            _phantom: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    // Mock data structure for testing
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestInput {
        value: u32,
    }

    const TEST_METHOD_ELF: &[u8] = &[1, 2, 3, 4]; // Mock ELF data
    const TEST_METHOD_ID: [u32; 8] = [1, 0, 0, 0, 0, 0, 0, 0];

    #[test]
    fn test_debug_format() {
        let empty_vec: Vec<u8> = vec![];
        println!("Empty vec debug: '{:?}'", empty_vec);
        println!(
            "Empty vec debug trimmed: '{}'",
            format!("{:?}", empty_vec).trim()
        );
        println!("Empty vec size: {}", std::mem::size_of_val(&empty_vec));
    }

    #[test]
    fn test_validate_input() {
        let empty_vec: Vec<u8> = vec![];
        let result = ProofGenerator::<Vec<u8>>::validate_input(&empty_vec);
        println!("Validation result: {:?}", result);
        assert!(result.is_err(), "Empty vec should fail validation");
    }

    #[test]
    fn test_new_proof_generator() {
        // Test successful creation
        let result = ProofGenerator::<TestInput>::new(TEST_METHOD_ELF, TEST_METHOD_ID);
        assert!(result.is_ok());

        // Test empty ELF
        let result = ProofGenerator::<TestInput>::new(&[], TEST_METHOD_ID);
        assert!(matches!(
            result.unwrap_err(), e if e.to_string().contains("Method ELF cannot be empty")
        ));

        // Test zero method ID
        let result = ProofGenerator::<TestInput>::new(TEST_METHOD_ELF, [0; 8]);
        assert!(matches!(
            result.unwrap_err(), e if e.to_string().contains("Method ID cannot be all zeros")
        ));
    }

    #[tokio::test]
    async fn test_generate_stark_proof_invalid_input() {
        let proof_generator =
            ProofGenerator::<Vec<u8>>::new(TEST_METHOD_ELF, TEST_METHOD_ID).unwrap();
        let result = proof_generator.generate_stark_proof(vec![]).await;
        assert!(result.is_err());
    }

    // Note: Testing the actual proof generation would require mock implementations
    // of the RISC Zero prover and related components. Here's a sketch of how that
    // might look with proper mocking:

    // #[tokio::test]
    // async fn test_generate_stark_proof_success() {
    // Would need to mock:
    // - ExecutorEnv
    // - default_prover
    // - compute_image_id
    //
    // let generator = ProofGenerator::<TestInput>::new(TEST_METHOD_ELF, TEST_METHOD_ID).unwrap();
    // let input = TestInput { value: 42 };
    // let result = generator.generate_stark_proof(input).await;
    // assert!(result.is_ok());
    // }
    //
    // #[tokio::test]
    // async fn test_generate_groth16_proof_success() {
    // Would need to mock:
    // - ExecutorEnv
    // - default_prover
    // - compute_image_id
    // - encode_seal
    // - Groth16Proof conversion
    // - get_groth16_calldata_felt
    //
    // let generator = ProofGenerator::<TestInput>::new(TEST_METHOD_ELF, TEST_METHOD_ID).unwrap();
    // let input = TestInput { value: 42 };
    // let result = generator.generate_groth16_proof(input).await;
    // assert!(result.is_ok());
    // }

    #[test]
    fn test_decode_journal() {
        // Would need mock Groth16 proof with valid journal data
        // This test would verify that journal decoding works correctly
        // and handles errors appropriately
    }
}
