//! Partition utilities - partition, is_partitioned, partition_point

/// Partitions a slice based on a predicate.
///
/// Elements that match the predicate are moved to the front.
/// Returns the number of elements for which the predicate returned true.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::partition;
///
/// let mut data = [1, 2, 3, 4, 5, 6];
/// let count = partition(&mut data, |&x| x % 2 == 0);
/// assert_eq!(count, 3);
/// assert_eq!(data[..count], [2, 4, 6]);
/// ```
#[inline]
pub fn partition<T, F>(slice: &mut [T], mut predicate: F) -> usize
where
    F: FnMut(&T) -> bool,
{
    let mut i = 0;
    for j in 0..slice.len() {
        if predicate(&slice[j]) {
            slice.swap(i, j);
            i += 1;
        }
    }
    i
}

/// Checks if a slice is partitioned according to a predicate.
///
/// Returns true if all elements matching the predicate come before
/// all elements that don't.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::is_partitioned;
///
/// assert!(is_partitioned(&[2, 4, 6, 1, 3, 5], |&x| x % 2 == 0));
/// assert!(!is_partitioned(&[1, 2, 4, 6, 3, 5], |&x| x % 2 == 0));
/// ```
#[inline]
pub fn is_partitioned<T, F>(slice: &[T], mut predicate: F) -> bool
where
    F: FnMut(&T) -> bool,
{
    let mut saw_false = false;
    for item in slice {
        if predicate(item) {
            if saw_false {
                return false;
            }
        } else {
            saw_false = true;
        }
    }
    true
}

/// Finds the partition point in a slice.
///
/// Assumes the slice is partitioned according to the predicate.
/// Returns the index of the first element where the predicate returns false.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::partition_point;
///
/// let data = [2, 4, 6, 1, 3, 5];
/// assert_eq!(partition_point(&data, |&x| x % 2 == 0), 3);
/// ```
#[inline]
pub fn partition_point<T, F>(slice: &[T], mut predicate: F) -> usize
where
    F: FnMut(&T) -> bool,
{
    slice.iter().take_while(|&x| predicate(x)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition() {
        let mut data = [1, 2, 3, 4, 5, 6];
        let count = partition(&mut data, |&x| x % 2 == 0);
        assert_eq!(count, 3);
        assert!(data[..count].iter().all(|&x| x % 2 == 0));
        assert!(data[count..].iter().all(|&x| x % 2 != 0));
    }

    #[test]
    fn test_is_partitioned() {
        assert!(is_partitioned(&[2, 4, 6, 1, 3, 5], |&x| x % 2 == 0));
        assert!(!is_partitioned(&[1, 2, 4, 6, 3, 5], |&x| x % 2 == 0));
    }

    #[test]
    fn test_partition_point() {
        let data = [2, 4, 6, 1, 3, 5];
        assert_eq!(partition_point(&data, |&x| x % 2 == 0), 3);
    }
}
