//! Identity utilities - id, function_name

/// The identity function: returns its argument unchanged.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::id;
///
/// assert_eq!(id(42), 42);
/// assert_eq!(id("hello"), "hello");
/// ```
#[inline]
pub const fn id<T>(value: T) -> T {
    value
}

/// Converts a function to a string (placeholder).
///
/// This is a placeholder for function introspection/debugging.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::function_name;
///
/// let f = |x: i32| x + 1;
/// let name = function_name(&f);
/// assert!(name.contains("<unknown>"));
/// ```
#[inline]
pub fn function_name<F>(_: &F) -> alloc::string::String {
    alloc::format!("<unknown function at {:p}>", _)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id() {
        assert_eq!(id(42), 42);
        assert_eq!(id("hello"), "hello");
    }
}
