use garaga_rs::{
    calldata::full_proof_with_hints::groth16::{
        get_groth16_calldata_felt, risc0_utils::get_risc0_vk, Groth16Proof,
    },
    definitions::CurveID,
};
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{compute_image_id, default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use serde::Deserialize;
use tokio::task;
use tokio::time::{sleep, Duration};
use tracing::{debug, info};

use crate::utils::{Groth16, Stark};
use eyre::{eyre, Result};

const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 1000;

#[derive(Debug)]
pub struct ProofGenerator<T> {
    method_elf: &'static [u8],
    method_id: [u32; 8],
    _phantom: std::marker::PhantomData<T>,
}

impl<T> ProofGenerator<T>
where
    T: serde::Serialize + Clone + Send + 'static,
{
    pub fn new(method_elf: &'static [u8], method_id: [u32; 8]) -> Result<Self> {
        if method_elf.is_empty() {
            return Err(eyre!("Method ELF cannot be empty: {:?}", method_elf));
        }

        if method_id.iter().all(|&x| x == 0) {
            return Err(eyre!("Method ID cannot be all zeros: {:?}", method_id));
        }

        Ok(Self {
            method_elf,
            method_id,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Generate a standard Stark proof for intermediate batches
    pub async fn generate_stark_proof(&self, input: T) -> Result<Stark> {
        let input_size = std::mem::size_of_val(&input);
        if input_size == 0 {
            return Err(eyre!("Input cannot be empty"));
        }

        info!("Generating STARK proof for intermediate batch");
        debug!("Input size: {} bytes", input_size);

        let proof = task::spawn_blocking({
            let method_elf = self.method_elf;
            let method_id = self.method_id;
            let input = input.clone();

            move || -> Result<Stark> {
                debug!("Building executor environment");
                let env = ExecutorEnv::builder()
                    .write(&input)
                    .map_err(|e| eyre!("Failed to write input to executor env: {}", e))?
                    .build()
                    .map_err(|e| eyre!("Failed to build executor env: {}", e))?;

                debug!("Generating STARK proof with default prover");
                let receipt = default_prover()
                    .prove(env, method_elf)
                    .map_err(|e| eyre!("Failed to generate STARK proof: {}", e))?
                    .receipt;

                debug!("Computing image ID");
                let image_id = compute_image_id(method_elf)
                    .map_err(|e| eyre!("Failed to compute image ID: {}", e))?;

                info!("Successfully generated STARK proof");
                Ok(Stark::new(receipt, image_id.as_bytes().to_vec(), method_id))
            }
        })
        .await?
        .map_err(|e| eyre!("Failed to spawn blocking task: {}", e))?;

        Ok(proof)
    }

    /// Generate a Groth16 proof for the final batch
    pub async fn generate_groth16_proof(&self, input: T) -> Result<Groth16> {
        self.generate_groth16_proof_with_retry(input).await
    }

    pub fn decode_journal<U: for<'a> Deserialize<'a>>(&self, proof: &Groth16) -> Result<U> {
        if proof.receipt().journal.bytes.is_empty() {
            return Err(eyre!(
                "Proof journal cannot be empty: {:?}",
                proof.receipt().journal.bytes
            ));
        }

        let receipt = proof.receipt();
        Ok(receipt.journal.decode()?)
    }

    async fn generate_groth16_proof_with_retry(&self, input: T) -> Result<Groth16> {
        let mut retries = 0;
        let mut last_error = None;

        while retries < MAX_RETRIES {
            match self.generate_groth16_proof_internal(input.clone()).await {
                Ok(proof) => return Ok(proof),
                Err(e) => {
                    last_error = Some(e);
                    retries += 1;

                    if retries < MAX_RETRIES {
                        let delay = INITIAL_RETRY_DELAY_MS * (2_u64.pow(retries - 1));
                        tracing::warn!(
                            "Failed to generate Groth16 proof, retrying in {}ms (attempt {}/{})",
                            delay,
                            retries,
                            MAX_RETRIES
                        );
                        sleep(Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            eyre!(
                "Failed to generate Groth16 proof after {} attempts",
                MAX_RETRIES
            )
        }))
    }

    async fn generate_groth16_proof_internal(&self, input: T) -> Result<Groth16> {
        let input_size = std::mem::size_of_val(&input);
        if input_size == 0 {
            return Err(eyre!("Input cannot be empty"));
        }

        debug!("Input size: {} bytes", input_size);

        let method_elf = self.method_elf;
        let input = input.clone();

        let proof = task::spawn_blocking(move || -> Result<Groth16> {
            debug!("Building executor environment");
            let env = ExecutorEnv::builder()
                .write(&input)
                .map_err(|e| eyre!("Failed to write input to executor env: {}", e))?
                .build()
                .map_err(|e| eyre!("Failed to build executor env: {}", e))?;

            debug!("Generating proof with Groth16 options");
            let receipt = default_prover()
                .prove_with_ctx(
                    env,
                    &VerifierContext::default(),
                    method_elf,
                    &ProverOpts::groth16(),
                )
                .map_err(|e| eyre!("Failed to generate Groth16 proof: {}", e))?
                .receipt;

            debug!("Encoding seal");
            let encoded_seal =
                encode_seal(&receipt).map_err(|e| eyre!("Failed to encode seal: {}", e))?;

            debug!("Computing image ID");
            let image_id = compute_image_id(method_elf)
                .map_err(|e| eyre!("Failed to compute image ID: {}", e))?;

            let journal = receipt.journal.bytes.clone();

            debug!("Converting to Groth16 proof");
            let groth16_proof = Groth16Proof::from_risc0(
                encoded_seal,
                image_id.as_bytes().to_vec(),
                journal.clone(),
            );

            debug!("Generating calldata");
            let calldata =
                get_groth16_calldata_felt(&groth16_proof, &get_risc0_vk(), CurveID::BN254)
                    .map_err(|e| eyre!("Failed to generate calldata: {}", e))?;

            info!("Successfully generated Groth16 proof and calldata.");
            Ok(Groth16::new(receipt, calldata))
        })
        .await?
        .map_err(|e| eyre!("Failed to spawn blocking task: {}", e))?;

        Ok(proof)
    }

    #[cfg(test)]
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
    use super::*;
    use serde::{Deserialize, Serialize};

    // Mock data structure for testing
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestInput {
        value: u32,
    }

    const TEST_METHOD_ELF: &[u8] = &[1, 2, 3, 4]; // Mock ELF data
    const TEST_METHOD_ID: [u32; 8] = [1, 0, 0, 0, 0, 0, 0, 0];

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

    #[tokio::test]
    async fn test_generate_groth16_proof_invalid_input() {
        let proof_generator =
            ProofGenerator::<Vec<u8>>::new(TEST_METHOD_ELF, TEST_METHOD_ID).unwrap();
        let result = proof_generator.generate_groth16_proof(vec![]).await;
        assert!(result.is_err());
    }

    // Note: Testing the actual proof generation would require mock implementations
    // of the RISC Zero prover and related components. Here's a sketch of how that
    // might look with proper mocking:

    /*
    #[tokio::test]
    async fn test_generate_stark_proof_success() {
        // Would need to mock:
        // - ExecutorEnv
        // - default_prover
        // - compute_image_id

        let generator = ProofGenerator::<TestInput>::new(TEST_METHOD_ELF, TEST_METHOD_ID).unwrap();
        let input = TestInput { value: 42 };
        let result = generator.generate_stark_proof(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_groth16_proof_success() {
        // Would need to mock:
        // - ExecutorEnv
        // - default_prover
        // - compute_image_id
        // - encode_seal
        // - Groth16Proof conversion
        // - get_groth16_calldata_felt

        let generator = ProofGenerator::<TestInput>::new(TEST_METHOD_ELF, TEST_METHOD_ID).unwrap();
        let input = TestInput { value: 42 };
        let result = generator.generate_groth16_proof(input).await;
        assert!(result.is_ok());
    }
    */

    #[test]
    fn test_decode_journal() {
        // Would need mock Groth16 proof with valid journal data
        // This test would verify that journal decoding works correctly
        // and handles errors appropriately
    }
}
