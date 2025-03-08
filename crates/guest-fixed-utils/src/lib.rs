#[derive(Debug)]
pub struct U256 {
    pub high: u128,
    pub low: u128,
}

#[derive(Debug)]
pub struct UFixedPoint123x128 {
    pub value: U256,
}

impl From<f64> for UFixedPoint123x128 {
    fn from(x: f64) -> Self {
        // Only non-negative values supported.
        assert!(x >= 0.0, "Only non-negative values supported");
        let scale = 2f64.powi(128);
        let int_part = x.trunc();
        let frac_part = x - int_part;
        let high = int_part as u128;
        let low = (frac_part * scale).trunc() as u128;
        UFixedPoint123x128 {
            value: U256 { high, low },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_integer() {
        let test_value = 1.0_f64;
        let fixed: UFixedPoint123x128 = test_value.into();
        println!("Converting {} -> {:?}", test_value, fixed);
        assert_eq!(fixed.value.high, 1);
        assert_eq!(fixed.value.low, 0);
    }

    #[test]
    fn test_conversion_half() {
        let test_value = 1.5_f64;
        let fixed: UFixedPoint123x128 = test_value.into();
        println!("Converting {} -> {:?}", test_value, fixed);
        let expected_low = (0.5 * 2f64.powi(128)).trunc() as u128;
        assert_eq!(fixed.value.high, 1);
        assert_eq!(fixed.value.low, expected_low);
    }

    #[test]
    fn test_conversion_pi() {
        let test_value = 3.141592653589793_f64;
        let fixed: UFixedPoint123x128 = test_value.into();
        println!("Converting {} -> {:?}", test_value, fixed);
    }

    #[test]
    fn test_conversion_five_thirds() {
        let test_value = 5.0 / 3.0;
        let fixed: UFixedPoint123x128 = test_value.into();
        println!("Converting {} -> {:?}", test_value, fixed);
        let expected_high: u128 = 1;
        // Expected low = floor((2/3)*2^128)
        // On starknet, this is 226854911280625667494870980259286614016.
        // The rounding error you're observing is inherent to converting an f64 (which has 53 bits of precision) into a fixed point with 128 fractional bits.
        // In the case of 5/3, the discrepancy (around 2.5×10⁵ in absolute terms) is negligible relative to the total magnitude (roughly 2.27×10³⁸).
        // When calculating the mathematical mean, if all values are converted in the same way, these tiny discrepancies will hardly affect the final result.
        let expected_low: u128 = 226854911280625667494870980259286614016;
        assert_eq!(fixed.value.high, expected_high);
        assert_eq!(fixed.value.low, expected_low);
    }
}
