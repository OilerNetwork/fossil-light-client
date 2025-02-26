use risc0_zkvm::{Journal, Receipt};
use serde::{Deserialize, Serialize};
use starknet_crypto::Felt;
use starknet_handler::MmrState;

use eyre::{eyre, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Groth16 {
    receipt: Receipt,
    calldata: Vec<Felt>,
}

impl Groth16 {
    pub fn new(receipt: Receipt, calldata: Vec<Felt>) -> Self {
        Self { receipt, calldata }
    }

    pub fn receipt(&self) -> Receipt {
        self.receipt.clone()
    }

    pub fn calldata(&self) -> Vec<Felt> {
        self.calldata.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stark {
    receipt: Receipt,
    image_id: Vec<u8>,
    method_id: [u32; 8],
}

impl Stark {
    pub fn new(receipt: Receipt, image_id: Vec<u8>, method_id: [u32; 8]) -> Self {
        Self {
            receipt: receipt.clone(),
            image_id,
            method_id,
        }
    }

    pub fn receipt(&self) -> Receipt {
        self.receipt.clone()
    }

    pub fn journal(&self) -> Journal {
        self.receipt.journal.clone()
    }

    pub fn image_id(&self) -> Result<[u8; 32]> {
        self.image_id.clone().try_into().map_err(|_| {
            eyre!(
                "Failed to convert image ID to [u8; 32]: {:?}",
                self.image_id
            )
        })
    }
}

#[derive(Debug, Clone)]
pub struct BatchResult {
    start_block: u64,
    end_block: u64,
    new_mmr_state: MmrState,
    proof: Option<Groth16>,
    ipfs_hash: String,
}

impl BatchResult {
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

    pub fn start_block(&self) -> u64 {
        self.start_block
    }

    pub fn end_block(&self) -> u64 {
        self.end_block
    }

    pub fn new_mmr_state(&self) -> MmrState {
        self.new_mmr_state.clone()
    }

    pub fn proof(&self) -> Option<Groth16> {
        self.proof.clone()
    }

    pub fn ipfs_hash(&self) -> String {
        self.ipfs_hash.clone()
    }
}
