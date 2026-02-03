//! Conditional execution utilities - when, branch

/// Executes a function only if a predicate is true.
///
/// Returns `Some(result)` if the predicate was true, `None` otherwise.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::when;
///
/// let is_positive = |&x: &i32| x > 0;
/// let double = |x| x * 2;
///
/// assert_eq!(when(&5, is_positive, double), Some(10));
/// assert_eq!(when(&-5, is_positive, double), None);
/// ```
#[inline]
pub fn when<'a, T, U, P, F>(value: &'a T, mut predicate: P, mut f: F) -> Option<U>
where
    P: FnMut(&T) -> bool,
    F: FnMut(&'a T) -> U,
{
    if predicate(value) {
        Some(f(value))
    } else {
        None
    }
}

/// Executes different functions based on a condition.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::branch;
///
/// let double = |x: i32| x * 2;
/// let triple = |x: i32| x * 3;
///
/// assert_eq!(branch(true, 5, double, triple), 10);
/// assert_eq!(branch(false, 5, double, triple), 15);
/// ```
#[inline]
pub fn branch<A, B>(condition: bool, value: A, mut if_true: impl FnMut(A) -> B, mut if_false: impl FnMut(A) -> B) -> B {
    if condition {
        if_true(value)
    } else {
        if_false(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_when() {
        let is_positive = |&x: &i32| x > 0;
        let double = |x| x * 2;

        assert_eq!(when(&5, is_positive, double), Some(10));
        assert_eq!(when(&-5, is_positive, double), None);
    }

    #[test]
    fn test_branch() {
        let double = |x: i32| x * 2;
        let triple = |x: i32| x * 3;

        assert_eq!(branch(true, 5, double, triple), 10);
        assert_eq!(branch(false, 5, double, triple), 15);
    }
}
