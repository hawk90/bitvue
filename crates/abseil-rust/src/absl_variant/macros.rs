//! Macros for variant pattern matching and creation.

/// Pattern matching macro for variants.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::{variant_match, MultiVariant};
///
/// let v = MultiVariant::I32(42);
/// let result = variant_match!(v, {
///     MultiVariant::I32(x) => format!("int: {}", x),
///     MultiVariant::String(s) => format!("string: {}", s),
///     _ => "other".to_string(),
/// });
/// assert_eq!(result, "int: 42");
/// ```
#[macro_export]
macro_rules! variant_match {
    ($variant:expr, { $(MultiVariant::$variant:ident($pattern:pat) => $result:expr,)+ _ => $default:expr }) => {
        match $variant {
            $(MultiVariant::$variant($pattern) => $result,)*
            _ => $default,
        }
    };
    ($variant:expr, { $(MultiVariant::$variant:ident($pattern:pat) => $result:expr,)+ }) => {
        match $variant {
            $(MultiVariant::$variant($pattern) => $result,)*
        }
    };
}

/// Creates a variant from a value, inferring the type.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::variant;
///
/// let v = variant!(42);
/// assert!(v.is_i32());
///
/// let v = variant!("hello");
/// assert!(v.is_string());
///
/// let v = variant!(3.14);
/// assert!(v.is_f64());
/// ```
#[macro_export]
macro_rules! variant {
    ($value:expr) => {
        match $value {
            v @ (i8 | i16 | i32) => $crate::absl_variant::MultiVariant::I32(v as i32),
            v @ (i64 | i128) => $crate::absl_variant::MultiVariant::I64(v as i64),
            v @ (u8 | u16 | u32) => $crate::absl_variant::MultiVariant::U32(v as u32),
            v @ (u64 | u128 | usize) => $crate::absl_variant::MultiVariant::U64(v as u64),
            v @ f32 => $crate::absl_variant::MultiVariant::F32(v),
            v @ f64 => $crate::absl_variant::MultiVariant::F64(v),
            v @ bool => $crate::absl_variant::MultiVariant::Bool(v),
            v @ &str => $crate::absl_variant::MultiVariant::String(v.to_string()),
            v => $crate::absl_variant::MultiVariant::String(v.to_string()),
        }
    };
}
