use common::felt;
use guest_types::{AppendResult, PeaksFormattingOptions, PeaksOptions};
use serde::{Deserialize, Serialize};
use starknet_crypto::{poseidon_hash, poseidon_hash_many, poseidon_hash_single, Felt};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FormattingError {
    #[error("Formatting: Expected peaks output size is smaller than the actual size")]
    PeaksOutputSizeError,
}

#[derive(Error, Debug)]
pub enum MMRError {
    NoHashFoundForIndex(usize),
    Formatting(FormattingError),
    InsufficientPeaksForMerge,
    HashError,
}

pub struct GuestMMR {
    hashes: HashMap<usize, String>,
    elements_count: usize,
    leaves_count: usize,
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

        peaks.push(value);

        let no_merges = leaf_count_to_append_no_merges(self.leaves_count);

        for _ in 0..no_merges {
            if peaks.len() < 2 {
                return Err(MMRError::InsufficientPeaksForMerge);
            }

            last_element_idx += 1;

            // Pop the last two peaks to merge
            let right_hash = peaks.pop().unwrap();
            let left_hash = peaks.pop().unwrap();

            let parent_hash = hash(vec![left_hash, right_hash])?;
            self.hashes.insert(last_element_idx, parent_hash.clone());

            peaks.push(parent_hash);
        }

        self.elements_count = last_element_idx;
        self.leaves_count += 1;

        let bag = self.bag_the_peaks()?;
        let root_hash = self.calculate_root_hash(&bag, last_element_idx)?;

        Ok(AppendResult::new(
            self.leaves_count,
            last_element_idx,
            leaf_element_index,
            root_hash,
        ))
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

    fn bag_the_peaks(&self) -> Result<String, MMRError> {
        let peaks_idxs = find_peaks(self.elements_count);

        let peaks_hashes = self.retrieve_peaks_hashes(peaks_idxs)?;

        match peaks_hashes.len() {
            0 => Ok("0x0".to_string()),
            1 => Ok(peaks_hashes[0].clone()),
            _ => {
                let mut peaks_hashes: VecDeque<String> = peaks_hashes.into();
                let last = peaks_hashes.pop_back().unwrap();
                let second_last = peaks_hashes.pop_back().unwrap();
                let root0 = hash(vec![second_last, last])?;

                let final_root = peaks_hashes
                    .into_iter()
                    .rev()
                    .fold(root0, |prev: String, cur: String| {
                        hash(vec![cur, prev]).unwrap()
                    });

                Ok(final_root)
            }
        }
    }

    pub fn calculate_root_hash(
        &self,
        bag: &str,
        elements_count: usize,
    ) -> Result<String, MMRError> {
        match hash(vec![elements_count.to_string(), bag.to_string()]) {
            Ok(root_hash) => Ok(root_hash),
            Err(_) => Err(MMRError::HashError),
        }
    }

    pub fn get_peaks(&self, option: PeaksOptions) -> Result<Vec<String>, MMRError> {
        let tree_size = match option.elements_count {
            Some(count) => count,
            None => self.elements_count,
        };

        let peaks_indices = find_peaks(tree_size);
        let peaks = self.retrieve_peaks_hashes(peaks_indices)?;

        if let Some(formatting_opts) = option.formatting_opts {
            match format_peaks(peaks, &formatting_opts) {
                Ok(formatted_peaks) => Ok(formatted_peaks),
                Err(e) => Err(MMRError::Formatting(e)),
            }
        } else {
            Ok(peaks)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Proof {
    element_index: usize,
    element_hash: String,
    siblings_hashes: Vec<String>,
    peaks_hashes: Vec<String>,
    elements_count: usize,
}

impl std::fmt::Display for MMRError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MMRError::NoHashFoundForIndex(idx) => write!(f, "No hash found for index {}", idx),
            MMRError::Formatting(e) => write!(f, "Formatting error: {}", e),
            MMRError::InsufficientPeaksForMerge => write!(f, "Insufficient peaks for merge"),
            MMRError::HashError => write!(f, "Hash error"),
        }
    }
}

pub fn format_peaks(
    mut peaks: Vec<String>,
    formatting_opts: &PeaksFormattingOptions,
) -> Result<Vec<String>, FormattingError> {
    if peaks.len() > formatting_opts.output_size {
        return Err(FormattingError::PeaksOutputSizeError);
    }

    let expected_peaks_size_remainder = formatting_opts.output_size - peaks.len();
    let peaks_null_values: Vec<String> =
        vec![formatting_opts.null_value.clone(); expected_peaks_size_remainder];

    peaks.extend(peaks_null_values);

    Ok(peaks)
}

// Add this function at the bottom with other helper functions
pub fn find_peaks(mut elements_count: usize) -> Vec<usize> {
    let mut mountain_elements_count = (1 << bit_length(elements_count)) - 1;
    let mut mountain_index_shift = 0;
    let mut peaks = Vec::new();

    while mountain_elements_count > 0 {
        if mountain_elements_count <= elements_count {
            mountain_index_shift += mountain_elements_count;
            peaks.push(mountain_index_shift);
            elements_count -= mountain_elements_count;
        }
        mountain_elements_count >>= 1;
    }

    if elements_count > 0 {
        return Vec::new();
    }

    peaks
}

fn bit_length(num: usize) -> usize {
    (std::mem::size_of::<usize>() * 8) - num.leading_zeros() as usize
}

fn leaf_count_to_append_no_merges(leaf_count: usize) -> usize {
    if leaf_count == 0 {
        return 0;
    }
    (!leaf_count).trailing_zeros() as usize
}

fn hash(data: Vec<String>) -> Result<String, MMRError> {
    // for element in &data {
    //     self.is_element_size_valid(element)?;
    // }

    let field_elements: Vec<Felt> = data.iter().map(|e| felt(e).unwrap_or_default()).collect();

    let hash_core = match field_elements.len() {
        0 => return Err(MMRError::HashError),
        1 => poseidon_hash_single(field_elements[0]),
        2 => poseidon_hash(field_elements[0], field_elements[1]),
        _ => poseidon_hash_many(&field_elements),
    };

    let hash = format!("{:x}", hash_core);
    // if self.should_pad {
    //     hash = format!("{:0>63}", hash);
    // }
    let hash = format!("0x{}", hash);
    Ok(hash)
}
