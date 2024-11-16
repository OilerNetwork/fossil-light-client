pub use crate::types::ProofType;
use eyre::Result;
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
use tokio::task;
use tracing::info;

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
    pub async fn generate_stark_proof(&self, input: &CombinedInput) -> Result<ProofType> {
        let method_elf = self.method_elf;
        let method_id = self.method_id;
        let input = input.clone();

        info!("Generating STARK proof...");

        let proof = task::spawn_blocking(move || -> eyre::Result<ProofType> {
            let env = ExecutorEnv::builder()
                .write(&input)
                .map_err(|e| eyre::eyre!("Failed to write input: {}", e))?
                .build()
                .map_err(|e| eyre::eyre!("Failed to build executor env: {}", e))?;

            let receipt = default_prover()
                .prove(env, method_elf)
                .map_err(|e| eyre::eyre!("Proof generation failed: {}", e))?
                .receipt;

            let image_id = compute_image_id(method_elf)
                .map_err(|e| eyre::eyre!("Failed to compute image id: {}", e))?;

            Ok(ProofType::Stark {
                receipt,
                image_id: image_id.as_bytes().to_vec(),
                method_id,
            })
        })
        .await?
        .map_err(|e| eyre::eyre!("Spawn blocking task failed: {}", e))?;

        Ok(proof)
    }

    /// Generate a Groth16 proof for the final batch
    pub async fn generate_groth16_proof(&self, input: &CombinedInput) -> Result<ProofType> {
        let method_elf = self.method_elf;
        // let method_id = self.method_id;
        let input = input.clone();

        info!("Generating Groth16 proof...");

        let proof = task::spawn_blocking(move || -> eyre::Result<ProofType> {
            let env = ExecutorEnv::builder()
                .write(&input)
                .map_err(|e| eyre::eyre!("Failed to write input: {}", e))?
                .build()
                .map_err(|e| eyre::eyre!("Failed to build executor env: {}", e))?;

            // Generate with Groth16 options
            let receipt = default_prover()
                .prove_with_ctx(
                    env,
                    &VerifierContext::default(),
                    method_elf,
                    &ProverOpts::groth16(),
                )
                .map_err(|e| eyre::eyre!("Proof generation failed: {}", e))?
                .receipt;

            // Convert to Groth16
            let encoded_seal =
                encode_seal(&receipt).map_err(|e| eyre::eyre!("Failed to encode seal: {}", e))?;

            let image_id = compute_image_id(method_elf)
                .map_err(|e| eyre::eyre!("Failed to compute image id: {}", e))?;

            let journal = receipt.journal.bytes.clone();

            let groth16_proof =
                Groth16Proof::from_risc0(encoded_seal, image_id.as_bytes().to_vec(), journal);

            info!("Generating StarkNet calldata...");
            let calldata = get_groth16_calldata(&groth16_proof, &get_risc0_vk(), CurveID::BN254)
                .map_err(|e| eyre::eyre!("Failed to generate StarkNet calldata: {}", e))?;

            Ok(ProofType::Groth16 { receipt, calldata })
        })
        .await?
        .map_err(|e| eyre::eyre!("Spawn blocking task failed: {}", e))?;

        Ok(proof)
    }

    pub fn decode_journal<T: for<'a> Deserialize<'a>>(&self, proof: &ProofType) -> Result<T> {
        let receipt = match proof {
            ProofType::Groth16 { receipt, .. } | ProofType::Stark { receipt, .. } => receipt,
        };
        Ok(receipt.journal.decode()?)
    }
}
