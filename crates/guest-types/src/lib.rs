// mod.rs

#![deny(unused_crate_dependencies)]

use eth_rlp_types::BlockHeader;
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
    value: String,
}

impl AppendResult {
    pub fn new(
        leaves_count: usize,
        elements_count: usize,
        element_index: usize,
        value: String,
    ) -> Self {
        Self {
            leaves_count,
            elements_count,
            element_index,
            value,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
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
    root_hash: String,
    elements_count: usize,
    leaves_count: usize,
    all_hashes: Vec<(usize, String)>,
    append_results: Vec<AppendResult>,
}

impl GuestOutput {
    pub fn new(
        root_hash: String,
        elements_count: usize,
        leaves_count: usize,
        all_hashes: Vec<(usize, String)>,
        append_results: Vec<AppendResult>,
    ) -> Self {
        Self {
            root_hash,
            all_hashes,
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

    pub fn all_hashes(&self) -> Vec<(usize, String)> {
        self.all_hashes.clone()
    }

    pub fn leaves_count(&self) -> usize {
        self.leaves_count
    }
}

// CombinedInput
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CombinedInput {
    headers: Vec<BlockHeader>,
    mmr_input: MMRInput,
    skip_proof_verification: bool,
}

impl CombinedInput {
    pub fn new(
        headers: Vec<BlockHeader>,
        mmr_input: MMRInput,
        skip_proof_verification: bool,
    ) -> Self {
        Self {
            headers,
            mmr_input,
            skip_proof_verification,
        }
    }

    pub fn headers(&self) -> &Vec<BlockHeader> {
        &self.headers
    }

    pub fn mmr_input(&self) -> &MMRInput {
        &self.mmr_input
    }

    pub fn skip_proof_verification(&self) -> bool {
        self.skip_proof_verification
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MMRInput {
    initial_peaks: Vec<String>,
    elements_count: usize,
    leaves_count: usize,
    new_elements: Vec<String>,
}

impl MMRInput {
    pub fn new(
        initial_peaks: Vec<String>,
        elements_count: usize,
        leaves_count: usize,
        new_elements: Vec<String>,
    ) -> Self {
        Self {
            initial_peaks,
            elements_count,
            leaves_count,
            new_elements,
        }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FinalHash {
    hash: String,
    index: usize,
}

impl FinalHash {
    pub fn new(hash: String, index: usize) -> Self {
        Self { hash, index }
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocksValidityInput {
    headers: Vec<BlockHeader>,
    mmr_input: MMRInput,
    hash_indexes: Vec<usize>,
}
impl BlocksValidityInput {
    pub fn new(headers: Vec<BlockHeader>, mmr_input: MMRInput, hash_indexes: Vec<usize>) -> Self {
        Self {
            headers,
            mmr_input,
            hash_indexes,
        }
    }

    pub fn headers(&self) -> &Vec<BlockHeader> {
        &self.headers
    }

    pub fn hash_indexes(&self) -> &Vec<usize> {
        &self.hash_indexes
    }

    pub fn mmr_input(&self) -> &MMRInput {
        &self.mmr_input
    }
}
