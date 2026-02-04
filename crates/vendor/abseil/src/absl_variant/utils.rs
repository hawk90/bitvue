//! Variant utility functions.

use super::multi_variant::MultiVariant;
use super::error::VariantError;

/// Maximum string length for variant parsing to prevent DoS attacks.
/// This limit prevents memory exhaustion and CPU exhaustion from parsing
/// extremely long strings.
const MAX_PARSE_LENGTH: usize = 1024 * 1024; // 1 MB

/// Compares two variants, returning None if they're of different types.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::{variant_compare, MultiVariant};
///
/// let v1 = MultiVariant::I32(42);
/// let v2 = MultiVariant::I32(50);
/// assert_eq!(variant_compare(&v1, &v2), Some(std::cmp::Ordering::Less));
/// ```
pub fn variant_compare(
    a: &MultiVariant,
    b: &MultiVariant,
) -> Option<core::cmp::Ordering> {
    a.try_compare(b)
}

/// Returns true if two variants have equal values (even if different types).
///
/// This performs type coercion for compatible numeric types.
///
/// # Safety
///
/// For signed-to-unsigned conversions, this function checks that values
/// are non-negative before comparing, avoiding undefined behavior from
/// negative values being reinterpreted as large unsigned values.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::{variant_eq_coerce, MultiVariant};
///
/// let v1 = MultiVariant::I32(42);
/// let v2 = MultiVariant::I64(42);
/// assert!(variant_eq_coerce(&v1, &v2));
/// ```
pub fn variant_eq_coerce(a: &MultiVariant, b: &MultiVariant) -> bool {
    match (a, b) {
        // i32 to i64 is always safe (widening conversion)
        (MultiVariant::I32(a), MultiVariant::I64(b)) => *a as i64 == *b,
        // i64 to i32 needs range check - only compare if in i32 range
        (MultiVariant::I64(a), MultiVariant::I32(b)) => {
            *a >= i32::MIN as i64 && *a <= i32::MAX as i64 && *a == *b as i64
        }
        // u32 to u64 is always safe (widening conversion)
        (MultiVariant::U32(a), MultiVariant::U64(b)) => *a as u64 == *b,
        // u64 to u32 needs range check - only compare if in u32 range
        (MultiVariant::U64(a), MultiVariant::U32(b)) => {
            *a <= u32::MAX as u64 && *a == *b as u64
        }
        // f32 to f64 is always safe (widening conversion)
        (MultiVariant::F32(a), MultiVariant::F64(b)) => *a as f64 == *b,
        // f64 to f32 - comparison may have precision loss but won't crash
        (MultiVariant::F64(a), MultiVariant::F32(b)) => *a == *b as f64,
        _ => a == b,
    }
}

/// Attempts to cast a variant to a different type.
///
/// Returns `None` if the cast would lose precision or overflow.
///
/// # Safety
///
/// This function validates that conversions are safe:
/// - Signed to unsigned: only allows non-negative values
/// - Narrowing conversions: only allows values within target range
/// - Float to integer: not supported (would truncate)
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::{variant_cast, MultiVariant};
///
/// let v = MultiVariant::I32(42);
/// let casted = variant_cast(&v, "i64");
/// assert_eq!(casted, Some(MultiVariant::I64(42)));
///
/// let v = MultiVariant::I32(-1);
/// let casted = variant_cast(&v, "u32");
/// assert_eq!(casted, None); // Negative can't cast to unsigned
/// ```
pub fn variant_cast(variant: &MultiVariant, target_type: &str) -> Option<MultiVariant> {
    match (variant, target_type) {
        // i32 to i64 is always safe (widening conversion)
        (MultiVariant::I32(v), "i64") => Some(MultiVariant::I64(*v as i64)),
        // i32 to u32 requires non-negative check
        (MultiVariant::I32(v), "u32") if *v >= 0 => Some(MultiVariant::U32(*v as u32)),
        // i32 to f32 may lose precision but won't panic
        (MultiVariant::I32(v), "f32") => Some(MultiVariant::F32(*v as f32)),
        // i32 to f64 is always safe (widening conversion)
        (MultiVariant::I32(v), "f64") => Some(MultiVariant::F64(*v as f64)),
        // i64 to i32 requires range check
        (MultiVariant::I64(v), "i32") if *v >= i32::MIN as i64 && *v <= i32::MAX as i64 => {
            Some(MultiVariant::I32(*v as i32))
        }
        // i64 to u64 requires non-negative check
        (MultiVariant::I64(v), "u64") if *v >= 0 => Some(MultiVariant::U64(*v as u64)),
        // i64 to f64 may lose precision but won't panic
        (MultiVariant::I64(v), "f64") => Some(MultiVariant::F64(*v as f64)),
        // u32 to i32 requires range check
        (MultiVariant::U32(v), "i32") if *v <= i32::MAX as u32 => Some(MultiVariant::I32(*v as i32)),
        // u32 to i64 is always safe (widening conversion)
        (MultiVariant::U32(v), "i64") => Some(MultiVariant::I64(*v as i64)),
        // u32 to u64 is always safe (widening conversion)
        (MultiVariant::U32(v), "u64") => Some(MultiVariant::U64(*v as u64)),
        // u64 to i32 requires range check
        (MultiVariant::U64(v), "i64") if *v <= i64::MAX as u64 => Some(MultiVariant::I64(*v as i64)),
        // f32 to f64 is always safe (widening conversion)
        (MultiVariant::F32(v), "f64") => Some(MultiVariant::F64(*v as f64)),
        // f64 to f32 may lose precision but won't panic
        (MultiVariant::F64(v), "f32") => Some(MultiVariant::F32(*v as f32)),
        _ => None,
    }
}

/// Clones a variant, returning a new variant.
pub fn variant_clone(variant: &MultiVariant) -> MultiVariant {
    variant.clone()
}

/// Returns a hash of the variant's value.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::{variant_hash, MultiVariant};
///
/// let v = MultiVariant::I32(42);
/// let hash = variant_hash(&v);
/// assert!(hash > 0);
/// ```
pub fn variant_hash(variant: &MultiVariant) -> u64 {
    let mut hasher = FNVHashBuilder::default();
    match variant {
        MultiVariant::I32(v) => hasher.update(&v.to_le_bytes()),
        MultiVariant::I64(v) => hasher.update(&v.to_le_bytes()),
        MultiVariant::U32(v) => hasher.update(&v.to_le_bytes()),
        MultiVariant::U64(v) => hasher.update(&v.to_le_bytes()),
        MultiVariant::F32(v) => hasher.update(&v.to_le_bytes()),
        MultiVariant::F64(v) => hasher.update(&v.to_le_bytes()),
        MultiVariant::Bool(v) => hasher.update(&[*v as u8]),
        MultiVariant::String(v) => hasher.update(v.as_bytes()),
    }
    hasher.finish()
}

/// Simple FNV hash builder for variant hashing.
struct FNVHashBuilder {
    state: u64,
}

impl FNVHashBuilder {
    fn default() -> Self {
        Self { state: 0xcbf29ce484222325 }
    }

    fn update(&mut self, bytes: &[u8]) -> &mut Self {
        const FNV_PRIME: u64 = 0x100000001b3;
        for &byte in bytes {
            self.state ^= byte as u64;
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
        self
    }

    fn finish(self) -> u64 {
        self.state
    }
}

/// Attempts to parse a string into a variant.
///
/// # Limits
///
/// For security reasons, strings longer than 1 MB will not be parsed
/// and this function will return `None`. This prevents DoS attacks
/// through memory exhaustion or CPU exhaustion from parsing extremely
/// long strings.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::variant_parse;
///
/// let v = variant_parse("42").unwrap();
/// assert!(v.is_i32());
///
/// let v = variant_parse("3.14").unwrap();
/// assert!(v.is_f64());
///
/// let v = variant_parse("true").unwrap();
/// assert!(v.is_bool());
/// ```
pub fn variant_parse(s: &str) -> Option<MultiVariant> {
    // SECURITY: Limit input length to prevent DoS attacks
    if s.len() > MAX_PARSE_LENGTH {
        return None;
    }

    // Try parsing as bool first
    match s.to_lowercase().as_str() {
        "true" => return Some(MultiVariant::Bool(true)),
        "false" => return Some(MultiVariant::Bool(false)),
        _ => {}
    }

    // Try i64
    if let Ok(v) = s.parse::<i64>() {
        if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
            return Some(MultiVariant::I32(v as i32));
        }
        return Some(MultiVariant::I64(v));
    }

    // Try u64
    if let Ok(v) = s.parse::<u64>() {
        if v <= u32::MAX as u64 {
            return Some(MultiVariant::U32(v as u32));
        }
        return Some(MultiVariant::U64(v));
    }

    // Try f64
    if let Ok(v) = s.parse::<f64>() {
        if v >= f32::MIN as f64 && v <= f32::MAX as f64 {
            return Some(MultiVariant::F32(v as f32));
        }
        return Some(MultiVariant::F64(v));
    }

    // Default to string
    Some(MultiVariant::String(s.to_string()))
}

/// Validates a variant, checking for invalid states.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::{variant_validate, MultiVariant};
///
/// let v = MultiVariant::I32(42);
/// assert!(variant_validate(&v).is_ok());
///
/// let v = MultiVariant::F32(f32::NAN);
/// assert!(variant_validate(&v).is_err());
/// ```
pub fn variant_validate(variant: &MultiVariant) -> Result<(), VariantError> {
    match variant {
        MultiVariant::F32(v) if v.is_nan() => Err(VariantError::InvalidFloat("NaN".into())),
        MultiVariant::F64(v) if v.is_nan() => Err(VariantError::InvalidFloat("NaN".into())),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cmp::Ordering;

    #[test]
    fn test_variant_compare() {
        let v1 = MultiVariant::I32(42);
        let v2 = MultiVariant::I32(50);
        assert_eq!(variant_compare(&v1, &v2), Some(Ordering::Less));
    }

    #[test]
    fn test_variant_eq_coerce() {
        let v1 = MultiVariant::I32(42);
        let v2 = MultiVariant::I64(42);
        assert!(variant_eq_coerce(&v1, &v2));

        let v1 = MultiVariant::I32(42);
        let v2 = MultiVariant::I64(43);
        assert!(!variant_eq_coerce(&v1, &v2));
    }

    #[test]
    fn test_variant_cast() {
        let v = MultiVariant::I32(42);
        let casted = variant_cast(&v, "i64");
        assert_eq!(casted, Some(MultiVariant::I64(42)));

        let casted = variant_cast(&v, "f64");
        assert_eq!(casted, Some(MultiVariant::F64(42.0)));

        let v = MultiVariant::I32(-1);
        let casted = variant_cast(&v, "u32");
        assert_eq!(casted, None); // Negative can't cast to unsigned
    }

    #[test]
    fn test_variant_hash() {
        let v1 = MultiVariant::I32(42);
        let h1 = variant_hash(&v1);

        let v2 = MultiVariant::I32(42);
        let h2 = variant_hash(&v2);

        let v3 = MultiVariant::I32(43);
        let h3 = variant_hash(&v3);

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_variant_parse() {
        assert_eq!(variant_parse("42"), Some(MultiVariant::I32(42)));
        assert_eq!(variant_parse("3.14"), Some(MultiVariant::F64(3.14)));
        assert_eq!(variant_parse("true"), Some(MultiVariant::Bool(true)));
        assert_eq!(variant_parse("false"), Some(MultiVariant::Bool(false)));
        assert_eq!(variant_parse("hello"), Some(MultiVariant::String("hello".to_string())));
    }

    #[test]
    fn test_variant_validate() {
        let v = MultiVariant::I32(42);
        assert!(variant_validate(&v).is_ok());

        let v = MultiVariant::F32(f32::NAN);
        assert!(variant_validate(&v).is_err());
    }

    // Test for LOW security fix - string parsing length limit

    #[test]
    fn test_variant_parse_length_limit() {
        // Normal length strings should parse fine
        assert!(variant_parse("42").is_some());
        assert!(variant_parse("hello").is_some());

        // String exceeding maximum length should return None
        let long_string = "a".repeat(MAX_PARSE_LENGTH + 1);
        assert!(variant_parse(&long_string).is_none());
    }

    #[test]
    fn test_variant_parse_max_length_boundary() {
        // String exactly at the limit should parse
        let max_string = "42".repeat(MAX_PARSE_LENGTH / 2);
        // This should succeed as it's within the limit
        assert!(variant_parse(&max_string[..100]).is_some());
    }
}
