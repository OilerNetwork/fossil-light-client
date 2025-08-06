use risc0_zkvm::{Journal, Receipt};
use serde::{Deserialize, Serialize};
use starknet_crypto::Felt;
use starknet_handler::MmrState;

use crate::error::{PublisherError, PublisherResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Groth16 proof wrapper containing receipt and calldata
pub struct Groth16 {
    receipt: Receipt,
    calldata: Vec<Felt>,
}

impl Groth16 {
    /// Create a new Groth16 proof
    pub fn new(receipt: Receipt, calldata: Vec<Felt>) -> Self {
        Self { receipt, calldata }
    }

    /// Get the RISC Zero receipt for Groth16
    pub fn receipt(&self) -> Receipt {
        self.receipt.clone()
    }

    /// Get the Starknet calldata
    pub fn calldata(&self) -> Vec<Felt> {
        self.calldata.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// STARK proof wrapper containing receipt and method information
pub struct Stark {
    receipt: Receipt,
    image_id: Vec<u8>,
    method_id: [u32; 8],
}

impl Stark {
    /// Create a new STARK proof
    pub fn new(receipt: Receipt, image_id: Vec<u8>, method_id: [u32; 8]) -> Self {
        Self {
            receipt: receipt.clone(),
            image_id,
            method_id,
        }
    }

    /// Get the RISC Zero receipt from the STARK proof
    pub fn receipt(&self) -> Receipt {
        self.receipt.clone()
    }

    /// Get the journal from the receipt
    pub fn journal(&self) -> Journal {
        self.receipt.journal.clone()
    }

    /// Get the image ID as a 32-byte array
    pub fn image_id(&self) -> PublisherResult<[u8; 32]> {
        self.image_id.clone().try_into().map_err(|_| {
            PublisherError::serialization(format!(
                "Failed to convert image ID to [u8; 32]: {:?}",
                self.image_id
            ))
        })
    }
}

#[derive(Debug, Clone)]
/// Result of processing a batch of blocks, containing the new MMR state and optional proof
pub struct BatchResult {
    start_block: u64,
    end_block: u64,
    new_mmr_state: MmrState,
    proof: Option<Groth16>,
    ipfs_hash: String,
}

impl BatchResult {
    /// Create a new BatchResult
    pub fn new(
        start_block: u64,
        end_block: u64,
        new_mmr_state: MmrState,
        proof: Option<Groth16>,
        ipfs_hash: String,
    ) -> Self {
        Self {
            start_block,
            end_block,
            new_mmr_state,
            proof,
            ipfs_hash,
        }
    }

    /// Get the starting block number of the batch
    pub fn start_block(&self) -> u64 {
        self.start_block
    }

    /// Get the ending block number of the batch
    pub fn end_block(&self) -> u64 {
        self.end_block
    }

    /// Get the new MMR state after processing the batch
    pub fn new_mmr_state(&self) -> MmrState {
        self.new_mmr_state.clone()
    }

    /// Get the optional Groth16 proof for the batch
    pub fn proof(&self) -> Option<Groth16> {
        self.proof.clone()
    }

    /// Get the IPFS hash of the batch data
    pub fn ipfs_hash(&self) -> String {
        self.ipfs_hash.clone()
    }
}
