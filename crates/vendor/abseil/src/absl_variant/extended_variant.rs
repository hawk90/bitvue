//! ExtendedVariant - A variant supporting more primitive types.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// An extended variant that supports more primitive types.
#[derive(Clone, Debug, PartialEq)]
pub enum ExtendedVariant {
    /// 8-bit signed integer
    I8(i8),
    /// 16-bit signed integer
    I16(i16),
    /// 32-bit signed integer
    I32(i32),
    /// 64-bit signed integer
    I64(i64),
    /// 128-bit signed integer
    I128(i128),
    /// 8-bit unsigned integer
    U8(u8),
    /// 16-bit unsigned integer
    U16(u16),
    /// 32-bit unsigned integer
    U32(u32),
    /// 64-bit unsigned integer
    U64(u64),
    /// 128-bit unsigned integer
    U128(u128),
    /// 32-bit floating point
    F32(f32),
    /// 64-bit floating point
    F64(f64),
    /// Boolean
    Bool(bool),
    /// Character
    Char(char),
    /// String
    String(String),
    /// Byte slice
    Bytes(Vec<u8>),
    /// Unit/empty
    Unit,
}

impl ExtendedVariant {
    /// Returns the type name of the contained value.
    pub fn type_name(&self) -> &'static str {
        match self {
            ExtendedVariant::I8(_) => "i8",
            ExtendedVariant::I16(_) => "i16",
            ExtendedVariant::I32(_) => "i32",
            ExtendedVariant::I64(_) => "i64",
            ExtendedVariant::I128(_) => "i128",
            ExtendedVariant::U8(_) => "u8",
            ExtendedVariant::U16(_) => "u16",
            ExtendedVariant::U32(_) => "u32",
            ExtendedVariant::U64(_) => "u64",
            ExtendedVariant::U128(_) => "u128",
            ExtendedVariant::F32(_) => "f32",
            ExtendedVariant::F64(_) => "f64",
            ExtendedVariant::Bool(_) => "bool",
            ExtendedVariant::Char(_) => "char",
            ExtendedVariant::String(_) => "string",
            ExtendedVariant::Bytes(_) => "bytes",
            ExtendedVariant::Unit => "unit",
        }
    }

    /// Returns the size of the contained value in bytes.
    pub fn size(&self) -> usize {
        match self {
            ExtendedVariant::I8(_) => core::mem::size_of::<i8>(),
            ExtendedVariant::I16(_) => core::mem::size_of::<i16>(),
            ExtendedVariant::I32(_) => core::mem::size_of::<i32>(),
            ExtendedVariant::I64(_) => core::mem::size_of::<i64>(),
            ExtendedVariant::I128(_) => core::mem::size_of::<i128>(),
            ExtendedVariant::U8(_) => core::mem::size_of::<u8>(),
            ExtendedVariant::U16(_) => core::mem::size_of::<u16>(),
            ExtendedVariant::U32(_) => core::mem::size_of::<u32>(),
            ExtendedVariant::U64(_) => core::mem::size_of::<u64>(),
            ExtendedVariant::U128(_) => core::mem::size_of::<u128>(),
            ExtendedVariant::F32(_) => core::mem::size_of::<f32>(),
            ExtendedVariant::F64(_) => core::mem::size_of::<f64>(),
            ExtendedVariant::Bool(_) => core::mem::size_of::<bool>(),
            ExtendedVariant::Char(_) => core::mem::size_of::<char>(),
            ExtendedVariant::String(s) => s.len(),
            ExtendedVariant::Bytes(b) => b.len(),
            ExtendedVariant::Unit => 0,
        }
    }

    /// Attempts to convert to i64.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ExtendedVariant::I8(v) => Some(*v as i64),
            ExtendedVariant::I16(v) => Some(*v as i64),
            ExtendedVariant::I32(v) => Some(*v as i64),
            ExtendedVariant::I64(v) => Some(*v),
            ExtendedVariant::U8(v) => Some(*v as i64),
            ExtendedVariant::U16(v) => Some(*v as i64),
            ExtendedVariant::U32(v) => Some(*v as i64),
            _ => None,
        }
    }

    /// Attempts to convert to u64.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            ExtendedVariant::U8(v) => Some(*v as u64),
            ExtendedVariant::U16(v) => Some(*v as u64),
            ExtendedVariant::U32(v) => Some(*v as u64),
            ExtendedVariant::U64(v) => Some(*v),
            ExtendedVariant::I8(v) if *v >= 0 => Some(*v as u64),
            ExtendedVariant::I16(v) if *v >= 0 => Some(*v as u64),
            ExtendedVariant::I32(v) if *v >= 0 => Some(*v as u64),
            ExtendedVariant::I64(v) if *v >= 0 => Some(*v as u64),
            _ => None,
        }
    }

    /// Attempts to convert to f64.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ExtendedVariant::F32(v) => Some(*v as f64),
            ExtendedVariant::F64(v) => Some(*v),
            ExtendedVariant::I8(v) => Some(*v as f64),
            ExtendedVariant::I16(v) => Some(*v as f64),
            ExtendedVariant::I32(v) => Some(*v as f64),
            ExtendedVariant::I64(v) => Some(*v as f64),
            ExtendedVariant::U8(v) => Some(*v as f64),
            ExtendedVariant::U16(v) => Some(*v as f64),
            ExtendedVariant::U32(v) => Some(*v as f64),
            ExtendedVariant::U64(v) => Some(*v as f64),
            _ => None,
        }
    }
}

impl Default for ExtendedVariant {
    fn default() -> Self {
        ExtendedVariant::Unit
    }
}

impl fmt::Display for ExtendedVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtendedVariant::I8(v) => write!(f, "{}", v),
            ExtendedVariant::I16(v) => write!(f, "{}", v),
            ExtendedVariant::I32(v) => write!(f, "{}", v),
            ExtendedVariant::I64(v) => write!(f, "{}", v),
            ExtendedVariant::I128(v) => write!(f, "{}", v),
            ExtendedVariant::U8(v) => write!(f, "{}", v),
            ExtendedVariant::U16(v) => write!(f, "{}", v),
            ExtendedVariant::U32(v) => write!(f, "{}", v),
            ExtendedVariant::U64(v) => write!(f, "{}", v),
            ExtendedVariant::U128(v) => write!(f, "{}", v),
            ExtendedVariant::F32(v) => write!(f, "{}", v),
            ExtendedVariant::F64(v) => write!(f, "{}", v),
            ExtendedVariant::Bool(v) => write!(f, "{}", v),
            ExtendedVariant::Char(v) => write!(f, "{}", v),
            ExtendedVariant::String(v) => write!(f, "{}", v),
            ExtendedVariant::Bytes(v) => write!(f, "{:?}", v),
            ExtendedVariant::Unit => write!(f, "()"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extended_variant_i8() {
        let v = ExtendedVariant::I8(42);
        assert_eq!(v.type_name(), "i8");
        assert_eq!(v.size(), 1);
    }

    #[test]
    fn test_extended_variant_i128() {
        let v = ExtendedVariant::I128(42);
        assert_eq!(v.type_name(), "i128");
        assert_eq!(v.size(), 16);
    }

    #[test]
    fn test_extended_variant_char() {
        let v = ExtendedVariant::Char('a');
        assert_eq!(v.type_name(), "char");
        assert_eq!(format!("{}", v), "a");
    }

    #[test]
    fn test_extended_variant_bytes() {
        let v = ExtendedVariant::Bytes(vec![1, 2, 3]);
        assert_eq!(v.type_name(), "bytes");
        assert_eq!(v.size(), 3);
    }

    #[test]
    fn test_extended_variant_unit() {
        let v = ExtendedVariant::Unit;
        assert_eq!(v.type_name(), "unit");
        assert_eq!(v.size(), 0);
        assert_eq!(format!("{}", v), "()");
    }

    #[test]
    fn test_extended_variant_as_i64() {
        assert_eq!(ExtendedVariant::I8(42).as_i64(), Some(42));
        assert_eq!(ExtendedVariant::I16(42).as_i64(), Some(42));
        assert_eq!(ExtendedVariant::I32(42).as_i64(), Some(42));
        assert_eq!(ExtendedVariant::I64(42).as_i64(), Some(42));
        assert_eq!(ExtendedVariant::U8(42).as_i64(), Some(42));
        assert_eq!(ExtendedVariant::String("hello".to_string()).as_i64(), None);
    }

    #[test]
    fn test_extended_variant_as_u64() {
        assert_eq!(ExtendedVariant::U8(42).as_u64(), Some(42));
        assert_eq!(ExtendedVariant::U16(42).as_u64(), Some(42));
        assert_eq!(ExtendedVariant::U32(42).as_u64(), Some(42));
        assert_eq!(ExtendedVariant::U64(42).as_u64(), Some(42));
        assert_eq!(ExtendedVariant::I8(-1).as_u64(), None);
    }

    #[test]
    fn test_extended_variant_as_f64() {
        assert_eq!(ExtendedVariant::F32(3.14).as_f64(), Some(3.14));
        assert_eq!(ExtendedVariant::F64(3.14).as_f64(), Some(3.14));
        assert_eq!(ExtendedVariant::I32(42).as_f64(), Some(42.0));
    }
}
