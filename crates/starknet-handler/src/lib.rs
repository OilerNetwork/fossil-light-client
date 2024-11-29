#![deny(unused_crate_dependencies)]

use crypto_bigint::U256 as CryptoBigIntU256;
pub mod account;
pub mod provider;
use starknet::accounts::single_owner::SignError;
use starknet::accounts::AccountError;
use starknet::core::codec::{Decode, Encode};
use starknet::core::types::U256;
use starknet::signers::local_wallet::SignError as LocalWalletSignError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StarknetHandlerError {
    #[error("Failed to parse: {0}")]
    ParseError(#[from] url::ParseError),
    #[error("Failed to create selector: {0}")]
    SelectorError(String),
    #[error("Failed to execute transaction: {0}")]
    TransactionError(String),
    #[error("Starknet error: {0}")]
    Starknet(#[from] SignError<LocalWalletSignError>),
    #[error("Account error: {0}")]
    Account(#[from] AccountError<SignError<LocalWalletSignError>>),
    #[error("Utils error: {0}")]
    Utils(#[from] common::UtilsError),
    #[error("Encode error: {0}")]
    Encode(#[from] starknet::core::codec::Error),
    #[error("Error parsing int: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Provider error: {0}")]
    Provider(#[from] starknet::providers::ProviderError),
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct MmrState {
    latest_block_number: u64,
    root_hash: U256,
    elements_count: u64,
    leaves_count: u64,
}

impl MmrState {
    pub fn new(
        latest_block_number: u64,
        root_hash: U256,
        elements_count: u64,
        leaves_count: u64,
    ) -> Self {
        Self {
            latest_block_number,
            root_hash,
            elements_count,
            leaves_count,
        }
    }

    pub fn latest_block_number(&self) -> u64 {
        self.latest_block_number
    }

    pub fn root_hash(&self) -> U256 {
        self.root_hash
    }

    pub fn elements_count(&self) -> u64 {
        self.elements_count
    }

    pub fn leaves_count(&self) -> u64 {
        self.leaves_count
    }
}

pub fn u256_from_hex(hex: &str) -> Result<U256, StarknetHandlerError> {
    let crypto_bigint = CryptoBigIntU256::from_be_hex(hex);

    Ok(U256::from(crypto_bigint))
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
    }

    #[test]
    #[should_panic]
    fn test_u256_from_hex_invalid_input() {
        // Test invalid hex string (contains non-hex characters)
        u256_from_hex("0xghijkl").unwrap();
    }
}
