//! Function composition utilities - compose, pipe, constant, complement

/// Composes two functions: `compose(g, f)(x) = g(f(x))`.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::compose;
///
/// let add_one = |x: i32| x + 1;
/// let double = |x: i32| x * 2;
/// let add_one_then_double = compose(double, add_one);
/// assert_eq!(add_one_then_double(5), 12);
/// ```
#[inline]
pub fn compose<A, B, C, F, G>(mut g: G, mut f: F) -> impl FnMut(A) -> C
where
    F: FnMut(A) -> B,
    G: FnMut(B) -> C,
{
    move |x| g(f(x))
}

/// Composes two functions in reverse order: `pipe(f, g)(x) = g(f(x))`.
///
/// This is the same as `compose` but with arguments in the opposite order,
/// which can be more intuitive for left-to-right reading.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::pipe;
///
/// let add_one = |x: i32| x + 1;
/// let double = |x: i32| x * 2;
/// let add_one_then_double = pipe(add_one, double);
/// assert_eq!(add_one_then_double(5), 12);
/// ```
#[inline]
pub fn pipe<A, B, C, F, G>(mut f: F, mut g: G) -> impl FnMut(A) -> C
where
    F: FnMut(A) -> B,
    G: FnMut(B) -> C,
{
    move |x| g(f(x))
}

/// Creates a function that always returns a constant value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::constant;
///
/// let always_42 = constant(42);
/// assert_eq!(always_42(), 42);
/// assert_eq!(always_42(), 42);
/// ```
#[inline]
pub const fn constant<T: Copy>(value: T) -> impl Fn() -> T {
    move || value
}

/// Negates a predicate function.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::complement;
///
/// let is_positive = |x: &i32| *x > 0;
/// let is_not_positive = complement(is_positive);
/// assert!(is_positive(&5));
/// assert!(!is_not_positive(&5));
/// assert!(!is_positive(&-5));
/// assert!(is_not_positive(&-5));
/// ```
#[inline]
pub fn complement<T, F>(mut f: F) -> impl FnMut(&T) -> bool
where
    F: FnMut(&T) -> bool,
{
    move |x| !f(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compose() {
        let add_one = |x: i32| x + 1;
        let double = |x: i32| x * 2;
        let add_one_then_double = compose(double, add_one);
        assert_eq!(add_one_then_double(5), 12);
    }

    #[test]
    fn test_pipe() {
        let add_one = |x: i32| x + 1;
        let double = |x: i32| x * 2;
        let add_one_then_double = pipe(add_one, double);
        assert_eq!(add_one_then_double(5), 12);
    }

    #[test]
    fn test_constant() {
        let always_42 = constant(42);
        assert_eq!(always_42(), 42);
        assert_eq!(always_42(), 42);
    }

    #[test]
    fn test_complement() {
        let is_positive = |x: &i32| *x > 0;
        let is_not_positive = complement(is_positive);
        assert!(is_positive(&5));
        assert!(!is_not_positive(&5));
        assert!(!is_positive(&-5));
        assert!(is_not_positive(&-5));
    }
}
