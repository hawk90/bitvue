//! Quick sort implementation.


extern crate alloc;

use core::cmp::Ordering;

// Threshold for switching to insertion sort
const INSERTION_SORT_THRESHOLD: usize = 16;

/// Unstable quick sort.
pub fn quicksort<T: Ord>(slice: &mut [T]) {
    if slice.len() <= 1 {
        return;
    }

    // Use insertion sort for small arrays (faster for small n)
    if slice.len() < INSERTION_SORT_THRESHOLD {
        insertion_sort(slice);
        return;
    }

    let pivot_idx = partition(slice);

    let (left, right) = slice.split_at_mut(pivot_idx);
    quicksort(left);
    if right.len() > 1 {
        quicksort(&mut right[1..]);
    }
}

/// Insertion sort for small arrays.
///
/// This is more efficient than quicksort for small arrays due to
/// lower overhead and better cache locality.
fn insertion_sort<T: Ord>(slice: &mut [T]) {
    for i in 1..slice.len() {
        let mut j = i;
        while j > 0 && slice[j - 1] > slice[j] {
            slice.swap(j, j - 1);
            j -= 1;
        }
    }
}

fn partition<T: Ord>(slice: &mut [T]) -> usize {
    let len = slice.len();
    let pivot_idx = len / 2;
    let pivot_idx = median_of_three(slice, 0, len - 1, pivot_idx);

    slice.swap(pivot_idx, len - 1);

    let mut store_idx = 0;
    for i in 0..len - 1 {
        if slice[i] < slice[len - 1] {
            slice.swap(i, store_idx);
            store_idx += 1;
        }
    }

    slice.swap(store_idx, len - 1);
    store_idx
}

fn median_of_three<T: Ord>(slice: &[T], a: usize, b: usize, c: usize) -> usize {
    if slice[a] < slice[b] {
        if slice[b] < slice[c] {
            b
        } else if slice[a] < slice[c] {
            c
        } else {
            a
        }
    } else {
        if slice[a] < slice[c] {
            a
        } else if slice[b] < slice[c] {
            c
        } else {
            b
        }
    }
}

/// Quick sort with custom comparison function.
pub fn quicksort_by<T, F>(slice: &mut [T], mut compare: F)
where
    F: FnMut(&T, &T) -> Ordering,
{
    // Use insertion sort for small arrays
    if slice.len() < INSERTION_SORT_THRESHOLD {
        insertion_sort_by(slice, &mut compare);
        return;
    }

    // Use iterative approach to avoid recursive reference issues
    if slice.len() <= 1 {
        return;
    }

    let mut stack = alloc::vec::Vec::new();
    stack.push((0, slice.len()));

    while let Some((start, end)) = stack.pop() {
        if end - start <= 1 {
            continue;
        }

        // Use insertion sort for small partitions
        if end - start < INSERTION_SORT_THRESHOLD {
            insertion_sort_by(&mut slice[start..end], &mut compare);
            continue;
        }

        let pivot_idx = partition_by_range(&mut slice[start..end], &mut compare);
        let pivot_idx = start + pivot_idx;

        // Push smaller partition first to limit stack size
        if pivot_idx - start < end - pivot_idx {
            if pivot_idx - start > 1 {
                stack.push((start, pivot_idx));
            }
            if end - pivot_idx > 1 {
                stack.push((pivot_idx + 1, end));
            }
        } else {
            if end - pivot_idx > 1 {
                stack.push((pivot_idx + 1, end));
            }
            if pivot_idx - start > 1 {
                stack.push((start, pivot_idx));
            }
        }
    }
}

/// Insertion sort with custom comparison function.
fn insertion_sort_by<T, F>(slice: &mut [T], compare: &mut F)
where
    F: FnMut(&T, &T) -> Ordering,
{
    for i in 1..slice.len() {
        let mut j = i;
        while j > 0 && compare(&slice[j - 1], &slice[j]) == Ordering::Greater {
            slice.swap(j, j - 1);
            j -= 1;
        }
    }
}

fn partition_by_range<T, F>(slice: &mut [T], compare: &mut F) -> usize
where
    F: FnMut(&T, &T) -> Ordering,
{
    let len = slice.len();
    let pivot_idx = len / 2;
    slice.swap(pivot_idx, len - 1);

    let mut store_idx = 0;
    for i in 0..len - 1 {
        if compare(&slice[i], &slice[len - 1]) != Ordering::Greater {
            slice.swap(i, store_idx);
            store_idx += 1;
        }
    }

    slice.swap(store_idx, len - 1);
    store_idx
}

fn partition_by<T, F>(slice: &mut [T], compare: &mut F) -> usize
where
    F: FnMut(&T, &T) -> Ordering,
{
    let len = slice.len();
    let pivot_idx = len / 2;
    slice.swap(pivot_idx, len - 1);

    let mut store_idx = 0;
    for i in 0..len - 1 {
        if compare(&slice[i], &slice[len - 1]) != Ordering::Greater {
            slice.swap(i, store_idx);
            store_idx += 1;
        }
    }

    slice.swap(store_idx, len - 1);
    store_idx
}

/// Alias for quicksort (unstable sort).
pub fn unstable_sort<T: Ord>(slice: &mut [T]) {
    quicksort(slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quicksort() {
        let mut data = vec![5, 2, 8, 1, 9, 3];
        quicksort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 5, 8, 9]);
    }

    #[test]
    fn test_quicksort_small_array() {
        // Test that small arrays use insertion sort
        let mut data = vec![3, 1, 2];
        quicksort(&mut data);
        assert_eq!(data, vec![1, 2, 3]);
    }

    #[test]
    fn test_quicksort_single_element() {
        let mut data = vec![42];
        quicksort(&mut data);
        assert_eq!(data, vec![42]);
    }

    #[test]
    fn test_quicksort_empty() {
        let mut data: Vec<i32> = vec![];
        quicksort(&mut data);
        assert_eq!(data, vec![]);
    }

    #[test]
    fn test_quicksort_by() {
        let mut data = vec![5, 2, 8, 1, 9];
        quicksort_by(&mut data, |a, b| b.cmp(a));
        assert_eq!(data, vec![9, 8, 5, 2, 1]);
    }

    #[test]
    fn test_quicksort_by_small() {
        let mut data = vec![3, 1, 2];
        quicksort_by(&mut data, |a, b| b.cmp(a));
        assert_eq!(data, vec![3, 2, 1]);
    }

    #[test]
    fn test_unstable_sort() {
        let mut data = vec![5, 2, 8, 1, 9];
        unstable_sort(&mut data);
        assert_eq!(data, vec![1, 2, 5, 8, 9]);
    }

    #[test]
    fn test_insertion_sort() {
        let mut data = vec![5, 2, 8, 1, 9, 3, 7];
        insertion_sort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 5, 7, 8, 9]);
    }

    #[test]
    fn test_insertion_sort_already_sorted() {
        let mut data = vec![1, 2, 3, 4, 5];
        insertion_sort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_insertion_sort_reverse() {
        let mut data = vec![5, 4, 3, 2, 1];
        insertion_sort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }
}
