#![deny(unused_crate_dependencies)]

use block_validity::BlockHeader;
use risc0_zkvm::Receipt;
use serde::{Deserialize, Serialize};

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
}

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

    pub fn final_peaks(&self) -> &Vec<String> {
        &self.final_peaks
    }
}

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
}

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
}

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
}
