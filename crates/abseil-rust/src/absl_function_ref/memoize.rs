//! Memoization utilities - memoize, memoize_with

use core::cell::RefCell;

/// Memoizes a function based on a single argument.
///
/// Note: This is a simple implementation that stores results in a `RefCell`.
/// For production use, consider a more sophisticated caching strategy.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::memoize;
///
/// let expensive = memoize(|x: i32| x * x);
/// assert_eq!(expensive(5), 25);
/// // Second call uses cached result
/// assert_eq!(expensive(5), 25);
/// ```
pub fn memoize<A, B, F>(f: F) -> impl FnMut(A) -> B
where
    A: PartialEq + core::hash::Hash + Clone,
    B: Clone,
    F: Fn(A) -> B,
{
    let cache = RefCell::alloc::collections::HashMap::new();
    move |input| {
        let mut cache = cache.borrow_mut();
        if let Some(result) = cache.get(&input) {
            return result.clone();
        }
        let result = f(input.clone());
        cache.insert(input, result.clone());
        result
    }
}

/// Memoizes a function with a custom hash function.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::memoize_with;
///
/// let compute = |x: i32| x * x;
/// let memoized = memoize_with(compute, |x| x as u64);
/// assert_eq!(memoized(5), 25);
/// ```
pub fn memoize_with<A, B, F, H>(f: F, _hash: H) -> impl FnMut(A) -> B
where
    A: PartialEq + Clone,
    B: Clone,
    F: Fn(A) -> B,
    H: Fn(&A) -> u64,
{
    let cache = RefCell::alloc::collections::HashMap::new();
    move |input| {
        let mut cache = cache.borrow_mut();
        if let Some(result) = cache.get(&input) {
            return result.clone();
        }
        let result = f(input.clone());
        cache.insert(input, result.clone());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memoize() {
        let expensive = memoize(|x: i32| x * x);
        assert_eq!(expensive(5), 25);
        assert_eq!(expensive(5), 25);
    }
}
