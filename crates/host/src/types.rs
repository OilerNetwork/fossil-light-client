use risc0_zkvm::Receipt;
use serde::{Deserialize, Serialize};
use starknet_crypto::Felt;

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

pub struct BatchResult {
    pub start_block: u64,
    pub end_block: u64,
    pub new_mmr_root_hash: String,
    pub proof: Option<ProofType>,
}

// #[derive(Clone)]
// pub struct MMRState {
//     pub peaks: Vec<String>,
//     pub elements_count: usize,
//     pub leaves_count: usize,
// }
