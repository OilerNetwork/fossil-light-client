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
    batch_index: u64,
    latest_mmr_block: u64,
    latest_mmr_block_hash: String,
    root_hash: String,
    leaves_count: usize,
}

impl GuestOutput {
    pub fn new(
        batch_index: u64,
        latest_mmr_block: u64,
        latest_mmr_block_hash: String,
        root_hash: String,
        leaves_count: usize,
    ) -> Self {
        Self {
            batch_index,
            latest_mmr_block,
            latest_mmr_block_hash,
            root_hash,
            leaves_count,
        }
    }

    pub fn latest_mmr_block(&self) -> u64 {
        self.latest_mmr_block
    }

    pub fn latest_mmr_block_hash(&self) -> &str {
        &self.latest_mmr_block_hash
    }

    pub fn root_hash(&self) -> &str {
        &self.root_hash
    }

    pub fn batch_index(&self) -> u64 {
        self.batch_index
    }

    pub fn leaves_count(&self) -> usize {
        self.leaves_count
    }
}

// CombinedInput
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CombinedInput {
    chain_id: u64,
    batch_size: u64,
    headers: Vec<BlockHeader>,
    mmr_input: MMRInput,
    batch_link: Option<String>,
    next_batch_link: Option<String>,
    skip_proof_verification: bool,
}

impl CombinedInput {
    pub fn new(
        chain_id: u64,
        batch_size: u64,
        headers: Vec<BlockHeader>,
        mmr_input: MMRInput,
        batch_link: Option<String>,
        next_batch_link: Option<String>,
        skip_proof_verification: bool,
    ) -> Self {
        Self {
            chain_id,
            batch_size,
            headers,
            mmr_input,
            batch_link,
            next_batch_link,
            skip_proof_verification,
        }
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub fn batch_size(&self) -> u64 {
        self.batch_size
    }

    pub fn headers(&self) -> &Vec<BlockHeader> {
        &self.headers
    }

    pub fn mmr_input(&self) -> &MMRInput {
        &self.mmr_input
    }

    pub fn batch_link(&self) -> Option<&str> {
        self.batch_link.as_deref()
    }

    pub fn next_batch_link(&self) -> Option<&str> {
        self.next_batch_link.as_deref()
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
pub struct GuestProof {
    pub element_index: usize,
    pub element_hash: String,
    pub siblings_hashes: Vec<String>,
    pub peaks_hashes: Vec<String>,
    pub elements_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocksValidityInput {
    chain_id: u64,
    headers: Vec<BlockHeader>,
    mmr_input: MMRInput,
    proofs: Vec<GuestProof>,
}
impl BlocksValidityInput {
    pub fn new(
        chain_id: u64,
        headers: Vec<BlockHeader>,
        mmr_input: MMRInput,
        proofs: Vec<GuestProof>,
    ) -> Self {
        Self {
            chain_id,
            headers,
            mmr_input,
            proofs,
        }
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub fn headers(&self) -> &Vec<BlockHeader> {
        &self.headers
    }

    pub fn proofs(&self) -> &Vec<GuestProof> {
        &self.proofs
    }

    pub fn mmr_input(&self) -> &MMRInput {
        &self.mmr_input
    }
}
