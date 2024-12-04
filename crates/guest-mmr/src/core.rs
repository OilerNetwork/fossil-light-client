use guest_types::{AppendResult, GuestProof};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

use crate::formatting::{FormattingError, ProofOptions};
use crate::helper::{
    element_index_to_leaf_index, find_peaks, find_siblings, get_peak_info, hasher,
    leaf_count_to_append_no_merges, leaf_count_to_peaks_count, mmr_size_to_leaf_count,
};

#[derive(Error, Debug)]
pub enum MMRError {
    #[error("No hash found for index {0}")]
    NoHashFoundForIndex(usize),
    #[error("Insufficient peaks for merge")]
    InsufficientPeaksForMerge,
    #[error("From hex error: {0}")]
    FromHexError(#[from] hex::FromHexError),
    #[error("Parse big int error: {0}")]
    ParseBigIntError(#[from] num_bigint::ParseBigIntError),
    #[error("Hash error")]
    HashError,
    #[error("Invalid element index")]
    InvalidElementIndex,
    #[error("Invalid element count")]
    InvalidElementCount,
    #[error("Formatting error: {0}")]
    FormattingError(#[from] FormattingError),
    #[error("Invalid peaks count")]
    InvalidPeaksCount,
}

#[derive(Debug)]
pub struct GuestMMR {
    hashes: HashMap<usize, String>,
    elements_count: usize,
    leaves_count: usize,
    root_hash: String,
}

impl GuestMMR {
    pub fn new(initial_peaks: Vec<String>, elements_count: usize, leaves_count: usize) -> Self {
        let mut hashes = HashMap::new();

        // Initialize hashes with the peaks at their correct positions
        let peak_positions = find_peaks(elements_count);
        for (peak, pos) in initial_peaks.into_iter().zip(peak_positions) {
            hashes.insert(pos, peak);
        }

        Self {
            elements_count,
            leaves_count,
            hashes,
            root_hash: "".to_string(),
        }
    }

    pub fn get_elements_count(&self) -> usize {
        self.elements_count
    }

    pub fn get_leaves_count(&self) -> usize {
        self.leaves_count
    }

    pub fn append(&mut self, value: String) -> Result<AppendResult, MMRError> {
        let elements_count = self.elements_count;

        let mut peaks = self.retrieve_peaks_hashes(find_peaks(elements_count))?;

        let mut last_element_idx = self.elements_count + 1;
        let leaf_element_index = last_element_idx;

        // Store the new leaf in the hash map
        self.hashes.insert(last_element_idx, value.clone());

        peaks.push(value.clone());

        let no_merges = leaf_count_to_append_no_merges(self.leaves_count);

        for _ in 0..no_merges {
            if peaks.len() < 2 {
                return Err(MMRError::InsufficientPeaksForMerge);
            }

            last_element_idx += 1;

            // Pop the last two peaks to merge
            let right_hash = peaks.pop().ok_or(MMRError::InsufficientPeaksForMerge)?;
            let left_hash = peaks.pop().ok_or(MMRError::InsufficientPeaksForMerge)?;

            let parent_hash = hasher(vec![left_hash, right_hash])?;
            self.hashes.insert(last_element_idx, parent_hash.clone());

            peaks.push(parent_hash);
        }

        self.elements_count = last_element_idx;
        self.leaves_count += 1;

        let root_hash = self.calculate_root_hash(last_element_idx)?;
        self.root_hash = root_hash;

        Ok(AppendResult::new(
            self.leaves_count,
            last_element_idx,
            leaf_element_index,
            value,
        ))
    }

    pub fn get_proof(&self, element_index: usize) -> Result<GuestProof, MMRError> {
        if element_index == 0 {
            return Err(MMRError::InvalidElementIndex);
        }

        let tree_size = self.elements_count;

        if element_index > tree_size {
            return Err(MMRError::InvalidElementIndex);
        }

        let peaks = find_peaks(tree_size);

        let siblings = find_siblings(element_index, tree_size)?;

        let peaks_hashes = self.retrieve_peaks_hashes(peaks)?;

        let siblings_hashes = self.get_many_hashes(&siblings)?;

        let element_hash = self
            .hashes
            .get(&element_index)
            .ok_or(MMRError::NoHashFoundForIndex(element_index))?;

        Ok(GuestProof {
            element_index,
            element_hash: element_hash.clone(),
            siblings_hashes,
            peaks_hashes,
            elements_count: tree_size,
        })
    }

    pub fn verify_proof(
        &self,
        mut proof: GuestProof,
        element_value: String,
        options: Option<ProofOptions>,
    ) -> Result<bool, MMRError> {
        let options = options.unwrap_or_default();
        let tree_size = match options.elements_count {
            Some(count) => count,
            None => self.elements_count,
        };

        let leaf_count = mmr_size_to_leaf_count(tree_size);
        let peaks_count = leaf_count_to_peaks_count(leaf_count);

        if peaks_count as usize != proof.peaks_hashes.len() {
            return Err(MMRError::InvalidPeaksCount);
        }

        if let Some(formatting_opts) = options.formatting_opts {
            let proof_format_null_value = &formatting_opts.proof.null_value;
            let peaks_format_null_value = &formatting_opts.peaks.null_value;

            let proof_null_values_count = proof
                .siblings_hashes
                .iter()
                .filter(|&s| s == proof_format_null_value)
                .count();
            proof
                .siblings_hashes
                .truncate(proof.siblings_hashes.len() - proof_null_values_count);

            let peaks_null_values_count = proof
                .peaks_hashes
                .iter()
                .filter(|&s| s == peaks_format_null_value)
                .count();
            proof
                .peaks_hashes
                .truncate(proof.peaks_hashes.len() - peaks_null_values_count);
        }
        let element_index = proof.element_index;

        if element_index == 0 {
            return Err(MMRError::InvalidElementIndex);
        }

        if element_index > tree_size {
            return Err(MMRError::InvalidElementIndex);
        }

        let (peak_index, peak_height) = get_peak_info(tree_size, element_index);
        if proof.siblings_hashes.len() != peak_height {
            return Ok(false);
        }

        let mut hash = element_value.clone();
        let mut leaf_index = element_index_to_leaf_index(element_index)?;

        for proof_hash in proof.siblings_hashes.iter() {
            let is_right = leaf_index % 2 == 1;
            leaf_index /= 2;

            hash = if is_right {
                hasher(vec![proof_hash.clone(), hash.clone()])?
            } else {
                hasher(vec![hash.clone(), proof_hash.clone()])?
            };
        }

        let peak_hashes = self.retrieve_peaks_hashes(find_peaks(tree_size))?;

        Ok(peak_hashes[peak_index] == hash)
    }

    fn retrieve_peaks_hashes(&self, peak_idxs: Vec<usize>) -> Result<Vec<String>, MMRError> {
        let mut peaks = Vec::new();

        for &idx in &peak_idxs {
            // Use `idx` directly since `self.hashes` expects a `usize` key
            if let Some(hash) = self.hashes.get(&idx) {
                peaks.push(hash.clone());
            } else {
                return Err(MMRError::NoHashFoundForIndex(idx));
            }
        }

        Ok(peaks)
    }

    pub fn bag_the_peaks(&self) -> Result<String, MMRError> {
        let peaks_idxs = find_peaks(self.elements_count);

        let peaks_hashes = self.retrieve_peaks_hashes(peaks_idxs)?;

        match peaks_hashes.len() {
            0 => Ok("0x0".to_string()),
            1 => Ok(peaks_hashes[0].clone()),
            _ => {
                let mut peaks_hashes: VecDeque<String> = peaks_hashes.into();
                let last = peaks_hashes
                    .pop_back()
                    .ok_or(MMRError::InsufficientPeaksForMerge)?;
                let second_last = peaks_hashes
                    .pop_back()
                    .ok_or(MMRError::InsufficientPeaksForMerge)?;
                let root0 = hasher(vec![second_last, last])?;

                peaks_hashes
                    .into_iter()
                    .rev()
                    .try_fold(root0, |prev, cur| hasher(vec![cur, prev]))
            }
        }
    }

    pub fn calculate_root_hash(&self, elements_count: usize) -> Result<String, MMRError> {
        let bag = self.bag_the_peaks()?;

        match hasher(vec![elements_count.to_string(), bag.to_string()]) {
            Ok(root_hash) => Ok(root_hash),
            Err(_) => Err(MMRError::HashError),
        }
    }

    pub fn get_all_hashes(&self) -> Vec<(usize, String)> {
        let mut hashes: Vec<_> = self
            .hashes
            .iter()
            .map(|(&index, hash)| (index, hash.clone()))
            .collect();
        hashes.sort_by_key(|(index, _)| *index); // Sort by index
        hashes
    }

    pub fn get_many_hashes(&self, idxs: &[usize]) -> Result<Vec<String>, MMRError> {
        let mut hashes = Vec::new();
        for &idx in idxs {
            hashes.push(
                self.hashes
                    .get(&idx)
                    .cloned()
                    .ok_or(MMRError::NoHashFoundForIndex(idx))?,
            );
        }
        Ok(hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INITIAL_PEAK_VALUE: &str =
        "0x0000000000000000000000000000000000000000000000000000000000000001";
    const APPEND_VALUE: &str = "0x0000000000000000000000000000000000000000000000000000000000000002";

    fn create_test_mmr() -> GuestMMR {
        // Use a properly formatted hex string for the initial peak
        let initial_peaks = vec![INITIAL_PEAK_VALUE.to_string()];
        GuestMMR::new(initial_peaks, 1, 1)
    }

    #[test]
    fn test_new_mmr() {
        let mmr = create_test_mmr();
        assert_eq!(mmr.get_elements_count(), 1);
        assert_eq!(mmr.get_leaves_count(), 1);
    }

    #[test]
    fn test_append() {
        let mut mmr = create_test_mmr();

        let result = mmr.append(APPEND_VALUE.to_string()).unwrap();

        assert_eq!(result.leaves_count(), 2);
        assert_eq!(result.value(), APPEND_VALUE);
        assert_eq!(mmr.get_leaves_count(), 2);
    }

    #[test]
    fn test_get_proof() {
        let mut mmr = create_test_mmr();
        mmr.append(APPEND_VALUE.to_string()).unwrap();

        let proof = mmr.get_proof(1).unwrap();

        assert_eq!(proof.element_index, 1);
        assert_eq!(proof.elements_count, mmr.get_elements_count());
    }

    #[test]
    fn test_verify_proof() {
        let mut mmr = create_test_mmr();
        mmr.append(APPEND_VALUE.to_string()).unwrap();

        let proof = mmr.get_proof(1).unwrap();
        println!("proof: {:?}", proof);
        let is_valid = mmr
            .verify_proof(proof, INITIAL_PEAK_VALUE.to_string(), None)
            .unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_invalid_element_index() {
        let mmr = create_test_mmr();

        let result = mmr.get_proof(0);
        assert!(matches!(result, Err(MMRError::InvalidElementIndex)));

        let result = mmr.get_proof(999);
        assert!(matches!(result, Err(MMRError::InvalidElementIndex)));
    }

    #[test]
    fn test_bag_the_peaks() {
        let mut mmr = create_test_mmr();
        mmr.append(APPEND_VALUE.to_string()).unwrap();

        let result = mmr.bag_the_peaks();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_many_hashes() {
        let mmr = create_test_mmr();

        // Test with valid index
        let result = mmr.get_many_hashes(&[1]);
        assert!(result.is_ok());

        // Test with invalid index
        let result = mmr.get_many_hashes(&[999]);
        assert!(matches!(result, Err(MMRError::NoHashFoundForIndex(_))));
    }

    #[test]
    fn test_get_all_hashes() {
        let mut mmr = create_test_mmr();

        mmr.append(APPEND_VALUE.to_string()).unwrap();

        let hashes = mmr.get_all_hashes();

        assert!(!hashes.is_empty());
        assert_eq!(hashes[0].1, INITIAL_PEAK_VALUE);
        assert_eq!(hashes[1].1, APPEND_VALUE);
    }
}
