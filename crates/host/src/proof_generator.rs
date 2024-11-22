pub use crate::types::ProofType;
use garaga_rs::{
    calldata::full_proof_with_hints::groth16::{
        get_groth16_calldata, risc0_utils::get_risc0_vk, Groth16Proof,
    },
    definitions::CurveID,
};
use guest_types::CombinedInput;
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{compute_image_id, default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use serde::Deserialize;
use thiserror::Error;
use tokio::task;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum ProofGeneratorError {
    #[error("Failed to write input to executor env: {0}")]
    ExecutorEnvError(String),
    #[error("Failed to generate receipt: {0}")]
    ReceiptError(String),
    #[error("Failed to compute image id: {0}")]
    ImageIdError(String),
    #[error("Failed to encode seal: {0}")]
    SealError(String),
    #[error("Failed to generate StarkNet calldata: {0}")]
    CalldataError(String),
    #[error("Failed to spawn blocking task: {0}")]
    SpawnBlocking(String),
    #[error("Task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("Risc0 serde error: {0}")]
    Risc0Serde(#[from] risc0_zkvm::serde::Error),
}

pub struct ProofGenerator {
    method_elf: &'static [u8],
    method_id: [u32; 8],
}

impl ProofGenerator {
    pub fn new(method_elf: &'static [u8], method_id: [u32; 8]) -> Self {
        Self {
            method_elf,
            method_id,
        }
    }

    /// Generate a standard Stark proof for intermediate batches
    pub async fn generate_stark_proof(
        &self,
        input: &CombinedInput,
    ) -> Result<ProofType, ProofGeneratorError> {
        info!("Generating STARK proof for intermediate batch");
        debug!("Input size: {} bytes", std::mem::size_of_val(input));

        let method_elf = self.method_elf;
        let method_id = self.method_id;
        let input = input.clone();

        let proof = task::spawn_blocking(move || -> eyre::Result<ProofType> {
            debug!("Building executor environment");
            let env = ExecutorEnv::builder()
                .write(&input)
                .map_err(|e| {
                    warn!("Failed to write input to executor env: {}", e);
                    ProofGeneratorError::ExecutorEnvError(e.to_string())
                })?
                .build()
                .map_err(|e| {
                    warn!("Failed to build executor env: {}", e);
                    ProofGeneratorError::ExecutorEnvError(e.to_string())
                })?;

            debug!("Generating STARK proof with default prover");
            let receipt = default_prover()
                .prove(env, method_elf)
                .map_err(|e| {
                    warn!("Failed to generate STARK proof: {}", e);
                    ProofGeneratorError::ReceiptError(e.to_string())
                })?
                .receipt;

            debug!("Computing image ID");
            let image_id = compute_image_id(method_elf)
                .map_err(|e| ProofGeneratorError::ImageIdError(e.to_string()))?;

            info!("Successfully generated STARK proof");
            Ok(ProofType::Stark {
                receipt,
                image_id: image_id.as_bytes().to_vec(),
                method_id,
            })
        })
        .await?
        .map_err(|e| ProofGeneratorError::SpawnBlocking(e.to_string()))?;

        Ok(proof)
    }

    /// Generate a Groth16 proof for the final batch
    pub async fn generate_groth16_proof(
        &self,
        input: &CombinedInput,
    ) -> Result<ProofType, ProofGeneratorError> {
        info!("Generating Groth16 proof for final batch");
        debug!("Input size: {} bytes", std::mem::size_of_val(input));

        let method_elf = self.method_elf;
        let input = input.clone();

        let proof = task::spawn_blocking(move || -> eyre::Result<ProofType> {
            debug!("Building executor environment");
            let env = ExecutorEnv::builder()
                .write(&input)
                .map_err(|e| {
                    warn!("Failed to write input to executor env: {}", e);
                    ProofGeneratorError::ExecutorEnvError(e.to_string())
                })?
                .build()
                .map_err(|e| {
                    warn!("Failed to build executor env: {}", e);
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
                    warn!("Failed to generate Groth16 proof: {}", e);
                    ProofGeneratorError::ReceiptError(e.to_string())
                })?
                .receipt;

            debug!("Encoding seal");
            let encoded_seal = encode_seal(&receipt).map_err(|e| {
                warn!("Failed to encode seal: {}", e);
                ProofGeneratorError::SealError(e.to_string())
            })?;

            debug!("Computing image ID");
            let image_id = compute_image_id(method_elf)
                .map_err(|e| ProofGeneratorError::ImageIdError(e.to_string()))?;

            let journal = receipt.journal.bytes.clone();
            debug!("Journal size: {} bytes", journal.len());

            debug!("Converting to Groth16 proof");
            let groth16_proof =
                Groth16Proof::from_risc0(encoded_seal, image_id.as_bytes().to_vec(), journal);

            debug!("Generating calldata");
            let calldata = get_groth16_calldata(&groth16_proof, &get_risc0_vk(), CurveID::BN254)
                .map_err(|e| {
                    warn!("Failed to generate calldata: {}", e);
                    ProofGeneratorError::CalldataError(e.to_string())
                })?;

            info!("Successfully generated Groth16 proof");
            Ok(ProofType::Groth16 { receipt, calldata })
        })
        .await?
        .map_err(|e| ProofGeneratorError::SpawnBlocking(e.to_string()))?;

        Ok(proof)
    }

    pub fn decode_journal<T: for<'a> Deserialize<'a>>(
        &self,
        proof: &ProofType,
    ) -> Result<T, ProofGeneratorError> {
        let receipt = match proof {
            ProofType::Groth16 { receipt, .. } | ProofType::Stark { receipt, .. } => receipt,
        };
        Ok(receipt.journal.decode()?)
    }
}
