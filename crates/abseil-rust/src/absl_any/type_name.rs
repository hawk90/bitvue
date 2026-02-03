//! TypeName - Custom type names for type-erased values.

/// Trait for custom type names.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::TypeName;
///
/// struct MyType;
///
/// impl TypeName for MyType {
///     fn type_name() -> &'static str {
///         "MyType"
///     }
/// }
/// ```
pub trait TypeName {
    /// Returns the name of this type.
    fn type_name() -> &'static str;
}

impl<T: ?Sized> TypeName for T {
    default fn type_name() -> &'static str {
        core::any::type_name::<T>()
    }
}

/// Override for specific types to provide custom names.
impl TypeName for i32 {
    fn type_name() -> &'static str {
        "i32"
    }
}

impl TypeName for i64 {
    fn type_name() -> &'static str {
        "i64"
    }
}

impl TypeName for alloc::string::String {
    fn type_name() -> &'static str {
        "String"
    }
}

impl TypeName for &str {
    fn type_name() -> &'static str {
        "&str"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_name_i32() {
        assert_eq!(i32::type_name(), "i32");
    }

    #[test]
    fn test_type_name_string() {
        assert_eq!(alloc::string::String::type_name(), "String");
    }

    #[test]
    fn test_type_name_str() {
        assert_eq!(<str>::type_name(), "&str");
    }
}
