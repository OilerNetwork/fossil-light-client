use guest_types::AppendResult;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;
use num_bigint::BigInt;
use num_traits::Num;
use std::str::FromStr;

#[derive(Error, Debug)]
pub enum MMRError {
    NoHashFoundForIndex(usize),
    InsufficientPeaksForMerge,
    HashError,
    FromHexError(#[from] hex::FromHexError),
}

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
            let right_hash = peaks.pop().unwrap();
            let left_hash = peaks.pop().unwrap();

            let parent_hash = hash(vec![left_hash, right_hash])?;
            self.hashes.insert(last_element_idx, parent_hash.clone());

            peaks.push(parent_hash);
        }

        for value in self.hashes.values() {
            println!("{}", value);
        }

        self.elements_count = last_element_idx;
        self.leaves_count += 1;

        let bag = self.bag_the_peaks()?;
        let root_hash = self.calculate_root_hash(&bag, last_element_idx)?;
        self.root_hash = root_hash;

        Ok(AppendResult::new(
            self.leaves_count,
            last_element_idx,
            leaf_element_index,
            value
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

    pub fn get_all_hashes(&self) -> Vec<(usize, String)> {
        self.hashes
            .iter()
            .map(|(&index, hash)| (index, hash.clone()))
            .collect()
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
            MMRError::InsufficientPeaksForMerge => write!(f, "Insufficient peaks for merge"),
            MMRError::HashError => write!(f, "Hash error"),
            MMRError::FromHexError(e) => write!(f, "From hex error: {}", e),
        }
    }
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
    let mut sha2 = Sha256::new();

    //? We deliberately don't validate the size of the elements here, because we want to allow hashing of the RLP encoded block to get a block hash
    if data.is_empty() {
        sha2.update(&[]);
    } else if data.len() == 1 {
        let no_prefix = data[0].strip_prefix("0x").unwrap_or(&data[0]);
        sha2.update(&hex::decode(no_prefix)?);
    } else {
        let mut result: Vec<u8> = Vec::new();

        for e in data.iter() {
            let bigint = if e.starts_with("0x") || e.starts_with("0X") {
                // Parse hexadecimal
                BigInt::from_str_radix(&e[2..], 16).unwrap()
            } else {
                // Parse decimal
                BigInt::from_str(e).unwrap()
            };

            let hex = format!("{:0>64}", bigint.to_str_radix(16));
            let bytes = hex::decode(hex).unwrap();
            result.extend(bytes);
        }

        sha2.update(&result);
    }

    let hash = sha2.finalize();
    Ok(format!("0x{:0>64}", hex::encode(hash)))
}