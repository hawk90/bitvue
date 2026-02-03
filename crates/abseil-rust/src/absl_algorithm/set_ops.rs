//! Merge and set operations - merge, merge_in_place, set_union, set_intersection, set_difference, set_symmetric_difference

use alloc::vec::Vec;

/// Merges two sorted slices into a new sorted vector.
///
/// Both input slices must be sorted in ascending order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::merge;
///
/// let a = &[1, 3, 5];
/// let b = &[2, 4, 6];
/// let merged = merge(a, b);
/// assert_eq!(merged, vec![1, 2, 3, 4, 5, 6]);
/// ```
#[inline]
pub fn merge<T: Ord>(a: &[T], b: &[T]) -> Vec<T>
where
    T: Clone,
{
    let mut result = Vec::with_capacity(a.len() + b.len());
    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        if a[i] <= b[j] {
            result.push(a[i].clone());
            i += 1;
        } else {
            result.push(b[j].clone());
            j += 1;
        }
    }

    while i < a.len() {
        result.push(a[i].clone());
        i += 1;
    }

    while j < b.len() {
        result.push(b[j].clone());
        j += 1;
    }

    result
}

/// Merges two sorted slices in place.
///
/// The destination slice must be large enough to hold both inputs.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::merge_in_place;
///
/// let mut a = vec![1, 3, 5, 0, 0, 0];
/// let b = vec![2, 4, 6];
/// merge_in_place(&mut a[..3], &b, &mut a);
/// assert_eq!(&a[..6], &[1, 2, 3, 4, 5, 6]);
/// ```
#[inline]
pub fn merge_in_place<T: Ord + Clone>(a: &mut [T], b: &[T], dest: &mut [T]) {
    let mut i = a.len();
    let mut j = b.len();
    let mut k = a.len() + b.len();

    // Merge from the end
    while i > 0 && j > 0 {
        k -= 1;
        if a[i - 1] > b[j - 1] {
            i -= 1;
            dest[k] = a[i].clone();
        } else {
            j -= 1;
            dest[k] = b[j].clone();
        }
    }

    // Copy remaining elements from a
    while i > 0 {
        k -= 1;
        i -= 1;
        dest[k] = a[i].clone();
    }

    // Copy remaining elements from b
    while j > 0 {
        k -= 1;
        j -= 1;
        dest[k] = b[j].clone();
    }
}

/// Computes the union of two sorted slices.
///
/// Returns a new vector containing all elements from both inputs.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::set_union;
///
/// let a = &[1, 2, 3, 5];
/// let b = &[2, 4, 5, 6];
/// let result = set_union(a, b);
/// assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
/// ```
#[inline]
pub fn set_union<T: Ord + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let mut result = Vec::with_capacity(a.len() + b.len());
    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        if a[i] < b[j] {
            result.push(a[i].clone());
            i += 1;
        } else if a[i] > b[j] {
            result.push(b[j].clone());
            j += 1;
        } else {
            result.push(a[i].clone());
            i += 1;
            j += 1;
        }
    }

    while i < a.len() {
        result.push(a[i].clone());
        i += 1;
    }

    while j < b.len() {
        result.push(b[j].clone());
        j += 1;
    }

    result
}

/// Computes the intersection of two sorted slices.
///
/// Returns a new vector containing elements common to both inputs.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::set_intersection;
///
/// let a = &[1, 2, 3, 5];
/// let b = &[2, 4, 5, 6];
/// let result = set_intersection(a, b);
/// assert_eq!(result, vec![2, 5]);
/// ```
#[inline]
pub fn set_intersection<T: Ord + Clone + PartialEq>(a: &[T], b: &[T]) -> Vec<T> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        if a[i] < b[j] {
            i += 1;
        } else if a[i] > b[j] {
            j += 1;
        } else {
            result.push(a[i].clone());
            i += 1;
            j += 1;
        }
    }

    result
}

/// Computes the difference of two sorted slices (elements in a but not in b).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::set_difference;
///
/// let a = &[1, 2, 3, 5];
/// let b = &[2, 4, 5, 6];
/// let result = set_difference(a, b);
/// assert_eq!(result, vec![1, 3]);
/// ```
#[inline]
pub fn set_difference<T: Ord + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        if a[i] < b[j] {
            result.push(a[i].clone());
            i += 1;
        } else if a[i] > b[j] {
            j += 1;
        } else {
            i += 1;
            j += 1;
        }
    }

    while i < a.len() {
        result.push(a[i].clone());
        i += 1;
    }

    result
}

/// Computes the symmetric difference of two sorted slices.
///
/// Returns elements that are in either a or b, but not both.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::set_symmetric_difference;
///
/// let a = &[1, 2, 3, 5];
/// let b = &[2, 4, 5, 6];
/// let result = set_symmetric_difference(a, b);
/// assert_eq!(result, vec![1, 3, 4, 6]);
/// ```
#[inline]
pub fn set_symmetric_difference<T: Ord + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        if a[i] < b[j] {
            result.push(a[i].clone());
            i += 1;
        } else if a[i] > b[j] {
            result.push(b[j].clone());
            j += 1;
        } else {
            i += 1;
            j += 1;
        }
    }

    while i < a.len() {
        result.push(a[i].clone());
        i += 1;
    }

    while j < b.len() {
        result.push(b[j].clone());
        j += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge() {
        let a = &[1, 3, 5];
        let b = &[2, 4, 6];
        let merged = merge(a, b);
        assert_eq!(merged, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_set_union() {
        let a = &[1, 2, 3, 5];
        let b = &[2, 4, 5, 6];
        let result = set_union(a, b);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_set_intersection() {
        let a = &[1, 2, 3, 5];
        let b = &[2, 4, 5, 6];
        let result = set_intersection(a, b);
        assert_eq!(result, vec![2, 5]);
    }

    #[test]
    fn test_set_difference() {
        let a = &[1, 2, 3, 5];
        let b = &[2, 4, 5, 6];
        let result = set_difference(a, b);
        assert_eq!(result, vec![1, 3]);
    }

    #[test]
    fn test_set_symmetric_difference() {
        let a = &[1, 2, 3, 5];
        let b = &[2, 4, 5, 6];
        let result = set_symmetric_difference(a, b);
        assert_eq!(result, vec![1, 3, 4, 6]);
    }
}
