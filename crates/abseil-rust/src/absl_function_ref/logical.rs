//! Logical utilities - any_of, all_of, none_of

/// Creates a predicate that is true if any of the given predicates are true.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::any_of;
///
/// let is_even = |&x: &i32| x % 2 == 0;
/// let is_positive = |&x: &i32| x > 0;
/// let is_even_or_positive = any_of(&[is_even, is_positive]);
///
/// assert!(is_even_or_positive(&4));
/// assert!(is_even_or_positive(&3));
/// assert!(!is_even_or_positive(&-1));
/// ```
pub fn any_of<'a, T, F>(predicates: &'a [F]) -> impl Fn(&T) -> bool + 'a
where
    T: ?Sized,
    F: Fn(&T) -> bool + 'a,
{
    move |value| predicates.iter().any(|p| p(value))
}

/// Creates a predicate that is true only if all of the given predicates are true.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::all_of;
///
/// let is_positive = |&x: &i32| x > 0;
/// let is_even = |&x: &i32| x % 2 == 0;
/// let is_positive_and_even = all_of(&[is_positive, is_even]);
///
/// assert!(is_positive_and_even(&4));
/// assert!(!is_positive_and_even(&3));
/// assert!(!is_positive_and_even(&-2));
/// ```
pub fn all_of<'a, T, F>(predicates: &'a [F]) -> impl Fn(&T) -> bool + 'a
where
    T: ?Sized,
    F: Fn(&T) -> bool + 'a,
{
    move |value| predicates.iter().all(|p| p(value))
}

/// Creates a predicate that is true if none of the given predicates are true.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::none_of;
///
/// let is_negative = |&x: &i32| x < 0;
/// let is_zero = |&x: &i32| x == 0;
/// let is_positive = none_of(&[is_negative, is_zero]);
///
/// assert!(is_positive(&5));
/// assert!(!is_positive(&0));
/// assert!(!is_positive(&-5));
/// ```
pub fn none_of<'a, T, F>(predicates: &'a [F]) -> impl Fn(&T) -> bool + 'a
where
    T: ?Sized,
    F: Fn(&T) -> bool + 'a,
{
    move |value| predicates.iter().all(|p| !p(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_of() {
        let is_even = |&x: &i32| x % 2 == 0;
        let is_positive = |&x: &i32| x > 0;
        let is_even_or_positive = any_of(&[is_even, is_positive]);

        assert!(is_even_or_positive(&4));
        assert!(is_even_or_positive(&3));
        assert!(!is_even_or_positive(&-1));
    }

    #[test]
    fn test_all_of() {
        let is_positive = |&x: &i32| x > 0;
        let is_even = |&x: &i32| x % 2 == 0;
        let is_positive_and_even = all_of(&[is_positive, is_even]);

        assert!(is_positive_and_even(&4));
        assert!(!is_positive_and_even(&3));
        assert!(!is_positive_and_even(&-2));
    }

    #[test]
    fn test_none_of() {
        let is_negative = |&x: &i32| x < 0;
        let is_zero = |&x: &i32| x == 0;
        let is_positive = none_of(&[is_negative, is_zero]);

        assert!(is_positive(&5));
        assert!(!is_positive(&0));
        assert!(!is_positive(&-5));
    }
}
