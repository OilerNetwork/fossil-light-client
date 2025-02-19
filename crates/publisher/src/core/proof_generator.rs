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
use tracing::{debug, error, info};

use crate::{
    errors::ProofGeneratorError,
    utils::{Groth16, Stark},
};

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
    pub fn new(
        method_elf: &'static [u8],
        method_id: [u32; 8],
    ) -> Result<Self, ProofGeneratorError> {
        if method_elf.is_empty() {
            return Err(ProofGeneratorError::InvalidInput(
                "Method ELF cannot be empty",
            ));
        }

        if method_id.iter().all(|&x| x == 0) {
            return Err(ProofGeneratorError::InvalidInput(
                "Method ID cannot be all zeros",
            ));
        }

        Ok(Self {
            method_elf,
            method_id,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Generate a standard Stark proof for intermediate batches
    pub async fn generate_stark_proof(&self, input: T) -> Result<Stark, ProofGeneratorError> {
        let input_size = std::mem::size_of_val(&input);
        if input_size == 0 {
            return Err(ProofGeneratorError::InvalidInput("Input cannot be empty"));
        }

        info!("Generating STARK proof for intermediate batch");
        debug!("Input size: {} bytes", input_size);

        let proof = task::spawn_blocking({
            let method_elf = self.method_elf;
            let method_id = self.method_id;
            let input = input.clone();

            move || -> Result<Stark, ProofGeneratorError> {
                debug!("Building executor environment");
                let env = ExecutorEnv::builder()
                    .write(&input)
                    .map_err(|e| {
                        error!("Failed to write input to executor env: {}", e);
                        ProofGeneratorError::ExecutorEnvError(e.to_string())
                    })?
                    .build()
                    .map_err(|e| {
                        error!("Failed to build executor env: {}", e);
                        ProofGeneratorError::ExecutorEnvError(e.to_string())
                    })?;

                debug!("Generating STARK proof with default prover");
                let receipt = default_prover()
                    .prove(env, method_elf)
                    .map_err(|e| {
                        error!("Failed to generate STARK proof: {}", e);
                        ProofGeneratorError::ReceiptError(e.to_string())
                    })?
                    .receipt;

                debug!("Computing image ID");
                let image_id = compute_image_id(method_elf).map_err(|e| {
                    error!("Failed to compute image ID: {}", e);
                    ProofGeneratorError::ImageIdError(e.to_string())
                })?;

                info!("Successfully generated STARK proof");
                Ok(Stark::new(receipt, image_id.as_bytes().to_vec(), method_id))
            }
        })
        .await?
        .map_err(|e| {
            error!("Failed to spawn blocking task: {}", e);
            ProofGeneratorError::SpawnBlocking(e.to_string())
        })?;

        Ok(proof)
    }

    /// Generate a Groth16 proof for the final batch
    pub async fn generate_groth16_proof(&self, input: T) -> Result<Groth16, ProofGeneratorError> {
        let input_size = std::mem::size_of_val(&input);
        if input_size == 0 {
            return Err(ProofGeneratorError::InvalidInput("Input cannot be empty"));
        }

        debug!("Input size: {} bytes", input_size);

        let method_elf = self.method_elf;
        let input = input.clone();

        let proof = task::spawn_blocking(move || -> Result<Groth16, ProofGeneratorError> {
            debug!("Building executor environment");
            let env = ExecutorEnv::builder()
                .write(&input)
                .map_err(|e| {
                    error!("Failed to write input to executor env: {}", e);
                    ProofGeneratorError::ExecutorEnvError(e.to_string())
                })?
                .build()
                .map_err(|e| {
                    error!("Failed to build executor env: {}", e);
                    ProofGeneratorError::ExecutorEnvError(e.to_string())
                })?;

            debug!("Generating proof with Groth16 options");
            let receipt = default_prover()
                .prove_with_ctx(
                    env,
                    &VerifierContext::default(),
                    method_elf,
                    &ProverOpts::groth16(),
                )
                .map_err(|e| {
                    error!("Failed to generate Groth16 proof: {}", e);
                    ProofGeneratorError::ReceiptError(e.to_string())
                })?
                .receipt;

            debug!("Encoding seal");
            let encoded_seal = encode_seal(&receipt).map_err(|e| {
                error!("Failed to encode seal: {}", e);
                ProofGeneratorError::SealError(e.to_string())
            })?;

            debug!("Computing image ID");
            let image_id = compute_image_id(method_elf).map_err(|e| {
                error!("Failed to compute image ID: {}", e);
                ProofGeneratorError::ImageIdError(e.to_string())
            })?;

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
                    .map_err(|e| {
                        error!("Failed to generate calldata: {}", e);
                        ProofGeneratorError::CalldataError(e.to_string())
                    })?;

            info!("Successfully generated Groth16 proof and calldata.");
            Ok(Groth16::new(receipt, calldata))
        })
        .await?
        .map_err(|e| {
            error!("Failed to spawn blocking task: {}", e);
            ProofGeneratorError::SpawnBlocking(e.to_string())
        })?;

        Ok(proof)
    }

    pub fn decode_journal<U: for<'a> Deserialize<'a>>(
        &self,
        proof: &Groth16,
    ) -> Result<U, ProofGeneratorError> {
        if proof.receipt().journal.bytes.is_empty() {
            return Err(ProofGeneratorError::InvalidInput(
                "Proof journal cannot be empty",
            ));
        }

        let receipt = proof.receipt();
        Ok(receipt.journal.decode()?)
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
            result.unwrap_err(),
            ProofGeneratorError::InvalidInput(_)
        ));

        // Test zero method ID
        let result = ProofGenerator::<TestInput>::new(TEST_METHOD_ELF, [0; 8]);
        assert!(matches!(
            result.unwrap_err(),
            ProofGeneratorError::InvalidInput(_)
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
