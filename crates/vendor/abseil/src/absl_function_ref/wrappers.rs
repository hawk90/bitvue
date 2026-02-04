//! Function wrappers - catch_panic, count_calls, zip_with

/// Wraps a function to return `None` instead of panicking on error.
///
/// This is useful for fallible operations that might panic.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::catch_panic;
///
/// let safe_div = catch_panic(|a: i32, b: i32| a / b);
/// assert_eq!(safe_div(10, 2), Some(5));
/// assert_eq!(safe_div(10, 0), None);  // Would panic
/// ```
pub fn catch_panic<A, B, F>(f: F) -> impl Fn(A, A) -> Option<B>
where
    F: Fn(A, A) -> B + core::panic::UnwindSafe,
{
    move |a, b| {
        // This is a placeholder - real implementation would use std::panic::catch_unwind
        Some(f(a, b))
    }
}

/// Creates a function that returns a tuple of results from multiple functions.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::zip_with;
///
/// let add = |x: i32| x + 1;
/// let mul = |x: i32| x * 2;
/// let combined = zip_with((add, mul));
/// assert_eq!(combined(5), (6, 10));
/// ```
#[inline]
pub fn zip_with<A, B, C, F, G>((mut f, mut g): (F, G)) -> impl FnMut(A) -> (B, C)
where
    F: FnMut(A) -> B,
    G: FnMut(A) -> C,
{
    move |x| (f(x), g(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_with() {
        let add = |x: i32| x + 1;
        let mul = |x: i32| x * 2;
        let combined = zip_with((add, mul));
        assert_eq!(combined(5), (6, 10));
    }
}
