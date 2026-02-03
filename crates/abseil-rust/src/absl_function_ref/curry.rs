//! Currying and partial application utilities - curry, apply_partial, flip

/// Curries a binary function into a chain of unary functions.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::curry;
///
/// let add = |x: i32, y: i32| x + y;
/// let add_5 = curry(add)(5);
/// assert_eq!(add_5(10), 15);
/// ```
#[inline]
pub fn curry<A, B, C, F>(f: F) -> impl Fn(A) -> impl Fn(B) -> C
where
    F: Fn(A, B) -> C,
{
    move |a| move |b| f(a, b)
}

/// Partially applies a function to its first argument.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::apply_partial;
///
/// let add = |x: i32, y: i32| x + y;
/// let add_5 = apply_partial(add, 5);
/// assert_eq!(add_5(10), 15);
/// ```
#[inline]
pub fn apply_partial<A, B, C, F>(mut f: F, a: A) -> impl FnMut(B) -> C
where
    F: FnMut(A, B) -> C,
{
    move |b| f(a, b)
}

/// Flips the arguments of a binary function.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::flip;
///
/// let subtract = |x: i32, y: i32| x - y;
/// let flipped = flip(subtract);
/// assert_eq!(flipped(5, 10), 5);  // 10 - 5 = 5
/// ```
#[inline]
pub fn flip<A, B, C, F>(mut f: F) -> impl FnMut(B, A) -> C
where
    F: FnMut(A, B) -> C,
{
    move |b, a| f(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curry() {
        let add = |x: i32, y: i32| x + y;
        let add_5 = curry(add)(5);
        assert_eq!(add_5(10), 15);
    }

    #[test]
    fn test_apply_partial() {
        let subtract = |x: i32, y: i32| x - y;
        let subtract_from_10 = apply_partial(subtract, 10);
        assert_eq!(subtract_from_10(3), 7);
    }

    #[test]
    fn test_flip() {
        let subtract = |x: i32, y: i32| x - y;
        let flipped = flip(subtract);
        assert_eq!(flipped(5, 10), 5);  // 10 - 5 = 5
    }
}
