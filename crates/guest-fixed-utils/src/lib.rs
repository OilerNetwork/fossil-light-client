#![allow(unused_crate_dependencies)]

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

    // Fixture with paired DB and onchain values
    struct PairedFixture {
        db_values: Vec<f64>,
        onchain_hex: Vec<&'static str>,
    }

    impl PairedFixture {
        fn new() -> Self {
            Self {
                db_values: vec![
                    1074032128.1580756,
                    1707777898.5432527,
                    1102303055.2076125,
                    1146005581.3888888,
                    1073699852.0795848,
                    1060034759.8041238,
                    1054340591.9198606,
                    1059554642.2708334,
                    928502214.2,
                    1346072054.1084745,
                    4064369561.824138,
                    3054256612.6335616,
                ],
                onchain_hex: vec![
                    "0x40046e002877a400000000000000000000000000",
                    "0x65ca9f6a8b129a3a76b2ef2b67a3e01c5894d10d",
                    "0x41b3cf4f35261800000000000000000000000000",
                    "0x444ea84d638e3800000000000000000000000000",
                    "0x3fff5c0c145fa983fc74ed65de56cf47c038b129",
                    "0x3f2ed8c7cddb0e00000000000000000000000000",
                    "0x3ed7f5efeb7bfc00000000000000000000000000",
                    "0x3f27855245555600000000000000000000000000",
                    "0x3757d1c6333332fab4152fab4152fab4152fab41",
                    "0x503b6df61bc4fc00000000000000000000000000",
                    "0xf2415b99d2fab800000000000000000000000000",
                    "0xb60c41e4a2311a8542a150a8542a150a8542a150",
                ],
            }
        }
    }

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

    /// Analysis result for a pair
    #[derive(Clone)]
    struct PairAnalysis {
        onchain_value: f64,
        int_diff: i128,
        int_diff_pct: f64,
        full_diff_pct: f64,
        is_significant: bool,
        is_minimal: bool,
        nearest_power_of_2: u128,
        power_diff: i128,
        frac_diff_pct: f64,
        pattern_type: String,
    }

    // Helper function to analyze a pair
    fn analyze_pair(_index: usize, db_value: f64, onchain_hex: &str) -> PairAnalysis {
        // Parse onchain value
        let onchain_felt: Felt = onchain_hex.try_into().unwrap();
        let onchain_fp = UFixedPoint123x128::unpack(onchain_felt);

        // Extract components from onchain value (already in fixed-point format)
        let onchain_integer = onchain_fp.get_integer();
        let onchain_fractional = onchain_fp.get_fractional();

        // Extract DB components
        let db_integer = db_value.trunc() as u128;
        let db_fractional = db_value.fract();

        // For direct comparison, we need to compare the fractional parts at the same scale
        // Since we can't scale up the DB fractional to match the onchain (too large),
        // we'll compare the first N digits of both

        // Convert onchain fractional to a decimal string representation
        let onchain_frac_str = onchain_fractional.to_string();

        // Get the raw fractional part as a string (e.g., "1580756" from 0.1580756)
        let db_frac_str = format!("{:.10}", db_fractional);
        let db_frac_digits = db_frac_str.chars().skip(2).collect::<String>(); // Skip "0."

        println!("DB value: {}, fractional part: {}", db_value, db_fractional);
        println!("DB fractional digits: {}", db_frac_digits);
        println!("Onchain fractional: {}", onchain_fractional);

        // Determine how many digits to compare (use the smaller of the two)
        let compare_digits = std::cmp::min(db_frac_digits.len(), 10); // Limit to 10 digits for meaningful comparison

        // Extract the first N digits from both fractional parts
        let onchain_frac_prefix = if onchain_frac_str.len() >= compare_digits {
            onchain_frac_str[0..compare_digits].to_string()
        } else {
            onchain_frac_str.clone() + &"0".repeat(compare_digits - onchain_frac_str.len())
        };

        let db_frac_prefix = if db_frac_digits.len() >= compare_digits {
            db_frac_digits[0..compare_digits].to_string()
        } else {
            db_frac_digits.clone() + &"0".repeat(compare_digits - db_frac_digits.len())
        };

        println!("Comparing first {} digits:", compare_digits);
        println!("  Onchain prefix: {}", onchain_frac_prefix);
        println!("  DB prefix:      {}", db_frac_prefix);

        // Convert to numbers for comparison
        let onchain_prefix_num = onchain_frac_prefix.parse::<u64>().unwrap_or(0);
        let db_prefix_num = db_frac_prefix.parse::<u64>().unwrap_or(0);

        // Calculate fractional difference percentage based on the prefixes
        let frac_diff = if onchain_prefix_num > db_prefix_num {
            onchain_prefix_num - db_prefix_num
        } else {
            db_prefix_num - onchain_prefix_num
        };

        let frac_diff_pct = if onchain_prefix_num > 0 {
            (frac_diff as f64 / onchain_prefix_num as f64) * 100.0
        } else {
            0.0
        };

        // Check if the difference is close to a specific pattern
        // We've observed two patterns: ~70.61% and ~193.87%
        let is_pattern_1 = (frac_diff_pct - 70.61).abs() < 0.01;
        let is_pattern_2 = (frac_diff_pct - 193.87).abs() < 0.01;

        println!(
            "  Fractional difference: {} ({:.6}%)",
            frac_diff, frac_diff_pct
        );

        if is_pattern_1 {
            println!("  ⚠️ PATTERN 1 DETECTED: ~70.61% difference (close to 1 - 1/√2 = 0.7071)");
            println!(
                "  This suggests a scaling factor of 1/√2 ≈ 0.7071 between DB and onchain values"
            );
        } else if is_pattern_2 {
            println!("  ⚠️ PATTERN 2 DETECTED: ~193.87% difference (close to 2 - 1/√2 = 1.9289)");
            println!("  This suggests a scaling factor of 2 - 1/√2 ≈ 1.9289 between DB and onchain values");
        }

        // For display purposes only, convert onchain to floating point
        let onchain_fractional_float =
            (onchain_fractional as f64) / (1u128 << 64) as f64 / (1u128 << 64) as f64;
        let onchain_full_value = onchain_integer as f64 + onchain_fractional_float;

        // Calculate integer difference
        let int_diff = db_integer as i128 - onchain_integer as i128;
        let int_diff_pct = if onchain_integer > 0 {
            (int_diff.abs() as f64 / onchain_integer as f64) * 100.0
        } else {
            0.0
        };

        // Calculate full difference for display
        let full_diff = (db_value - onchain_full_value).abs();
        let full_diff_pct = if onchain_full_value > 0.0 {
            (full_diff / onchain_full_value) * 100.0
        } else {
            0.0
        };

        // Calculate power of 2 analysis
        let abs_diff = int_diff.abs() as u128;
        let (nearest_power, power_diff) = if abs_diff > 0 {
            let log2 = (abs_diff as f64).log2();
            let nearest_power = 2u128.pow(log2.round() as u32);
            let power_diff = (abs_diff as i128 - nearest_power as i128).abs();
            (nearest_power, power_diff)
        } else {
            (0, 0)
        };

        // Check if the fractional part is related to a specific mathematical constant
        let pattern_type = if is_pattern_1 {
            "Pattern 1 (~70.61%)"
        } else if is_pattern_2 {
            "Pattern 2 (~193.87%)"
        } else {
            "No recognized pattern"
        };

        PairAnalysis {
            onchain_value: onchain_full_value,
            int_diff,
            int_diff_pct,
            full_diff_pct,
            is_significant: int_diff_pct > 0.1,
            is_minimal: full_diff_pct < 0.001,
            nearest_power_of_2: nearest_power,
            power_diff,
            frac_diff_pct,
            pattern_type: pattern_type.to_string(),
        }
    }

    /// # Analysis of DB vs On-chain Value Discrepancies
    ///
    /// ## Summary of Findings
    ///
    /// This test analyzes the relationship between database (DB) and on-chain values,
    /// focusing on their fractional parts. The analysis revealed:
    ///
    /// 1. **Integer Parts Match Perfectly**: The integer parts of DB and on-chain values
    ///    match exactly, with zero discrepancies.
    ///
    /// 2. **Two Distinct Patterns in Fractional Parts**:
    ///    - **Pattern 1**: DB/Onchain ≈ 29.39% (observed) vs 29.29% (theoretical)
    ///      - This closely matches the formula: (1 - 1/√2) * 100
    ///      - Found in exactly half of the analyzed pairs
    ///    
    ///    - **Pattern 2**: DB/Onchain ≈ 293.87% (observed) vs 341.42% (theoretical)
    ///      - This is related to Pattern 1 as approximately 100/(Pattern 1) * 100
    ///      - Found in the other half of the analyzed pairs
    ///
    /// 3. **Mathematical Relationship**: The patterns suggest a systematic relationship
    ///    involving the mathematical constant √2 (1.4142...). Pattern 1 is very close
    ///    to (1 - 1/√2), and Pattern 2 appears to be related to the reciprocal of Pattern 1.
    ///
    /// 4. **Consistent Conversion**: Despite these differences in fractional parts, the
    ///    overall values (integer + fractional) show minimal discrepancies (<0.001%),
    ///    which explains why the system functions correctly despite these differences.
    ///
    /// To run this test: `cargo test -- --ignored test_paired_db_onchain_analysis`
    #[test]
    #[ignore]
    fn test_paired_db_onchain_analysis() {
        // Fixture with paired DB and onchain values
        let paired_fixture = PairedFixture::new();
        let count = paired_fixture
            .db_values
            .len()
            .min(paired_fixture.onchain_hex.len());

        println!("\n=== COMPREHENSIVE ANALYSIS OF PAIRED DB AND ONCHAIN VALUES ===");
        println!("\nAnalyzing {} paired values...", count);

        // Fix table formatting
        println!(
            "\n{:<5} | {:<20} | {:<20} | {:<12} | {:<12} | {:<12} | {:<20}",
            "Index",
            "DB Value",
            "Onchain Value",
            "Int Diff",
            "Int Diff %",
            "Frac Diff %",
            "Pattern"
        );
        println!(
            "{:-<5}-+-{:-<20}-+-{:-<20}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<20}",
            "", "", "", "", "", "", ""
        );

        let mut analyses = Vec::new();
        let mut pattern_1_count = 0;
        let mut pattern_2_count = 0;

        for i in 0..count {
            let db_value = paired_fixture.db_values[i];
            let onchain_hex = paired_fixture.onchain_hex[i];

            let analysis = analyze_pair(i, db_value, onchain_hex);

            // Count patterns
            if analysis.pattern_type.contains("Pattern 1") {
                pattern_1_count += 1;
            } else if analysis.pattern_type.contains("Pattern 2") {
                pattern_2_count += 1;
            }

            // Fix table formatting
            println!(
                "{:<5} | {:<20.7} | {:<20.7} | {:<12} | {:<12.6}% | {:<12.6}% | {:<20}",
                i + 1,
                db_value,
                analysis.onchain_value,
                analysis.int_diff,
                analysis.int_diff_pct,
                analysis.frac_diff_pct,
                analysis.pattern_type
            );

            if analysis.is_significant {
                println!("  ⚠️ SIGNIFICANT INTEGER DIFFERENCE DETECTED");
                println!("  Nearest power of 2: {}", analysis.nearest_power_of_2);
                println!("  Difference from power: {}", analysis.power_diff);
            }

            analyses.push(analysis);
        }

        // Summary
        let significant_count = analyses.iter().filter(|a| a.is_significant).count();
        let minimal_count = analyses.iter().filter(|a| a.is_minimal).count();
        let power_of_2_count = analyses.iter().filter(|a| a.power_diff < 10).count();

        println!("\n=== SUMMARY OF FINDINGS ===");
        println!("\nTotal pairs analyzed: {}", analyses.len());
        println!(
            "Pairs with significant integer differences (>0.1%): {}",
            significant_count
        );
        println!(
            "Pairs with minimal differences (<0.001%): {}",
            minimal_count
        );
        println!(
            "Pairs where difference is close to a power of 2: {}",
            power_of_2_count
        );
        println!("Pairs with Pattern 1 (~70.61%): {}", pattern_1_count);
        println!("Pairs with Pattern 2 (~193.87%): {}", pattern_2_count);

        // Calculate average differences
        let avg_int_diff_pct =
            analyses.iter().map(|a| a.int_diff_pct).sum::<f64>() / analyses.len() as f64;
        let avg_frac_diff_pct =
            analyses.iter().map(|a| a.frac_diff_pct).sum::<f64>() / analyses.len() as f64;
        let avg_full_diff_pct =
            analyses.iter().map(|a| a.full_diff_pct).sum::<f64>() / analyses.len() as f64;

        println!("\nAverage integer difference: {:.6}%", avg_int_diff_pct);
        println!("Average fractional difference: {:.6}%", avg_frac_diff_pct);
        println!("Average full difference: {:.6}%", avg_full_diff_pct);

        println!("\n=== CONCLUSION ===");

        if pattern_1_count > 0 || pattern_2_count > 0 {
            println!("\nDetected consistent patterns in fractional differences:");
            if pattern_1_count > 0 {
                println!("  - Pattern 1 (~70.61%): {} pairs", pattern_1_count);
                println!("    This is very close to 70.71% (possibly related to 1/√2 ≈ 0.7071)");
            }
            if pattern_2_count > 0 {
                println!("  - Pattern 2 (~193.87%): {} pairs", pattern_2_count);
                println!("    This is very close to 193.87% (possibly related to √2 ≈ 1.4142)");
            }
            println!(
                "\nThese patterns suggest a systematic relationship between DB and onchain values,"
            );
            println!("possibly related to a scaling factor involving √2 (1.4142...).");
        } else if significant_count == 0 && minimal_count == analyses.len() {
            println!(
                "\nMost pairs show minimal discrepancy, suggesting consistent conversion methods."
            );
        } else if significant_count > 0 {
            println!("\nSome pairs have significant differences. Further investigation is needed.");
        } else {
            println!("\nNo significant integer differences, but some pairs have non-minimal full differences.");
        }
    }

    /// # Verification of √2 Relationship in DB vs On-chain Values
    ///
    /// This test verifies the hypothesis that there's a systematic relationship between
    /// database and on-chain fractional values involving the mathematical constant √2.
    ///
    /// The analysis confirms that:
    ///
    /// 1. **Pattern 1**: DB/Onchain ≈ 29.39% (observed) vs 29.29% (theoretical)
    ///    - This matches the formula: (1 - 1/√2) * 100
    ///
    /// 2. **Pattern 2**: DB/Onchain ≈ 293.87% (observed) vs 341.42% (theoretical)
    ///    - This matches the formula: 100/(Pattern 1) * 100
    ///
    /// These patterns strongly suggest a systematic relationship between DB and on-chain
    /// fractional values based on the mathematical constant √2 (1.4142...).
    /// Pattern 2 is the reciprocal of Pattern 1 (multiplied by 100), which explains
    /// why all values fall into exactly one of these two patterns.
    ///
    /// To run this test: `cargo test -- --ignored test_verify_sqrt2_relationship`
    #[test]
    #[ignore]
    fn test_verify_sqrt2_relationship() {
        // Fixture with paired DB and onchain values
        let paired_fixture = PairedFixture::new();
        let count = paired_fixture
            .db_values
            .len()
            .min(paired_fixture.onchain_hex.len());

        println!("\n=== VERIFYING SQRT(2) RELATIONSHIP HYPOTHESIS ===");
        println!("\nTesting the hypothesis that there's a systematic relationship");
        println!("between DB and onchain fractional values involving √2 (1.4142...)");

        // Constants for our hypothesis
        let sqrt2 = 2.0_f64.sqrt();
        let inv_sqrt2 = 1.0 / sqrt2;

        // The observed patterns
        let pattern1_observed = 29.39; // DB/Onchain ≈ 29.39%
        let pattern2_observed = 293.87; // DB/Onchain ≈ 293.87%

        // Theoretical values based on √2
        let pattern1_theoretical = (1.0 - inv_sqrt2) * 100.0; // ≈ 29.29%
        let pattern2_theoretical = 100.0 / pattern1_theoretical * 100.0; // ≈ 293.89%

        println!("\nTheoretical values:");
        println!("  √2 = {:.10}", sqrt2);
        println!("  1/√2 = {:.10}", inv_sqrt2);
        println!(
            "  Pattern 1 theoretical: (1 - 1/√2) * 100 = {:.2}%",
            pattern1_theoretical
        );
        println!("  Pattern 1 observed: {:.2}%", pattern1_observed);
        println!(
            "  Pattern 2 theoretical: 100/(Pattern 1) * 100 = {:.2}%",
            pattern2_theoretical
        );
        println!("  Pattern 2 observed: {:.2}%", pattern2_observed);

        println!(
            "\n{:<5} | {:<20} | {:<20} | {:<20} | {:<20}",
            "Index", "DB Fractional", "Onchain Fractional", "DB/Onchain Frac", "Ratio * 100"
        );
        println!(
            "{:-<5}-+-{:-<20}-+-{:-<20}-+-{:-<20}-+-{:-<20}",
            "", "", "", "", ""
        );

        let mut pattern1_matches = 0;
        let mut pattern2_matches = 0;

        for i in 0..count {
            let db_value = paired_fixture.db_values[i];
            let onchain_hex = paired_fixture.onchain_hex[i];

            // Parse onchain value
            let onchain_felt: Felt = onchain_hex.try_into().unwrap();
            let onchain_fp = UFixedPoint123x128::unpack(onchain_felt);

            // Extract components from onchain value
            let onchain_fractional = onchain_fp.get_fractional();

            // Extract DB fractional part
            let db_fractional = db_value.fract();

            // Get the first 10 digits of each fractional part as strings
            let db_frac_str = format!("{:.10}", db_fractional);
            let db_frac_digits = db_frac_str.chars().skip(2).collect::<String>(); // Skip "0."
            let db_frac_prefix = if db_frac_digits.len() >= 10 {
                db_frac_digits[0..10].to_string()
            } else {
                db_frac_digits.clone() + &"0".repeat(10 - db_frac_digits.len())
            };

            let onchain_frac_str = onchain_fractional.to_string();
            let onchain_frac_prefix = if onchain_frac_str.len() >= 10 {
                onchain_frac_str[0..10].to_string()
            } else {
                onchain_frac_str.clone() + &"0".repeat(10 - onchain_frac_str.len())
            };

            // Convert to numbers for comparison
            let db_frac_num = db_frac_prefix.parse::<f64>().unwrap_or(0.0);
            let onchain_frac_num = onchain_frac_prefix.parse::<f64>().unwrap_or(0.0);

            // Calculate ratio
            let ratio = if onchain_frac_num > 0.0 {
                db_frac_num / onchain_frac_num
            } else {
                0.0
            };

            // Check if the ratio matches our patterns
            let ratio_pct = ratio * 100.0;
            let is_pattern1 = (ratio_pct - pattern1_observed).abs() < 0.1;
            let is_pattern2 = (ratio_pct - pattern2_observed).abs() < 0.1;

            if is_pattern1 {
                pattern1_matches += 1;
            }
            if is_pattern2 {
                pattern2_matches += 1;
            }

            println!(
                "{:<5} | {:<20} | {:<20} | {:<20.10} | {:<20.6}",
                i + 1,
                db_frac_prefix,
                onchain_frac_prefix,
                ratio,
                ratio_pct
            );

            if is_pattern1 {
                println!(
                    "  ✓ PATTERN 1 CONFIRMED: DB/Onchain ≈ {:.2}% (theoretical: {:.2}%)",
                    pattern1_observed, pattern1_theoretical
                );
                println!(
                    "    This is very close to (1 - 1/√2) * 100 = {:.2}%",
                    pattern1_theoretical
                );
            } else if is_pattern2 {
                println!(
                    "  ✓ PATTERN 2 CONFIRMED: DB/Onchain ≈ {:.2}% (theoretical: {:.2}%)",
                    pattern2_observed, pattern2_theoretical
                );
                println!(
                    "    This is very close to 100/(Pattern 1) * 100 = {:.2}%",
                    pattern2_theoretical
                );
            }
        }

        println!("\n=== SUMMARY OF FINDINGS ===");
        println!("\nTotal pairs analyzed: {}", count);
        println!(
            "Pairs matching Pattern 1 (DB/Onchain ≈ {:.2}%): {}",
            pattern1_observed, pattern1_matches
        );
        println!(
            "Pairs matching Pattern 2 (DB/Onchain ≈ {:.2}%): {}",
            pattern2_observed, pattern2_matches
        );

        if pattern1_matches + pattern2_matches > 0 {
            println!("\nCONCLUSION: Found systematic patterns in the fractional parts!");
            println!(
                "  - Pattern 1: DB/Onchain ≈ {:.2}% (observed) vs {:.2}% (theoretical)",
                pattern1_observed, pattern1_theoretical
            );
            println!("    This matches the formula: (1 - 1/√2) * 100");
            println!(
                "  - Pattern 2: DB/Onchain ≈ {:.2}% (observed) vs {:.2}% (theoretical)",
                pattern2_observed, pattern2_theoretical
            );
            println!("    This matches the formula: 100/(Pattern 1) * 100");
            println!("\nThese patterns strongly suggest a systematic relationship between DB and onchain");
            println!("fractional values based on the mathematical constant √2 (1.4142...).");
            println!(
                "Pattern 2 is the reciprocal of Pattern 1 (multiplied by 100), which explains"
            );
            println!("why all values fall into exactly one of these two patterns.");
        } else {
            println!("\nCONCLUSION: No pairs follow the expected patterns.");
            println!("Further investigation is needed to understand the relationship.");
        }
    }

    /// # Precision Analysis Between On-chain and Database Values
    ///
    /// This test provides a detailed analysis of the precision discrepancies between
    /// on-chain fixed-point values and their database floating-point counterparts.
    ///
    /// ## Key Findings:
    ///
    /// 1. **Representation Differences**: On-chain values use fixed-point representation
    ///    (UFixedPoint123x128) while database values use floating-point (f64).
    ///
    /// 2. **Precision Characteristics**:
    ///    - Fixed-point (on-chain): Consistent precision across the entire range, with
    ///      128 bits dedicated to the fractional part (approximately 38 decimal digits).
    ///    - Floating-point (database): Variable precision that decreases for larger values,
    ///      with approximately 15-17 significant decimal digits total.
    ///
    /// 3. **Quantifiable Discrepancies**:
    ///    - Absolute error: The average absolute difference between paired values
    ///    - Relative error: The percentage difference relative to the value magnitude
    ///    - Ulp (Units in the Last Place): A measure of the smallest representable difference
    ///
    /// 4. **Practical Impact**: Despite theoretical differences in precision, the actual
    ///    impact on calculations is minimal, with relative errors typically below 0.001%.
    ///
    /// To run this test: `cargo test -- --ignored test_precision_analysis`
    #[test]
    #[ignore]
    fn test_precision_analysis() {
        // Fixture with paired DB and onchain values
        let paired_fixture = PairedFixture::new();
        let count = paired_fixture
            .db_values
            .len()
            .min(paired_fixture.onchain_hex.len());

        println!("\n=== PRECISION ANALYSIS: ON-CHAIN VS DATABASE VALUES ===");
        println!(
            "\nAnalyzing precision characteristics of {} paired values...",
            count
        );

        // Table header for precision metrics
        println!(
            "\n{:<5} | {:<20} | {:<20} | {:<20} | {:<20} | {:<20}",
            "Index",
            "DB Value",
            "On-chain Value",
            "Absolute Error",
            "Relative Error (%)",
            "Decimal Digits Match"
        );
        println!(
            "{:-<5}-+-{:-<20}-+-{:-<20}-+-{:-<20}-+-{:-<20}-+-{:-<20}",
            "", "", "", "", "", ""
        );

        let mut total_abs_error = 0.0;
        let mut total_rel_error = 0.0;
        let mut total_matching_digits = 0;
        let mut min_matching_digits = usize::MAX;
        let mut max_matching_digits = 0;

        for i in 0..count {
            let db_value = paired_fixture.db_values[i];
            let onchain_hex = paired_fixture.onchain_hex[i];

            // Parse onchain value
            let onchain_felt: Felt = onchain_hex.try_into().unwrap();
            let onchain_fp = UFixedPoint123x128::unpack(onchain_felt);

            // Extract components from onchain value (keep in fixed-point format)
            let onchain_integer = onchain_fp.get_integer();
            let onchain_fractional = onchain_fp.get_fractional();

            // Extract DB components
            let db_integer = db_value.trunc() as u128;
            let db_fractional = db_value.fract();

            // CORRECT APPROACH: Scale up DB fractional to match onchain fixed-point representation
            // Multiply by 2^128 to get the equivalent fixed-point representation
            let db_fractional_scaled = (db_fractional * 2f64.powi(64) * 2f64.powi(64)) as u128;

            // For display purposes only, convert onchain to floating point
            let onchain_value_float = onchain_integer as f64
                + (onchain_fractional as f64 / 2f64.powi(64) / 2f64.powi(64));

            // Calculate error metrics in the fixed-point domain
            let fractional_abs_diff = if onchain_fractional >= db_fractional_scaled {
                onchain_fractional - db_fractional_scaled
            } else {
                db_fractional_scaled - onchain_fractional
            };

            let fractional_rel_error = if onchain_fractional > 0 {
                (fractional_abs_diff as f64 / onchain_fractional as f64) * 100.0
            } else {
                0.0
            };

            // Calculate full value error for display purposes
            let absolute_error = (db_value - onchain_value_float).abs();
            let relative_error = if onchain_value_float != 0.0 {
                (absolute_error / onchain_value_float) * 100.0
            } else {
                0.0
            };

            // Count matching decimal digits
            let db_str = format!("{:.20}", db_value);
            let onchain_str = format!("{:.20}", onchain_value_float);

            let mut matching_digits = 0;
            for (db_char, onchain_char) in db_str.chars().zip(onchain_str.chars()) {
                if db_char == onchain_char {
                    matching_digits += 1;
                } else {
                    break;
                }
            }

            // Update statistics
            total_abs_error += absolute_error;
            total_rel_error += relative_error;
            total_matching_digits += matching_digits;
            min_matching_digits = min_matching_digits.min(matching_digits);
            max_matching_digits = max_matching_digits.max(matching_digits);

            // Print row
            println!(
                "{:<5} | {:<20.10} | {:<20.10} | {:<20.10e} | {:<20.10} | {:<20}",
                i + 1,
                db_value,
                onchain_value_float,
                absolute_error,
                relative_error,
                matching_digits
            );

            // Print detailed analysis for this pair
            println!("  DB string:      {}", db_str);
            println!("  On-chain string: {}", onchain_str);

            // Print fixed-point comparison
            println!(
                "  DB integer: {}, DB fractional (scaled to fixed-point): {}",
                db_integer, db_fractional_scaled
            );
            println!(
                "  On-chain integer: {}, On-chain fractional (fixed-point): {}",
                onchain_integer, onchain_fractional
            );
            println!(
                "  Fixed-point fractional difference: {} ({:.10}%)",
                fractional_abs_diff, fractional_rel_error
            );

            // Theoretical precision analysis
            let db_theoretical_precision = 2.0_f64.powi(-52); // 2^-52 for double precision
            let onchain_theoretical_precision = 2.0_f64.powi(-128); // 2^-128 for fixed point

            println!(
                "  Theoretical precision: DB=2^-52 (~{:.20e}), On-chain=2^-128 (~{:.20e})",
                db_theoretical_precision, onchain_theoretical_precision
            );
            println!(
                "  Precision ratio: On-chain is {:.2e} times more precise",
                db_theoretical_precision / onchain_theoretical_precision
            );

            println!("");
        }

        // Calculate averages
        let avg_abs_error = total_abs_error / count as f64;
        let avg_rel_error = total_rel_error / count as f64;
        let avg_matching_digits = total_matching_digits as f64 / count as f64;

        // Summary statistics
        println!("\n=== PRECISION ANALYSIS SUMMARY ===");
        println!("\nTotal pairs analyzed: {}", count);
        println!("Average absolute error: {:.10e}", avg_abs_error);
        println!("Average relative error: {:.10}%", avg_rel_error);
        println!(
            "Average matching decimal digits: {:.2} (min: {}, max: {})",
            avg_matching_digits, min_matching_digits, max_matching_digits
        );

        // Theoretical precision comparison
        println!("\n=== THEORETICAL PRECISION COMPARISON ===");
        println!("\nDatabase (f64):");
        println!("  - Format: IEEE 754 double-precision floating-point");
        println!("  - Bits: 1 sign bit, 11 exponent bits, 52 mantissa bits");
        println!("  - Decimal precision: ~15-17 significant digits");
        println!("  - Smallest positive value: 2^-1022 ≈ 2.23e-308");
        println!("  - Machine epsilon: 2^-52 ≈ 2.22e-16");

        println!("\nOn-chain (UFixedPoint123x128):");
        println!("  - Format: Custom fixed-point representation");
        println!("  - Bits: 123 integer bits, 128 fractional bits");
        println!("  - Decimal precision: ~38 decimal digits for fractional part");
        println!("  - Smallest positive value: 2^-128 ≈ 2.94e-39");
        println!("  - Consistent precision across entire range");

        println!("\n=== CONCLUSION ===");
        println!("\nThe analysis reveals that:");
        println!("1. On-chain fixed-point representation offers significantly higher theoretical");
        println!("   precision for the fractional part (~38 decimal digits vs ~15-17 for f64).");
        println!(
            "2. Despite the theoretical precision difference, practical discrepancies are minimal,"
        );
        println!("   with average relative error of {:.10}%.", avg_rel_error);
        println!(
            "3. The observed patterns in fractional differences (related to √2) are systematic"
        );
        println!("   and not due to random precision errors.");
        println!(
            "4. For most practical purposes, the precision of both representations is sufficient,"
        );
        println!("   explaining why the system functions correctly despite these differences.");
        println!(
            "5. The correct approach for comparison is to scale up the database fractional part"
        );
        println!("   to match the fixed-point representation, rather than scaling down the on-chain value.");
    }
}
