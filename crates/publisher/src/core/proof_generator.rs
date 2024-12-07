use garaga_rs::{
    calldata::full_proof_with_hints::groth16::{
        get_groth16_calldata, risc0_utils::get_risc0_vk, Groth16Proof,
    },
    definitions::CurveID,
};
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{compute_image_id, default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use serde::Deserialize;
use starknet_crypto::Felt;
use tokio::task;
use tracing::{debug, error, info};

use crate::{
    errors::ProofGeneratorError,
    utils::{Groth16, Stark},
};

pub struct ProofGenerator<T> {
    method_elf: &'static [u8],
    method_id: [u32; 8],
    skip_proof_verification: bool,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> ProofGenerator<T>
where
    T: serde::Serialize + Clone + Send + 'static,
{
    pub fn new(
        method_elf: &'static [u8],
        method_id: [u32; 8],
        skip_proof_verification: bool,
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
            skip_proof_verification,
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

        info!("Generating Groth16 proof...");
        debug!("Input size: {} bytes", input_size);

        let method_elf = self.method_elf;
        let input = input.clone();
        let skip_proof_verification = self.skip_proof_verification;

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
            let groth16_proof = if !skip_proof_verification {
                Groth16Proof::from_risc0(
                    encoded_seal,
                    image_id.as_bytes().to_vec(),
                    journal.clone(),
                )
            } else {
                Default::default()
            };

            debug!("Generating calldata");
            let calldata = if !skip_proof_verification {
                get_groth16_calldata(&groth16_proof, &get_risc0_vk(), CurveID::BN254).map_err(
                    |e| {
                        error!("Failed to generate calldata: {}", e);
                        ProofGeneratorError::CalldataError(e.to_string())
                    },
                )?
            } else {
                vec![Felt::ZERO]
            };
            println!("calldata: {:?}", calldata);

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
