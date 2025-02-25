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
    first_block_parent_hash: String,
    avg_fees: Vec<(usize, usize, u64)>, // (timestamp, data_points, avg_fee)
}

impl GuestOutput {
    pub fn new(
        batch_index: u64,
        latest_mmr_block: u64,
        latest_mmr_block_hash: String,
        root_hash: String,
        leaves_count: usize,
        first_block_parent_hash: String,
        avg_fees: Vec<(usize, usize, u64)>,
    ) -> Self {
        Self {
            batch_index,
            latest_mmr_block,
            latest_mmr_block_hash,
            root_hash,
            leaves_count,
            first_block_parent_hash,
            avg_fees,
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

    pub fn first_block_parent_hash(&self) -> &str {
        &self.first_block_parent_hash
    }
}

// CombinedInput
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CombinedInput {
    chain_id: u64,
    batch_size: u64,
    headers: Vec<(i64, Vec<BlockHeader>)>, // (representative_timestamp, headers)
    mmr_input: MMRInput,
}

impl CombinedInput {
    pub fn new(
        chain_id: u64,
        batch_size: u64,
        headers: Vec<(i64, Vec<BlockHeader>)>,
        mmr_input: MMRInput,
    ) -> Self {
        Self {
            chain_id,
            batch_size,
            headers,
            mmr_input,
        }
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub fn batch_size(&self) -> u64 {
        self.batch_size
    }

    pub fn headers(&self) -> &Vec<(i64, Vec<BlockHeader>)> {
        &self.headers
    }

    pub fn mmr_input(&self) -> &MMRInput {
        &self.mmr_input
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_result() {
        let result = AppendResult::new(10, 15, 5, "test_hash".to_string());

        assert_eq!(result.leaves_count(), 10);
        assert_eq!(result.last_element_idx(), 15);
        assert_eq!(result.element_index(), 5);
        assert_eq!(result.value(), "test_hash");
    }

    #[test]
    fn test_guest_output() {
        let output = GuestOutput::new(
            1,
            100,
            "block_hash".to_string(),
            "root_hash".to_string(),
            50,
            "first_block_parent_hash".to_string(),
            vec![(0, 0, 100), (1, 0, 200), (2, 0, 300), (3, 0, 400)],
        );

        assert_eq!(output.batch_index(), 1);
        assert_eq!(output.latest_mmr_block(), 100);
        assert_eq!(output.latest_mmr_block_hash(), "block_hash");
        assert_eq!(output.root_hash(), "root_hash");
        assert_eq!(output.leaves_count(), 50);
        assert_eq!(output.first_block_parent_hash(), "first_block_parent_hash");
    }

    #[test]
    fn test_combined_input() {
        let mmr_input = MMRInput::new(vec!["peak1".to_string()], 10, 5, vec!["elem1".to_string()]);

        let input = CombinedInput::new(1, 100, Vec::new(), mmr_input.clone());

        assert_eq!(input.chain_id(), 1);
        assert_eq!(input.batch_size(), 100);
        assert!(input.headers().is_empty());

        // Test MMRInput getters
        assert_eq!(input.mmr_input().elements_count(), 10);
        assert_eq!(input.mmr_input().leaves_count(), 5);
        assert_eq!(input.mmr_input().initial_peaks(), vec!["peak1"]);
    }

    #[test]
    fn test_final_hash() {
        let hash = FinalHash::new("test_hash".to_string(), 42);

        assert_eq!(hash.hash(), "test_hash");
        assert_eq!(hash.index(), 42);
    }

    #[test]
    fn test_blocks_validity_input() {
        let mmr_input = MMRInput::new(vec!["peak1".to_string()], 10, 5, vec!["elem1".to_string()]);

        let guest_proof = GuestProof {
            element_index: 1,
            element_hash: "hash".to_string(),
            siblings_hashes: vec!["sibling".to_string()],
            peaks_hashes: vec!["peak".to_string()],
            elements_count: 10,
        };

        let input = BlocksValidityInput::new(1, Vec::new(), mmr_input, vec![guest_proof]);

        assert_eq!(input.chain_id(), 1);
        assert!(input.headers().is_empty());
        assert_eq!(input.proofs().len(), 1);
        assert_eq!(input.mmr_input().elements_count(), 10);
    }
}
