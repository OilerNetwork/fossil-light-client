// mod.rs

#![deny(unused_crate_dependencies)]

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
// AppendResult
#[derive(Debug, Serialize, Deserialize)]
pub struct AppendResult {
    leaves_count: usize,
    elements_count: usize,
    element_index: usize,
    root_hash: String,
}

impl AppendResult {
    pub fn new(
        leaves_count: usize,
        elements_count: usize,
        element_index: usize,
        root_hash: String,
    ) -> Self {
        Self {
            leaves_count,
            elements_count,
            element_index,
            root_hash,
        }
    }

    pub fn root_hash(&self) -> &str {
        &self.root_hash
    }

    pub fn element_index(&self) -> usize {
        self.element_index
    }

    pub fn leaves_count(&self) -> usize {
        self.leaves_count
    }

    pub fn last_element_idx(&self) -> usize {
        self.elements_count
    }
}

// GuestOutput
#[derive(Debug, Serialize, Deserialize)]
pub struct GuestOutput {
    final_peaks: Vec<String>,
    elements_count: usize,
    leaves_count: usize,
    append_results: Vec<AppendResult>,
}

impl GuestOutput {
    pub fn new(
        final_peaks: Vec<String>,
        elements_count: usize,
        leaves_count: usize,
        append_results: Vec<AppendResult>,
    ) -> Self {
        Self {
            final_peaks,
            elements_count,
            leaves_count,
            append_results,
        }
    }

    pub fn elements_count(&self) -> usize {
        self.elements_count
    }

    pub fn append_results(&self) -> &Vec<AppendResult> {
        &self.append_results
    }

    pub fn final_peaks(&self) -> Vec<String> {
        self.final_peaks.clone()
    }

    pub fn leaves_count(&self) -> usize {
        self.leaves_count
    }
}

// CombinedInput
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CombinedInput {
    headers: Vec<BlockHeader>,
    mmr_input: GuestInput,
}

impl CombinedInput {
    pub fn new(headers: Vec<BlockHeader>, mmr_input: GuestInput) -> Self {
        Self { headers, mmr_input }
    }

    pub fn headers(&self) -> &Vec<BlockHeader> {
        &self.headers
    }

    pub fn mmr_input(&self) -> &GuestInput {
        &self.mmr_input
    }
}

// GuestInput
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestInput {
    initial_peaks: Vec<String>,
    elements_count: usize,
    leaves_count: usize,
    new_elements: Vec<String>,
    previous_proofs: Vec<BatchProof>,
}

impl GuestInput {
    pub fn new(
        initial_peaks: Vec<String>,
        elements_count: usize,
        leaves_count: usize,
        new_elements: Vec<String>,
        previous_proofs: Vec<BatchProof>,
    ) -> Self {
        Self {
            initial_peaks,
            elements_count,
            leaves_count,
            new_elements,
            previous_proofs,
        }
    }

    pub fn previous_proofs(&self) -> &Vec<BatchProof> {
        &self.previous_proofs
    }

    pub fn initial_peaks(&self) -> Vec<String> {
        self.initial_peaks.clone()
    }

    pub fn elements_count(&self) -> usize {
        self.elements_count
    }

    pub fn leaves_count(&self) -> usize {
        self.leaves_count
    }
}

// BatchProof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProof {
    receipt: Receipt,
    image_id: Vec<u8>,
    method_id: [u32; 8],
}
impl BatchProof {
    pub fn new(receipt: Receipt, image_id: Vec<u8>, method_id: [u32; 8]) -> Self {
        Self {
            receipt,
            image_id,
            method_id,
        }
    }

    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }

    pub fn method_id(&self) -> [u32; 8] {
        self.method_id
    }
}