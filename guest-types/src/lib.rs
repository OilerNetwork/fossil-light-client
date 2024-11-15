use block_validity::BlockHeader;
use risc0_zkvm::Receipt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
pub struct PeaksOptions {
    pub elements_count: Option<usize>,
    pub formatting_opts: Option<PeaksFormattingOptions>,
}

#[derive(Clone)]
pub struct FormattingOptions {
    pub output_size: usize,
    pub null_value: String,
}

pub type PeaksFormattingOptions = FormattingOptions;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppendResult {
    pub leaves_count: usize,
    pub elements_count: usize,
    pub element_index: usize,
    pub root_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GuestOutput {
    pub final_peaks: Vec<String>,
    pub elements_count: usize,
    pub leaves_count: usize,
    pub append_results: Vec<AppendResult>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CombinedInput {
    pub headers: Vec<BlockHeader>,
    pub mmr_input: GuestInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestInput {
    pub initial_peaks: Vec<String>,
    pub elements_count: usize,
    pub leaves_count: usize,
    pub new_elements: Vec<String>,
    pub previous_proofs: Vec<BatchProof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProof {
    pub receipt: Receipt,
    pub image_id: Vec<u8>,
    pub method_id: [u32; 8],
}
