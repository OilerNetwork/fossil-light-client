#![deny(unused_crate_dependencies)]

use crypto_bigint::U256 as CryptoBigIntU256;
pub mod account;
pub mod provider;
use eyre::{eyre, Result};
use starknet::core::codec::{Decode, Encode};
use starknet::core::types::{ByteArray, U256};
use tracing::{debug, instrument};

#[derive(Clone, Debug, Encode, Decode)]
pub struct MmrSnapshot {
    batch_index: u64,
    latest_mmr_block: u64,
    latest_mmr_block_hash: U256,
    root_hash: U256,
    leaves_count: u64,
    ipfs_hash: ByteArray,
}

impl MmrSnapshot {
    pub fn latest_mmr_block(&self) -> u64 {
        self.latest_mmr_block
    }

    pub fn latest_mmr_block_hash(&self) -> U256 {
        self.latest_mmr_block_hash
    }

    pub fn root_hash(&self) -> U256 {
        self.root_hash
    }

    pub fn leaves_count(&self) -> u64 {
        self.leaves_count
    }

    pub fn ipfs_hash(&self) -> ByteArray {
        self.ipfs_hash.clone()
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct MmrState {
    latest_mmr_block: u64,
    latest_mmr_block_hash: U256,
    root_hash: U256,
    leaves_count: u64,
    ipfs_hash: Option<ByteArray>,
}

impl MmrState {
    #[instrument(skip(root_hash), level = "debug")]
    pub fn new(
        latest_mmr_block: u64,
        latest_mmr_block_hash: U256,
        root_hash: U256,
        leaves_count: u64,
        ipfs_hash: Option<ByteArray>,
    ) -> Self {
        debug!(latest_mmr_block, leaves_count, "Creating new MMR state");
        Self {
            latest_mmr_block,
            latest_mmr_block_hash,
            root_hash,
            leaves_count,
            ipfs_hash,
        }
    }

    pub fn latest_mmr_block(&self) -> u64 {
        self.latest_mmr_block
    }

    pub fn latest_mmr_block_hash(&self) -> U256 {
        self.latest_mmr_block_hash
    }

    pub fn root_hash(&self) -> U256 {
        self.root_hash
    }

    // pub fn elements_count(&self) -> u64 {
    //     self.elements_count
    // }

    pub fn leaves_count(&self) -> u64 {
        self.leaves_count
    }

    pub fn ipfs_hash(&self) -> Option<ByteArray> {
        self.ipfs_hash.clone()
    }
}

#[instrument(level = "debug")]
pub fn u256_from_hex(hex: &str) -> Result<U256> {
    let hex_clean = hex.strip_prefix("0x").unwrap_or(hex);

    // Validate hex string length
    if hex_clean.len() != 64 {
        return Err(eyre!("Invalid hex string length: {}", hex_clean.len()));
    }

    // Validate hex characters
    if !hex_clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(eyre!("Invalid hex characters: {}", hex_clean));
    }

    let crypto_bigint = CryptoBigIntU256::from_be_hex(hex_clean);
    let result = U256::from(crypto_bigint);

    debug!(result = ?result, "Hex conversion completed");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_from_hex() {
        // Test valid hex string
        let result =
            u256_from_hex("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
                .unwrap();
        assert_eq!(
            result.to_string(),
            "77814517325470205911140941194401928579557062014761831930645393041380819009408"
        );

        // Test max value
        let result =
            u256_from_hex("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
                .unwrap();
        assert_eq!(
            result.to_string(),
            "115792089237316195423570985008687907853269984665640564039457584007913129639935"
        );

        // Test with "0x" prefix
        let result =
            u256_from_hex("0x0000000000000000000000000000000000000000000000000000000000001234")
                .unwrap();
        assert_eq!(result.to_string(), "4660");

        // Test with zero
        let result =
            u256_from_hex("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        assert_eq!(result.to_string(), "0");

        // Test with leading zeros
        let result =
            u256_from_hex("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap();
        assert_eq!(result.to_string(), "1");
    }

    #[test]
    fn test_mmr_state() {
        let block = 100u64;
        let block_hash =
            u256_from_hex("0000000000000000000000000000000000000000000000000000000000001234")
                .unwrap();
        let root =
            u256_from_hex("0000000000000000000000000000000000000000000000000000000000009876")
                .unwrap();
        let leaves = 50u64;
        let ipfs = Some(ByteArray::from("0x1234"));

        let state = MmrState::new(block, block_hash, root, leaves, ipfs.clone());

        assert_eq!(state.latest_mmr_block(), block);
        assert_eq!(state.latest_mmr_block_hash(), block_hash);
        assert_eq!(state.root_hash(), root);
        assert_eq!(state.leaves_count(), leaves);
        assert_eq!(state.ipfs_hash(), ipfs);
    }

    #[test]
    fn test_mmr_snapshot() {
        let snapshot = MmrSnapshot {
            batch_index: 1,
            latest_mmr_block: 100,
            latest_mmr_block_hash: u256_from_hex(
                "0000000000000000000000000000000000000000000000000000000000001234",
            )
            .unwrap(),
            root_hash: u256_from_hex(
                "0000000000000000000000000000000000000000000000000000000000009876",
            )
            .unwrap(),
            leaves_count: 50,
            ipfs_hash: ByteArray::from("0x1234"),
        };

        assert_eq!(
            snapshot.root_hash(),
            u256_from_hex("0000000000000000000000000000000000000000000000000000000000009876")
                .unwrap()
        );
        assert_eq!(snapshot.ipfs_hash(), ByteArray::from("0x1234"));
    }

    #[test]
    fn test_u256_from_hex_error_cases() {
        // Test invalid hex string (wrong length)
        assert!(u256_from_hex("123").is_err());

        // Test empty string
        assert!(u256_from_hex("").is_err());

        // Test invalid hex characters
        assert!(u256_from_hex("0xghijkl").is_err());
    }
}
