//! Domain-specific types for the Fossil Light Client.
//!
//! This module defines strong type wrappers around primitive types to prevent
//! type confusion and improve API clarity using the newtype pattern.

use std::fmt;

/// A blockchain block number.
///
/// This newtype wrapper prevents confusion between different numeric values
/// and provides type safety for block number operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockNumber(pub u64);

impl BlockNumber {
    /// Creates a new BlockNumber.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying u64 value.
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Returns the next block number.
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    /// Returns the previous block number, or None if this is block 0.
    pub fn prev(&self) -> Option<Self> {
        if self.0 > 0 {
            Some(Self(self.0 - 1))
        } else {
            None
        }
    }

    /// Safely subtracts 1, returning 0 if this would underflow.
    pub fn saturating_sub_one(&self) -> Self {
        Self(self.0.saturating_sub(1))
    }
}

impl From<u64> for BlockNumber {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<BlockNumber> for u64 {
    fn from(block: BlockNumber) -> Self {
        block.0
    }
}

impl fmt::Display for BlockNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A blockchain chain identifier.
///
/// This newtype wrapper ensures chain IDs cannot be confused with other
/// numeric values and provides type safety for chain-specific operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChainId(pub u64);

impl ChainId {
    /// Creates a new ChainId.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying u64 value.
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Ethereum mainnet chain ID.
    pub const ETHEREUM_MAINNET: Self = Self(1);

    /// Ethereum Sepolia testnet chain ID.
    pub const ETHEREUM_SEPOLIA: Self = Self(11155111);

    /// Starknet mainnet chain ID.
    pub const STARKNET_MAINNET: Self = Self(0x534e5f4d41494e);

    /// Starknet Sepolia testnet chain ID (shortened to fit u64).
    pub const STARKNET_SEPOLIA: Self = Self(0x534e5f534550);
}

impl From<u64> for ChainId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<ChainId> for u64 {
    fn from(chain: ChainId) -> Self {
        chain.0
    }
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A blockchain address.
///
/// This newtype wrapper provides type safety for address handling and
/// validation, preventing confusion with other string values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address(String);

impl Address {
    /// Creates a new Address after validation.
    ///
    /// # Arguments
    ///
    /// * `value` - The address string to validate and wrap
    ///
    /// # Returns
    ///
    /// Returns `Some(Address)` if the address is valid, `None` otherwise.
    ///
    /// # Validation Rules
    ///
    /// * Must start with "0x"
    /// * Must have at least 3 characters total
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let addr = value.into();
        if Self::is_valid(&addr) {
            Some(Self(addr))
        } else {
            None
        }
    }

    /// Creates a new Address without validation.
    ///
    /// # Safety
    ///
    /// This method bypasses validation and should only be used when
    /// you're certain the address is valid (e.g., from trusted sources).
    pub fn new_unchecked(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Validates an address string.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address string to validate
    ///
    /// # Returns
    ///
    /// Returns `true` if the address is valid, `false` otherwise.
    pub fn is_valid(addr: &str) -> bool {
        addr.starts_with("0x") && addr.len() >= 3
    }

    /// Returns the underlying string value.
    pub fn value(&self) -> &str {
        &self.0
    }

    /// Returns the address as a hex string (same as value()).
    pub fn as_hex(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Address {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value_clone = value.clone();
        Self::new(value).ok_or_else(|| format!("Invalid address format: {}", value_clone))
    }
}

impl TryFrom<&str> for Address {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value).ok_or_else(|| format!("Invalid address format: {}", value))
    }
}

/// A batch size for processing operations.
///
/// This newtype wrapper ensures batch sizes are always positive and
/// provides type safety for batch processing operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BatchSize(u64);

impl BatchSize {
    /// Creates a new BatchSize.
    ///
    /// # Arguments
    ///
    /// * `value` - The batch size value (must be > 0)
    ///
    /// # Returns
    ///
    /// Returns `Some(BatchSize)` if the value is valid (> 0), `None` otherwise.
    pub fn new(value: u64) -> Option<Self> {
        if value > 0 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns the underlying u64 value.
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Default batch size for most operations.
    pub const DEFAULT: Self = Self(1024);

    /// Small batch size for testing or low-resource environments.
    pub const SMALL: Self = Self(100);

    /// Large batch size for high-throughput scenarios.
    pub const LARGE: Self = Self(10000);
}

impl From<BatchSize> for u64 {
    fn from(batch: BatchSize) -> Self {
        batch.0
    }
}

impl fmt::Display for BatchSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A polling interval in seconds.
///
/// This newtype wrapper ensures polling intervals are always positive and
/// provides type safety for timing operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PollingInterval(u64);

impl PollingInterval {
    /// Creates a new PollingInterval.
    ///
    /// # Arguments
    ///
    /// * `seconds` - The polling interval in seconds (must be > 0)
    ///
    /// # Returns
    ///
    /// Returns `Some(PollingInterval)` if the value is valid (> 0), `None` otherwise.
    pub fn new(seconds: u64) -> Option<Self> {
        if seconds > 0 {
            Some(Self(seconds))
        } else {
            None
        }
    }

    /// Returns the interval in seconds.
    pub fn seconds(&self) -> u64 {
        self.0
    }

    /// Returns the interval as a `Duration`.
    pub fn as_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.0)
    }

    /// Default polling interval (5 seconds).
    pub const DEFAULT: Self = Self(5);

    /// Fast polling interval (1 second).
    pub const FAST: Self = Self(1);

    /// Slow polling interval (30 seconds).
    pub const SLOW: Self = Self(30);
}

impl From<PollingInterval> for u64 {
    fn from(interval: PollingInterval) -> Self {
        interval.0
    }
}

impl fmt::Display for PollingInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_number() {
        let block = BlockNumber::new(100);
        assert_eq!(block.value(), 100);
        assert_eq!(block.next().value(), 101);
        assert_eq!(block.prev().unwrap().value(), 99);

        let zero_block = BlockNumber::new(0);
        assert!(zero_block.prev().is_none());
        assert_eq!(zero_block.saturating_sub_one().value(), 0);
    }

    #[test]
    fn test_chain_id() {
        let chain = ChainId::new(1);
        assert_eq!(chain.value(), 1);
        assert_eq!(ChainId::ETHEREUM_MAINNET.value(), 1);
    }

    #[test]
    fn test_address_validation() {
        // Valid addresses
        assert!(Address::new("0x1234567890abcdef").is_some());
        assert!(Address::new("0x0").is_some());

        // Invalid addresses
        assert!(Address::new("1234567890abcdef").is_none());
        assert!(Address::new("0x").is_none());
        assert!(Address::new("").is_none());

        // Test validation function directly
        assert!(Address::is_valid("0x123"));
        assert!(!Address::is_valid("123"));
    }

    #[test]
    fn test_batch_size() {
        // Valid batch sizes
        assert!(BatchSize::new(1).is_some());
        assert!(BatchSize::new(1000).is_some());

        // Invalid batch size
        assert!(BatchSize::new(0).is_none());

        assert_eq!(BatchSize::DEFAULT.value(), 1024);
    }

    #[test]
    fn test_polling_interval() {
        // Valid intervals
        assert!(PollingInterval::new(1).is_some());
        assert!(PollingInterval::new(60).is_some());

        // Invalid interval
        assert!(PollingInterval::new(0).is_none());

        let interval = PollingInterval::new(5).unwrap();
        assert_eq!(interval.seconds(), 5);
        assert_eq!(interval.as_duration().as_secs(), 5);
    }

    #[test]
    fn test_conversions() {
        let block = BlockNumber::from(42u64);
        let value: u64 = block.into();
        assert_eq!(value, 42);

        let chain = ChainId::from(1u64);
        let value: u64 = chain.into();
        assert_eq!(value, 1);
    }
}
