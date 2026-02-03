//! MultiVariant type - A variant that can hold one of many predefined types.

use alloc::string::String;
use core::fmt;

/// A generic variant that can hold one of several types.
///
/// This provides similar functionality to C++'s `std::variant` but
/// using Rust's type system.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::MultiVariant;
///
/// let v = MultiVariant::I32(42);
/// assert_eq!(v.as_i32(), Some(&42));
/// assert_eq!(v.as_i64(), None);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum MultiVariant {
    /// Holds an i32 value.
    I32(i32),
    /// Holds an i64 value.
    I64(i64),
    /// Holds a u32 value.
    U32(u32),
    /// Holds a u64 value.
    U64(u64),
    /// Holds a f32 value.
    F32(f32),
    /// Holds a f64 value.
    F64(f64),
    /// Holds a boolean value.
    Bool(bool),
    /// Holds a string.
    String(String),
}

impl MultiVariant {
    /// Returns the value as i32 if it is an I32 variant.
    pub fn as_i32(&self) -> Option<&i32> {
        match self {
            MultiVariant::I32(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the value as i64 if it is an I64 variant.
    pub fn as_i64(&self) -> Option<&i64> {
        match self {
            MultiVariant::I64(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the value as u32 if it is a U32 variant.
    pub fn as_u32(&self) -> Option<&u32> {
        match self {
            MultiVariant::U32(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the value as u64 if it is a U64 variant.
    pub fn as_u64(&self) -> Option<&u64> {
        match self {
            MultiVariant::U64(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the value as f32 if it is an F32 variant.
    pub fn as_f32(&self) -> Option<&f32> {
        match self {
            MultiVariant::F32(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the value as f64 if it is an F64 variant.
    pub fn as_f64(&self) -> Option<&f64> {
        match self {
            MultiVariant::F64(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the value as bool if it is a Bool variant.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            MultiVariant::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the value as string if it is a String variant.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MultiVariant::String(v) => Some(v),
            _ => None,
        }
    }

    /// Returns true if the variant is I32.
    pub fn is_i32(&self) -> bool {
        matches!(self, MultiVariant::I32(_))
    }

    /// Returns true if the variant is I64.
    pub fn is_i64(&self) -> bool {
        matches!(self, MultiVariant::I64(_))
    }

    /// Returns true if the variant is U32.
    pub fn is_u32(&self) -> bool {
        matches!(self, MultiVariant::U32(_))
    }

    /// Returns true if the variant is U64.
    pub fn is_u64(&self) -> bool {
        matches!(self, MultiVariant::U64(_))
    }

    /// Returns true if the variant is F32.
    pub fn is_f32(&self) -> bool {
        matches!(self, MultiVariant::F32(_))
    }

    /// Returns true if the variant is F64.
    pub fn is_f64(&self) -> bool {
        matches!(self, MultiVariant::F64(_))
    }

    /// Returns true if the variant is Bool.
    pub fn is_bool(&self) -> bool {
        matches!(self, MultiVariant::Bool(_))
    }

    /// Returns true if the variant is String.
    pub fn is_string(&self) -> bool {
        matches!(self, MultiVariant::String(_))
    }

    /// Returns the type name of the contained value.
    pub fn type_name(&self) -> &'static str {
        match self {
            MultiVariant::I32(_) => "i32",
            MultiVariant::I64(_) => "i64",
            MultiVariant::U32(_) => "u32",
            MultiVariant::U64(_) => "u64",
            MultiVariant::F32(_) => "f32",
            MultiVariant::F64(_) => "f64",
            MultiVariant::Bool(_) => "bool",
            MultiVariant::String(_) => "string",
        }
    }

    /// Returns the size of the contained value in bytes.
    pub fn size(&self) -> usize {
        match self {
            MultiVariant::I32(_) => core::mem::size_of::<i32>(),
            MultiVariant::I64(_) => core::mem::size_of::<i64>(),
            MultiVariant::U32(_) => core::mem::size_of::<u32>(),
            MultiVariant::U64(_) => core::mem::size_of::<u64>(),
            MultiVariant::F32(_) => core::mem::size_of::<f32>(),
            MultiVariant::F64(_) => core::mem::size_of::<f64>(),
            MultiVariant::Bool(_) => core::mem::size_of::<bool>(),
            MultiVariant::String(s) => s.len(),
        }
    }
}

impl fmt::Display for MultiVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MultiVariant::I32(v) => write!(f, "{}", v),
            MultiVariant::I64(v) => write!(f, "{}", v),
            MultiVariant::U32(v) => write!(f, "{}", v),
            MultiVariant::U64(v) => write!(f, "{}", v),
            MultiVariant::F32(v) => write!(f, "{}", v),
            MultiVariant::F64(v) => write!(f, "{}", v),
            MultiVariant::Bool(v) => write!(f, "{}", v),
            MultiVariant::String(v) => write!(f, "{}", v),
        }
    }
}

impl Default for MultiVariant {
    fn default() -> Self {
        MultiVariant::I32(0)
    }
}

impl From<i32> for MultiVariant {
    fn from(v: i32) -> Self {
        MultiVariant::I32(v)
    }
}

impl From<i64> for MultiVariant {
    fn from(v: i64) -> Self {
        MultiVariant::I64(v)
    }
}

impl From<u32> for MultiVariant {
    fn from(v: u32) -> Self {
        MultiVariant::U32(v)
    }
}

impl From<u64> for MultiVariant {
    fn from(v: u64) -> Self {
        MultiVariant::U64(v)
    }
}

impl From<f32> for MultiVariant {
    fn from(v: f32) -> Self {
        MultiVariant::F32(v)
    }
}

impl From<f64> for MultiVariant {
    fn from(v: f64) -> Self {
        MultiVariant::F64(v)
    }
}

impl From<bool> for MultiVariant {
    fn from(v: bool) -> Self {
        MultiVariant::Bool(v)
    }
}

impl From<String> for MultiVariant {
    fn from(v: String) -> Self {
        MultiVariant::String(v)
    }
}

impl<'a> From<&'a str> for MultiVariant {
    fn from(v: &'a str) -> Self {
        MultiVariant::String(v.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_variant_i32() {
        let v = MultiVariant::I32(42);
        assert_eq!(v.as_i32(), Some(&42));
        assert!(v.is_i32());
        assert_eq!(v.type_name(), "i32");
    }

    #[test]
    fn test_multi_variant_i64() {
        let v = MultiVariant::I64(42);
        assert_eq!(v.as_i64(), Some(&42));
        assert!(v.is_i64());
    }

    #[test]
    fn test_multi_variant_u32() {
        let v = MultiVariant::U32(42);
        assert_eq!(v.as_u32(), Some(&42));
        assert!(v.is_u32());
    }

    #[test]
    fn test_multi_variant_u64() {
        let v = MultiVariant::U64(42);
        assert_eq!(v.as_u64(), Some(&42));
        assert!(v.is_u64());
    }

    #[test]
    fn test_multi_variant_f32() {
        let v = MultiVariant::F32(42.0);
        assert_eq!(v.as_f32(), Some(&42.0));
        assert!(v.is_f32());
    }

    #[test]
    fn test_multi_variant_f64() {
        let v = MultiVariant::F64(42.0);
        assert_eq!(v.as_f64(), Some(&42.0));
        assert!(v.is_f64());
    }

    #[test]
    fn test_multi_variant_bool() {
        let v = MultiVariant::Bool(true);
        assert_eq!(v.as_bool(), Some(true));
        assert!(v.is_bool());
    }

    #[test]
    fn test_multi_variant_string() {
        let v = MultiVariant::String("hello".to_string());
        assert_eq!(v.as_str(), Some("hello"));
        assert!(v.is_string());
    }

    #[test]
    fn test_multi_variant_clone() {
        let v1 = MultiVariant::I32(42);
        let v2 = v1.clone();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_multi_variant_display() {
        let v = MultiVariant::I32(42);
        assert_eq!(format!("{}", v), "42");
    }

    #[test]
    fn test_multi_variant_default() {
        let v = MultiVariant::default();
        assert!(v.is_i32());
        assert_eq!(v.as_i32(), Some(&0));
    }

    #[test]
    fn test_multi_variant_from_i32() {
        let v: MultiVariant = 42i32.into();
        assert!(v.is_i32());
        assert_eq!(v.as_i32(), Some(&42));
    }

    #[test]
    fn test_multi_variant_from_string() {
        let v: MultiVariant = "hello".into();
        assert!(v.is_string());
        assert_eq!(v.as_str(), Some("hello"));
    }

    #[test]
    fn test_multi_variant_size() {
        let v = MultiVariant::I32(42);
        assert_eq!(v.size(), 4);
    }
}
