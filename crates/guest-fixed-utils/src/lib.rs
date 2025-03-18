#![allow(unused_crate_dependencies)]

#[cfg(test)]
pub mod avg_fees_rounding_analysis;

use core::convert::TryFrom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U256 {
    pub high: u128,
    pub low: u128,
}

impl U256 {
    pub fn new(high: u128, low: u128) -> Self {
        Self { high, low }
    }

    pub fn to_be_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];

        let high_bytes = self.high.to_be_bytes();
        let low_bytes = self.low.to_be_bytes();

        bytes[0..16].copy_from_slice(&high_bytes);
        bytes[16..32].copy_from_slice(&low_bytes);

        bytes
    }

    pub fn get_high(&self) -> u128 {
        self.high
    }

    pub fn get_low(&self) -> u128 {
        self.low
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Felt {
    // Internal representation as bytes
    bytes: [u8; 32],
}

impl Felt {
    pub fn from_bytes_be(bytes: &[u8; 32]) -> Self {
        Self { bytes: *bytes }
    }

    pub fn to_bytes_be(&self) -> [u8; 32] {
        self.bytes
    }

    /// Creates a Felt from a hexadecimal string representation
    /// The string should be prefixed with "0x" and can represent fewer than 32 bytes
    /// (in which case it will be padded with leading zeros)
    pub fn from_hex_string(hex_str: &str) -> Result<Self, &'static str> {
        // Validate the hex string format
        if !hex_str.starts_with("0x") {
            return Err("Hex string must start with 0x");
        }

        // Remove the 0x prefix
        let hex_str = &hex_str[2..];

        // Create a buffer for the bytes (32 bytes = 64 hex chars)
        let mut bytes = [0u8; 32];

        // Calculate how many bytes the hex string represents
        let hex_len = hex_str.len();
        if hex_len > 64 {
            return Err("Hex string too long, must represent at most 32 bytes");
        }

        // Calculate padding needed
        let padding = 64 - hex_len;

        // Convert hex string to bytes with proper padding
        for (i, c) in hex_str.chars().enumerate() {
            let byte_pos = (i + padding) / 2;
            let nibble = match c.to_digit(16) {
                Some(d) => d as u8,
                None => return Err("Invalid hex character"),
            };

            if (i + padding) % 2 == 0 {
                // High nibble
                bytes[byte_pos] = nibble << 4;
            } else {
                // Low nibble
                bytes[byte_pos] |= nibble;
            }
        }

        Ok(Self { bytes })
    }

    /// Converts the Felt to a decimal string representation
    pub fn to_dec_string(&self) -> String {
        // Convert to U256 first
        let u256: U256 = self.clone().into();

        // Handle zero case
        if u256.high == 0 && u256.low == 0 {
            return "0".to_string();
        }

        // Convert to decimal string using base-10 division
        let mut result = String::new();
        let mut remaining = u256;
        let ten = U256::new(0, 10);

        while remaining.high != 0 || remaining.low != 0 {
            // Divide by 10 and get remainder
            let (quotient, remainder) = div_mod_u256(&remaining, &ten);

            // Add digit to result
            result.push(char::from_digit(remainder.low as u32, 10).unwrap());

            // Continue with quotient
            remaining = quotient;
        }

        // Reverse the string since we built it in reverse order
        result.chars().rev().collect()
    }

    /// Converts the Felt to a hexadecimal string representation with 0x prefix
    /// Returns a fixed-length string with 0x followed by 64 hex characters (32 bytes)
    pub fn to_hex_string(&self) -> String {
        let bytes = self.to_bytes_be();

        // Create a fixed-length hex string with 0x prefix (total 66 characters)
        let mut result = String::with_capacity(66); // 2 for "0x" + 64 for the hex digits
        result.push_str("0x");

        // Always include all bytes, including leading zeros
        for &byte in &bytes {
            result.push_str(&format!("{:02x}", byte));
        }

        result
    }
}

/// Helper function to divide a U256 by another U256 and return quotient and remainder
fn div_mod_u256(a: &U256, b: &U256) -> (U256, U256) {
    // Simple case: if divisor is larger than dividend
    if (a.high < b.high) || (a.high == b.high && a.low < b.low) {
        return (U256::new(0, 0), *a);
    }

    // Simple case: if both high parts are 0, just divide the low parts
    if a.high == 0 && b.high == 0 {
        return (U256::new(0, a.low / b.low), U256::new(0, a.low % b.low));
    }

    // For more complex cases, use a simple long division algorithm
    let mut quotient = U256::new(0, 0);
    let mut remainder = U256::new(0, 0);

    // Process 256 bits from most significant to least significant
    for i in (0..256).rev() {
        // Shift remainder left by 1 bit
        remainder = U256::new(
            (remainder.high << 1) | (remainder.low >> 127),
            remainder.low << 1,
        );

        // Get the current bit from dividend
        let bit = if i >= 128 {
            (a.high >> (i - 128)) & 1
        } else {
            (a.low >> i) & 1
        };

        // Add current bit to remainder
        if bit == 1 {
            remainder.low |= 1;
        }

        // If remainder >= divisor, subtract divisor and set quotient bit
        if (remainder.high > b.high) || (remainder.high == b.high && remainder.low >= b.low) {
            // remainder = remainder - divisor
            if remainder.low < b.low {
                remainder.high -= 1;
            }
            remainder.low = remainder.low.wrapping_sub(b.low);
            remainder.high = remainder.high.wrapping_sub(b.high);

            // Set quotient bit
            if i >= 128 {
                quotient.high |= 1 << (i - 128);
            } else {
                quotient.low |= 1 << i;
            }
        }
    }

    (quotient, remainder)
}

impl From<Felt> for U256 {
    fn from(value: Felt) -> Self {
        let bytes = value.to_bytes_be();

        let mut high_bytes = [0u8; 16];
        let mut low_bytes = [0u8; 16];

        high_bytes.copy_from_slice(&bytes[0..16]);
        low_bytes.copy_from_slice(&bytes[16..32]);

        let high = u128::from_be_bytes(high_bytes);
        let low = u128::from_be_bytes(low_bytes);

        U256::new(high, low)
    }
}

impl From<U256> for Felt {
    fn from(value: U256) -> Self {
        Felt::from_bytes_be(&value.to_be_bytes())
    }
}

impl TryFrom<&str> for Felt {
    type Error = &'static str;

    fn try_from(hex_str: &str) -> Result<Self, Self::Error> {
        Self::from_hex_string(hex_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UFixedPoint123x128 {
    value: U256,
}

impl UFixedPoint123x128 {
    pub fn new(high: u128, low: u128) -> Self {
        Self {
            value: U256::new(high, low),
        }
    }

    pub fn get_integer(&self) -> u128 {
        self.value.high
    }

    pub fn get_fractional(&self) -> u128 {
        self.value.low
    }
}

impl TryFrom<UFixedPoint123x128> for Felt {
    type Error = &'static str;
    fn try_from(fp: UFixedPoint123x128) -> Result<Self, Self::Error> {
        // Bound check: the value must fit in a 252-bit field element.
        const FELT252_PRIME_HIGH: u128 = 0x8000000000000110000000000000000;
        if fp.value.high > FELT252_PRIME_HIGH
            || (fp.value.high == FELT252_PRIME_HIGH && fp.value.low != 0)
        {
            return Err("FELT_OVERFLOW");
        }
        let bytes = fp.value.to_be_bytes();
        Ok(Felt::from_bytes_be(&bytes))
    }
}

impl From<Felt> for UFixedPoint123x128 {
    fn from(f: Felt) -> Self {
        let u256_val: U256 = f.into();
        UFixedPoint123x128 { value: u256_val }
    }
}

impl From<f64> for UFixedPoint123x128 {
    fn from(value: f64) -> Self {
        // Split into integer and fractional parts
        let integer_part = value.trunc() as u128;
        let fractional_part = value.fract();

        // Convert fractional part to fixed-point representation
        // Multiply by 2^128 and truncate to get the lower bits
        let fractional_bits = (fractional_part * 2f64.powi(64) * 2f64.powi(64)).trunc() as u128;

        // Combine integer and fractional parts
        UFixedPoint123x128 {
            value: U256::new(integer_part, fractional_bits),
        }
    }
}

impl From<UFixedPoint123x128> for f64 {
    fn from(fp: UFixedPoint123x128) -> Self {
        // Extract integer and fractional parts
        let integer_part = fp.value.high as f64;

        // Convert fixed-point fractional part back to float
        // Divide by 2^128 to get the fractional value
        let fractional_part = fp.value.low as f64 / 2f64.powi(64) / 2f64.powi(64);

        // Combine the parts
        integer_part + fractional_part
    }
}

pub trait StorePacking<T, F> {
    fn pack(value: T) -> F;
    fn unpack(f: F) -> T;
}

impl StorePacking<UFixedPoint123x128, Felt> for UFixedPoint123x128 {
    fn pack(value: UFixedPoint123x128) -> Felt {
        // Make sure the value fits within a Felt
        // A Felt is 251 bits, so we need to ensure our value is within that range
        if value.value.high >= (1u128 << 123) {
            panic!("Value too large to pack into Felt");
        }

        // Convert to bytes and then to Felt
        let bytes = value.value.to_be_bytes();

        // Create Felt from the bytes
        Felt::from_bytes_be(&bytes)
    }

    fn unpack(felt: Felt) -> UFixedPoint123x128 {
        // Convert Felt to U256 and create UFixedPoint123x128
        let u256_val: U256 = felt.into();
        UFixedPoint123x128 { value: u256_val }
    }
}

impl std::ops::Div for UFixedPoint123x128 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        // Ensure we're not dividing by zero
        assert!(
            rhs.value.high != 0 || rhs.value.low != 0,
            "Division by zero"
        );

        // Convert to f64 for the division operation
        // This is a simplification that works for testing but has precision limitations
        let self_f64 = self.value.high as f64 + (self.value.low as f64 / 2f64.powi(128));
        let rhs_f64 = rhs.value.high as f64 + (rhs.value.low as f64 / 2f64.powi(128));

        let result = self_f64 / rhs_f64;
        result.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_felt_to_hex_string() {
        // Test zero
        let zero = Felt::from_bytes_be(&[0; 32]);
        assert_eq!(
            zero.to_hex_string(),
            "0x0000000000000000000000000000000000000000000000000000000000000000"
        );

        // Test small number
        let one = Felt::from_bytes_be(&{
            let mut bytes = [0; 32];
            bytes[31] = 1;
            bytes
        });
        assert_eq!(
            one.to_hex_string(),
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        );

        // Test larger number
        let large = Felt::from_bytes_be(&{
            let mut bytes = [0; 32];
            bytes[30] = 0xAB;
            bytes[31] = 0xCD;
            bytes
        });
        assert_eq!(
            large.to_hex_string(),
            "0x000000000000000000000000000000000000000000000000000000000000abcd"
        );
    }

    #[test]
    fn test_felt_to_dec_string() {
        // Test zero
        let zero = Felt::from_bytes_be(&[0; 32]);
        assert_eq!(zero.to_dec_string(), "0");

        // Test small numbers
        let one = Felt::from_bytes_be(&{
            let mut bytes = [0; 32];
            bytes[31] = 1;
            bytes
        });
        assert_eq!(one.to_dec_string(), "1");

        let ten = Felt::from_bytes_be(&{
            let mut bytes = [0; 32];
            bytes[31] = 10;
            bytes
        });
        assert_eq!(ten.to_dec_string(), "10");

        // Test larger number
        let large = Felt::from_bytes_be(&{
            let mut bytes = [0; 32];
            bytes[30] = 0xAB;
            bytes[31] = 0xCD;
            bytes
        });
        assert_eq!(large.to_dec_string(), "43981");
    }

    fn example_fp() -> UFixedPoint123x128 {
        UFixedPoint123x128::from(5.0 / 3.0)
    }

    #[test]
    fn test_pack_unpack() {
        let fp = example_fp();
        println!("(5/3)_u256 = {:?}", fp);
        let felt = UFixedPoint123x128::pack(fp.clone());
        println!("Packed Felt: {:?}", felt);
        let unpacked = UFixedPoint123x128::unpack(felt);
        // rust unpacked:   567137278201564130958245587691054825472
        // cairo unpacked:  567137278201564105772291012386280352426
        // This difference is well below the precision limit of 64-bit floating point
        // (which is about 10^-16)
        println!("Unpacked UFixedPoint123x128: {:?}", unpacked);
        assert_eq!(fp, unpacked);
    }

    #[test]
    fn test_conversion_integer() {
        let value = 1.0_f64;
        println!(
            "Converting {} -> {:?}",
            value,
            UFixedPoint123x128::from(value)
        );

        // For integer 1, high should be 1 and low should be 0
        let fp = UFixedPoint123x128::from(value);
        assert_eq!(fp.value.high, 1);
        assert_eq!(fp.value.low, 0);
    }

    #[test]
    fn test_conversion_half() {
        let value = 1.5_f64;
        println!(
            "Converting {} -> {:?}",
            value,
            UFixedPoint123x128::from(value)
        );

        // For 1.5, high should be 1 and low should be 2^127
        let fp = UFixedPoint123x128::from(value);
        assert_eq!(fp.value.high, 1);
        assert_eq!(fp.value.low, 1u128 << 127);
    }

    #[test]
    fn test_conversion_pi() {
        let value = std::f64::consts::PI;
        println!(
            "Converting {} -> {:?}",
            value,
            UFixedPoint123x128::from(value)
        );

        // For π ≈ 3.14159..., high should be 3 and low should be approximately 0.14159... * 2^128
        let fp = UFixedPoint123x128::from(value);
        assert_eq!(fp.value.high, 3);

        // Use a different approach to calculate expected_low to avoid overflow
        let expected_low =
            (0.14159265358979323846 * (1u128 << 64) as f64 * (1u128 << 64) as f64) as u128;
        let tolerance = 1u128 << 120; // Allow some error margin
        assert!((fp.value.low as i128 - expected_low as i128).abs() < tolerance as i128);
    }

    #[test]
    fn test_conversion_five_thirds() {
        let value = 5.0 / 3.0;
        println!(
            "Converting {} -> {:?}",
            value,
            UFixedPoint123x128::from(value)
        );

        // For 5/3 ≈ 1.6666..., high should be 1 and low should be approximately 0.6666... * 2^128
        let fp = UFixedPoint123x128::from(value);
        assert_eq!(fp.value.high, 1);

        // Get the actual value from the implementation for comparison
        let actual_low = fp.value.low;

        // Print the actual value for debugging
        println!("Actual low bits: {}", actual_low);

        // Calculate bounds for the expected value
        let lower_bound =
            ((2.0 / 3.0 - 0.0001) * (1u128 << 64) as f64 * (1u128 << 64) as f64) as u128;
        let upper_bound =
            ((2.0 / 3.0 + 0.0001) * (1u128 << 64) as f64 * (1u128 << 64) as f64) as u128;

        assert!(actual_low > lower_bound);
        assert!(actual_low < upper_bound);
    }

    #[test]
    fn test_conversion_to_f64() {
        // Test converting several values from f64 to UFixedPoint123x128 and back to f64
        let test_values = [0.0, 1.0, 1.5, std::f64::consts::PI, 5.0 / 3.0, 123.456];

        for original in test_values.iter() {
            // Convert to fixed point
            let fixed = UFixedPoint123x128::from(*original);

            // Convert back to f64
            let converted: f64 = fixed.into();

            println!(
                "Original: {}, Converted: {}, Diff: {}",
                original,
                converted,
                (original - converted).abs()
            );

            // Verify the conversion maintains reasonable precision
            // Allow small error due to floating point precision limitations
            assert!((original - converted).abs() < 1e-10);
        }
    }
}
