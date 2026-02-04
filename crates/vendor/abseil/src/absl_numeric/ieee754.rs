//! IEEE 754 floating-point utilities.

/// Returns the bit representation of a float as u32.
///
/// This uses the safe Rust `f32::to_bits()` method which properly
/// handles all special values including NaN payloads, infinities, etc.
#[inline]
pub fn float_to_bits(value: f32) -> u32 {
    value.to_bits()
}

/// Returns the float from its bit representation.
///
/// This uses the safe Rust `f32::from_bits()` method which properly
/// handles all special values including NaN payloads, infinities, etc.
#[inline]
pub fn bits_to_float(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Returns the bit representation of a double as u64.
///
/// This uses the safe Rust `f64::to_bits()` method which properly
/// handles all special values including NaN payloads, infinities, etc.
#[inline]
pub fn double_to_bits(value: f64) -> u64 {
    value.to_bits()
}

/// Returns the double from its bit representation.
///
/// This uses the safe Rust `f64::from_bits()` method which properly
/// handles all special values including NaN payloads, infinities, etc.
#[inline]
pub fn bits_to_double(bits: u64) -> f64 {
    f64::from_bits(bits)
}

/// Checks if two floats have the same bit representation (including NaN).
#[inline]
pub fn float_eq_bits(a: f32, b: f32) -> bool {
    float_to_bits(a) == float_to_bits(b)
}

/// Checks if two doubles have the same bit representation (including NaN).
#[inline]
pub fn double_eq_bits(a: f64, b: f64) -> bool {
    double_to_bits(a) == double_to_bits(b)
}

/// Extracts the exponent from a float (biased).
#[inline]
pub fn extract_exponent_f32(value: f32) -> u32 {
    (float_to_bits(value) >> 23) & 0xFF
}

/// Extracts the mantissa from a float (without implicit leading 1).
#[inline]
pub fn extract_mantissa_f32(value: f32) -> u32 {
    float_to_bits(value) & 0x7FFFFF
}

/// Constructs a float from exponent and mantissa.
#[inline]
pub fn construct_f32(sign: bool, exponent: u32, mantissa: u32) -> f32 {
    let mut bits = 0u32;
    if sign {
        bits |= 1 << 31;
    }
    bits |= (exponent & 0xFF) << 23;
    bits |= mantissa & 0x7FFFFF;
    bits_to_float(bits)
}

/// Extracts the exponent from a double (biased).
#[inline]
pub fn extract_exponent_f64(value: f64) -> u16 {
    ((double_to_bits(value) >> 52) & 0x7FF) as u16
}

/// Extracts the mantissa from a double (without implicit leading 1).
#[inline]
pub fn extract_mantissa_f64(value: f64) -> u64 {
    double_to_bits(value) & 0xFFFFFFFFFFFFF
}

/// Constructs a double from exponent and mantissa.
#[inline]
pub fn construct_f64(sign: bool, exponent: u16, mantissa: u64) -> f64 {
    let mut bits = 0u64;
    if sign {
        bits |= 1 << 63;
    }
    bits |= ((exponent & 0x7FF) as u64) << 52;
    bits |= mantissa & 0xFFFFFFFFFFFFF;
    bits_to_double(bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_to_bits_roundtrip() {
        let original = 3.14159f32;
        let bits = float_to_bits(original);
        let restored = bits_to_float(bits);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_double_to_bits_roundtrip() {
        let original = 2.718281828459045f64;
        let bits = double_to_bits(original);
        let restored = bits_to_double(bits);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_float_special_values() {
        // Test zero
        assert_eq!(float_to_bits(0.0f32), 0x00000000);
        assert_eq!(float_to_bits(-0.0f32), 0x80000000);

        // Test infinity
        let pos_inf_bits = float_to_bits(f32::INFINITY);
        assert_eq!(pos_inf_bits, 0x7F800000);
        let neg_inf_bits = float_to_bits(f32::NEG_INFINITY);
        assert_eq!(neg_inf_bits, 0xFF800000);

        // Test NaN - different NaN values have different bit patterns
        let nan1 = f32::NAN;
        let nan2 = f32::nan();
        // Both should be NaN but may have different bit patterns
        assert!(nan1.is_nan());
        assert!(nan2.is_nan());
    }

    #[test]
    fn test_double_special_values() {
        // Test zero
        assert_eq!(double_to_bits(0.0f64), 0x0000000000000000);
        assert_eq!(double_to_bits(-0.0f64), 0x8000000000000000);

        // Test infinity
        let pos_inf_bits = double_to_bits(f64::INFINITY);
        assert_eq!(pos_inf_bits, 0x7FF0000000000000);
        let neg_inf_bits = double_to_bits(f64::NEG_INFINITY);
        assert_eq!(neg_inf_bits, 0xFFF0000000000000);

        // Test NaN
        let nan = f64::NAN;
        assert!(nan.is_nan());
    }

    #[test]
    fn test_float_eq_bits_with_nan() {
        // NaN values should compare equal using bit comparison
        // even though they don't compare equal normally
        let nan1 = f32::NAN;
        let nan2 = f32::from_bits(float_to_bits(nan1));
        assert!(float_eq_bits(nan1, nan2));
        // Normal comparison would fail
        assert!(nan1 != nan2);
    }

    #[test]
    fn test_double_eq_bits_with_nan() {
        let nan1 = f64::NAN;
        let nan2 = f64::from_bits(double_to_bits(nan1));
        assert!(double_eq_bits(nan1, nan2));
        assert!(nan1 != nan2); // Normal comparison fails
    }

    #[test]
    fn test_extract_exponent_f32() {
        // 1.0 = 1.0 * 2^0, exponent bias is 127
        assert_eq!(extract_exponent_f32(1.0), 127);
        // 2.0 = 1.0 * 2^1, exponent is 128
        assert_eq!(extract_exponent_f32(2.0), 128);
        // 0.5 = 1.0 * 2^-1, exponent is 126
        assert_eq!(extract_exponent_f32(0.5), 126);
    }

    #[test]
    fn test_extract_exponent_f64() {
        // 1.0 = 1.0 * 2^0, exponent bias is 1023
        assert_eq!(extract_exponent_f64(1.0), 1023);
        assert_eq!(extract_exponent_f64(2.0), 1024);
        assert_eq!(extract_exponent_f64(0.5), 1022);
    }

    #[test]
    fn test_extract_mantissa_f32() {
        // 1.5 = 1.1 in binary, mantissa is 0x400000
        assert_eq!(extract_mantissa_f32(1.5), 0x400000);
        // 3.0 = 1.5 * 2^1, mantissa should be same
        assert_eq!(extract_mantissa_f32(3.0), 0x400000);
    }

    #[test]
    fn test_extract_mantissa_f64() {
        // 1.5 = 1.1 in binary, mantissa is 0x8000000000000
        assert_eq!(extract_mantissa_f64(1.5), 0x8000000000000);
    }

    #[test]
    fn test_construct_f32() {
        // Construct 1.0: sign=0, exponent=127, mantissa=0
        let f = construct_f32(false, 127, 0);
        assert_eq!(f, 1.0);

        // Construct -2.0: sign=1, exponent=128, mantissa=0
        let f = construct_f32(true, 128, 0);
        assert_eq!(f, -2.0);
    }

    #[test]
    fn test_construct_f64() {
        // Construct 1.0: sign=0, exponent=1023, mantissa=0
        let f = construct_f64(false, 1023, 0);
        assert_eq!(f, 1.0);

        // Construct -2.0: sign=1, exponent=1024, mantissa=0
        let f = construct_f64(true, 1024, 0);
        assert_eq!(f, -2.0);
    }

    #[test]
    fn test_bits_to_float_preserves_special_values() {
        // Infinity bits should roundtrip correctly
        let inf_bits = 0x7F800000u32;
        let f = bits_to_float(inf_bits);
        assert!(f.is_infinite() && f.is_sign_positive());

        let neg_inf_bits = 0xFF800000u32;
        let f = bits_to_float(neg_inf_bits);
        assert!(f.is_infinite() && f.is_sign_negative());

        // NaN bits should roundtrip correctly (any exponent all 1s with non-zero mantissa)
        let nan_bits = 0x7FC00000u32; // Quiet NaN
        let f = bits_to_float(nan_bits);
        assert!(f.is_nan());
    }

    #[test]
    fn test_bits_to_double_preserves_special_values() {
        let inf_bits = 0x7FF0000000000000u64;
        let f = bits_to_double(inf_bits);
        assert!(f.is_infinite() && f.is_sign_positive());

        let neg_inf_bits = 0xFFF0000000000000u64;
        let f = bits_to_double(neg_inf_bits);
        assert!(f.is_infinite() && f.is_sign_negative());

        let nan_bits = 0x7FF8000000000000u64; // Quiet NaN
        let f = bits_to_double(nan_bits);
        assert!(f.is_nan());
    }

    #[test]
    fn test_safe_replacements_no_undefined_behavior() {
        // These tests verify that the safe replacements handle edge cases
        // that would be undefined behavior with raw transmute

        // Normal values
        assert_eq!(float_to_bits(1.0f32).to_le(), 1.0f32.to_bits().to_le());

        // Denormal numbers
        let tiny = f32::MIN_POSITIVE / 2.0;
        assert!(tiny > 0.0 && tiny < f32::MIN_POSITIVE);

        // All special values are handled safely
        assert!(bits_to_float(0x7F800000u32).is_infinite());
        assert!(bits_to_double(0x7FF0000000000000u64).is_infinite());
        assert!(bits_to_float(0x7FC00000u32).is_nan());
        assert!(bits_to_double(0x7FF8000000000000u64).is_nan());
    }
}
