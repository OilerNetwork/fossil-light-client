use num_bigint::BigInt;
use num_traits::Num;
use sha2::{Digest, Sha256};
use std::str::FromStr;

use eyre::{eyre, Result};

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

pub fn leaf_count_to_peaks_count(leaf_count: usize) -> u32 {
    count_ones(leaf_count) as u32
}

pub(crate) fn count_ones(mut value: usize) -> usize {
    let mut ones_count = 0;
    while value > 0 {
        value &= value - 1;
        ones_count += 1;
    }
    ones_count
}

fn bit_length(num: usize) -> usize {
    (std::mem::size_of::<usize>() * 8) - num.leading_zeros() as usize
}

pub fn leaf_count_to_append_no_merges(leaf_count: usize) -> usize {
    if leaf_count == 0 {
        return 0;
    }
    (!leaf_count).trailing_zeros() as usize
}

pub fn hasher(data: Vec<String>) -> Result<String> {
    let mut sha2 = Sha256::new();

    //? We deliberately don't validate the size of the elements here, because we want to allow hashing of the RLP encoded block to get a block hash
    if data.is_empty() {
        sha2.update([]);
    } else if data.len() == 1 {
        let no_prefix = data[0].strip_prefix("0x").unwrap_or(&data[0]);
        sha2.update(&hex::decode(no_prefix)?);
    } else {
        let mut result: Vec<u8> = Vec::new();

        for e in data.iter() {
            let bigint = if e.starts_with("0x") || e.starts_with("0X") {
                // Parse hexadecimal
                BigInt::from_str_radix(&e[2..], 16)?
            } else {
                // Parse decimal
                BigInt::from_str(e)?
            };

            let hex = format!("{:0>64}", bigint.to_str_radix(16));
            let bytes = hex::decode(hex)?;
            result.extend(bytes);
        }

        sha2.update(&result);
    }

    let hash = sha2.finalize();
    Ok(format!("0x{:0>64}", hex::encode(hash)))
}

pub fn find_siblings(element_index: usize, elements_count: usize) -> Result<Vec<usize>> {
    let mut leaf_index = element_index_to_leaf_index(element_index)?;
    let mut height = 0;
    let mut siblings = Vec::new();
    let mut current_element_index = element_index;

    while current_element_index <= elements_count {
        let siblings_offset = (2 << height) - 1;
        if leaf_index % 2 == 1 {
            // right child
            siblings.push(current_element_index - siblings_offset);
            current_element_index += 1;
        } else {
            // left child
            siblings.push(current_element_index + siblings_offset);
            current_element_index += siblings_offset + 1;
        }
        leaf_index /= 2;
        height += 1;
    }

    siblings.pop();
    Ok(siblings)
}

pub fn element_index_to_leaf_index(element_index: usize) -> Result<usize> {
    if element_index == 0 {
        return Err(eyre!("InvalidElementIndex"));
    }
    elements_count_to_leaf_count(element_index - 1)
}

pub fn elements_count_to_leaf_count(elements_count: usize) -> Result<usize> {
    let mut leaf_count = 0;
    let mut mountain_leaf_count = 1 << bit_length(elements_count);
    let mut current_elements_count = elements_count;

    while mountain_leaf_count > 0 {
        let mountain_elements_count = 2 * mountain_leaf_count - 1;
        if mountain_elements_count <= current_elements_count {
            leaf_count += mountain_leaf_count;
            current_elements_count -= mountain_elements_count;
        }
        mountain_leaf_count >>= 1;
    }

    if current_elements_count > 0 {
        Err(eyre!("InvalidElementCount"))
    } else {
        Ok(leaf_count)
    }
}

pub fn mmr_size_to_leaf_count(mmr_size: usize) -> usize {
    let mut remaining_size = mmr_size;
    let bits = bit_length(remaining_size + 1);
    let mut mountain_tips = 1 << (bits - 1); // Using bitwise shift to calculate 2^(bits-1)
    let mut leaf_count = 0;

    while mountain_tips != 0 {
        let mountain_size = 2 * mountain_tips - 1;
        if mountain_size <= remaining_size {
            remaining_size -= mountain_size;
            leaf_count += mountain_tips;
        }
        mountain_tips >>= 1; // Using bitwise shift for division by 2
    }

    leaf_count
}

pub fn get_peak_info(mut elements_count: usize, mut element_index: usize) -> (usize, usize) {
    let mut mountain_height = bit_length(elements_count);
    let mut mountain_elements_count = (1 << mountain_height) - 1;
    let mut mountain_index = 0;

    loop {
        if mountain_elements_count <= elements_count {
            if element_index <= mountain_elements_count {
                return (mountain_index, mountain_height - 1);
            }
            elements_count -= mountain_elements_count;
            element_index -= mountain_elements_count;
            mountain_index += 1;
        }
        mountain_elements_count >>= 1;
        mountain_height -= 1;
    }
}
