use risc0_zkvm::Receipt;
use serde::{Deserialize, Serialize};
use starknet_crypto::Felt;
use starknet_handler::MmrState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofType {
    Stark {
        receipt: Receipt,
        image_id: Vec<u8>,
        method_id: [u32; 8],
    },
    Groth16 {
        receipt: Receipt,
        calldata: Vec<Felt>,
    },
}

#[derive(Debug, Clone)]
pub struct BatchResult {
    start_block: u64,
    end_block: u64,
    new_mmr_state: MmrState,
    proof: Option<ProofType>,
}

impl BatchResult {
    pub fn new(
        start_block: u64,
        end_block: u64,
        new_mmr_state: MmrState,
        proof: Option<ProofType>,
    ) -> Self {
        Self {
            start_block,
            end_block,
            new_mmr_state,
            proof,
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

    pub fn proof(&self) -> Option<ProofType> {
        self.proof.clone()
    }
}
