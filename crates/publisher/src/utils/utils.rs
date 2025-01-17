use crate::errors::AccumulatorError;

/// Validates that a hex string represents a valid U256 (256-bit unsigned integer)
pub fn validate_u256_hex(hex: &str) -> Result<(), AccumulatorError> {
    if !hex.starts_with("0x") || hex.len() <= 2 {
        // Check for "0x" prefix and ensure there's data after it
        return Err(AccumulatorError::InvalidU256Hex(hex.to_string()));
    }

    // Remove '0x' prefix and check if remaining string is valid hex
    let hex_value = &hex[2..];
    if !hex_value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AccumulatorError::InvalidU256Hex(hex.to_string()));
    }

    // Check length - maximum 64 hex chars (256 bits = 64 hex digits)
    if hex_value.len() > 64 {
        return Err(AccumulatorError::InvalidU256Hex(hex.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_u256_hex() {
        // Basic valid cases
        assert!(validate_u256_hex("0x123").is_ok());
        assert!(validate_u256_hex("0xabc").is_ok());
        assert!(validate_u256_hex("0xABC123").is_ok());

        // Edge cases - valid
        assert!(validate_u256_hex("0x0").is_ok());
        assert!(validate_u256_hex(&("0x".to_owned() + &"f".repeat(64))).is_ok()); // Max length
        assert!(validate_u256_hex("0xdeadbeef").is_ok());
    }

    #[test]
    fn test_invalid_u256_hex() {
        assert!(validate_u256_hex("0x").is_err());
        assert!(validate_u256_hex("0").is_err());
        assert!(validate_u256_hex("").is_err());
        assert!(validate_u256_hex("invalid").is_err());
    }

    #[test]
    fn test_error_message() {
        let result = validate_u256_hex("invalid");
        match result {
            Err(AccumulatorError::InvalidU256Hex(msg)) => {
                assert_eq!(msg, "invalid");
            }
            _ => panic!("Expected InvalidU256Hex error"),
        }
    }
}
