use crate::errors::AccumulatorError;

/// Validates that a hex string represents a valid U256 (256-bit unsigned integer)
pub fn validate_u256_hex(hex_str: &str) -> Result<(), AccumulatorError> {
    // Check if it's a valid hex string with '0x' prefix
    if !hex_str.starts_with("0x") {
        return Err(AccumulatorError::InvalidU256Hex(hex_str.to_string()).into());
    }

    // Remove '0x' prefix and check if remaining string is valid hex
    let hex_value = &hex_str[2..];
    if !hex_value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AccumulatorError::InvalidU256Hex(hex_str.to_string()).into());
    }

    // Check length - maximum 64 hex chars (256 bits = 64 hex digits)
    // Note: we allow shorter values as they're valid smaller numbers
    if hex_value.len() > 64 {
        return Err(AccumulatorError::InvalidU256Hex(hex_str.to_string()).into());
    }

    Ok(())
}
