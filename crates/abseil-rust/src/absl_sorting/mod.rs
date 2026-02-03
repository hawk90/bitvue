//! Advanced sorting algorithms.
//!
//! This module provides advanced sorting algorithms similar to those found
//! in Abseil's sorting utilities and beyond.
//!
//! # Overview
//!
//! Sorting algorithms provide various ways to order collections efficiently.
//! This module includes:
//!
//! - Merge sort variations
//! - Quick sort variations
//! - Heap sort
//! - Radix sort
//! - Natural sort for human-friendly string ordering
//! - Specialized sorts for small arrays
//!
//! # Components
//!
//! - [`mergesort`] - Merge sort and variations
//! - [`quicksort`] - Quick sort and variations
//! - [`heapsort`] - Heap sort implementation
//! - [`radix_sort`] - Radix sort for integers
//! - [`natural_sort`] - Natural sort for human-friendly ordering
//! - [`specialized`] - Specialized sorts for specific data types
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_sorting::mergesort;
//!
//! let mut data = vec![5, 2, 8, 1, 9];
//! mergesort(&mut data);
//! assert_eq!(data, vec![1, 2, 5, 8, 9]);
//! ```


extern crate alloc;

use alloc::vec::Vec;
use core::mem::MaybeUninit;

pub mod mergesort;
pub mod quicksort;
pub mod heapsort;
pub mod radix_sort;
pub mod specialized;
pub mod hybrid;
pub mod natural_sort;
pub mod additional_sorts;

// Re-exports
pub use mergesort::{mergesort, mergesort_by, stable_sort};
pub use quicksort::{quicksort, quicksort_by, unstable_sort};
pub use heapsort::heapsort;
pub use radix_sort::{radix_sort, radix_sort_u8, radix_sort_u16, radix_sort_u32};
pub use specialized::{small_sort, insertion_sort, bubble_sort};
pub use hybrid::{introsort, timsort};
pub use natural_sort::{natural_cmp, natural_sort, natural_sort_string, natural_sort_by};
pub use additional_sorts::{
    selection_sort, selection_sort_by, shell_sort, cycle_sort, comb_sort, gnome_sort,
    cocktail_sort, odd_even_sort, stooge_sort,
};

/// Sorts a slice using the default sorting algorithm.
///
/// This is currently merge sort for stability.
pub fn sort<T: Ord>(slice: &mut [T]) {
    mergesort(slice);
}

/// Sorts a slice with a custom comparison function.
pub fn sort_by<T, F>(slice: &mut [T], compare: F)
where
    F: FnMut(&T, &T) -> core::cmp::Ordering,
{
    mergesort_by(slice, compare);
}

/// Checks if a slice is sorted.
pub fn is_sorted<T: Ord>(slice: &[T]) -> bool {
    slice.windows(2).all(|w| w[0] <= w[1])
}

/// Checks if a slice is sorted with a custom comparison function.
pub fn is_sorted_by<T, F>(slice: &[T], mut compare: F) -> bool
where
    F: FnMut(&T, &T) -> core::cmp::Ordering,
{
    slice.windows(2).all(|w| compare(&w[0], &w[1]) != core::cmp::Ordering::Greater)
}

/// Finds the minimum element in a slice.
pub fn min<T: Ord>(slice: &[T]) -> Option<&T> {
    slice.iter().min()
}

/// Finds the maximum element in a slice.
pub fn max<T: Ord>(slice: &[T]) -> Option<&T> {
    slice.iter().max()
}

/// Finds the minimum and maximum elements in a slice.
pub fn min_max<T: Ord>(slice: &[T]) -> Option<(&T, &T)> {
    if slice.is_empty() {
        return None;
    }

    let mut min = &slice[0];
    let mut max = &slice[0];

    for item in &slice[1..] {
        if item < min {
            min = item;
        }
        if item > max {
            max = item;
        }
    }

    Some((min, max))
}

/// Selects the k-th smallest element (quickselect).
pub fn select<T: Ord>(slice: &mut [T], k: usize) -> Option<&T> {
    if k >= slice.len() {
        return None;
    }

    let mut left = 0;
    let mut right = slice.len();

    while left < right {
        let pivot = partition(slice, left, right);

        if k == pivot {
            return Some(&slice[k]);
        } else if k < pivot {
            right = pivot;
        } else {
            left = pivot + 1;
        }
    }

    Some(&slice[k])
}

/// Partition function used by quicksort/quickselect.
///
/// # Panics
///
/// Panics if `left >= right` (invalid range).
fn partition<T: Ord>(slice: &mut [T], left: usize, right: usize) -> usize {
    // SAFETY: Validate range to prevent integer underflow
    if left >= right {
        panic!(
            "partition: invalid range left={} >= right={}, slice.len()={}",
            left, right, slice.len()
        );
    }
    if right > slice.len() {
        panic!(
            "partition: right={} exceeds slice.len()={}",
            right, slice.len()
        );
    }

    // SAFETY: left < right is guaranteed by the check above,
    // so (right - left) won't underflow
    let pivot_idx = left + (right - left) / 2;
    let pivot_idx = partition_pivot(slice, left, right, pivot_idx);

    // Move pivot to end
    slice.swap(pivot_idx, right - 1);

    let mut store_idx = left;
    // SAFETY: right - 1 >= left since right > left
    for i in left..right - 1 {
        if slice[i] < slice[right - 1] {
            slice.swap(i, store_idx);
            store_idx += 1;
        }
    }

    // Move pivot to final position
    slice.swap(store_idx, right - 1);
    store_idx
}

/// Median-of-three pivot selection.
///
/// # Panics
///
/// Panics if indices are invalid or out of bounds.
fn partition_pivot<T: Ord>(slice: &mut [T], left: usize, right: usize, pivot_idx: usize) -> usize {
    // SAFETY: Validate indices to prevent integer underflow/out-of-bounds access
    if left >= right {
        panic!(
            "partition_pivot: invalid range left={} >= right={}",
            left, right
        );
    }
    if right > slice.len() {
        panic!(
            "partition_pivot: right={} exceeds slice.len()={}",
            right, slice.len()
        );
    }
    if pivot_idx >= right {
        panic!(
            "partition_pivot: pivot_idx={} >= right={}",
            pivot_idx, right
        );
    }

    // SAFETY: left < right is guaranteed, so (right - left) won't underflow
    let mid = left + (right - left) / 2;

    // Order left, mid, pivot_idx
    if slice[mid] < slice[left] {
        slice.swap(left, mid);
    }
    if slice[right - 1] < slice[left] {
        slice.swap(left, right - 1);
    }
    if slice[right - 1] < slice[mid] {
        slice.swap(mid, right - 1);
    }

    mid
}

/// Reverses a slice.
pub fn reverse<T>(slice: &mut [T]) {
    let len = slice.len();
    for i in 0..len / 2 {
        slice.swap(i, len - 1 - i);
    }
}

/// Rotates a slice left by `mid` positions.
pub fn rotate_left<T>(slice: &mut [T], mid: usize) {
    if mid == 0 || mid >= slice.len() {
        return;
    }

    let len = slice.len();
    // SAFETY: We use MaybeUninit to safely handle types with destructors.
    // The values are moved from slice to temp, then moved back to slice.
    // Each value is moved exactly once, ensuring proper drop semantics.
    let mut temp: Vec<MaybeUninit<T>> = Vec::with_capacity(mid);

    // Save first `mid` elements
    // SAFETY: i is in bounds (0..mid) which is < len
    for i in 0..mid {
        unsafe {
            temp.push(MaybeUninit::new(core::ptr::read(&slice[i])));
        }
    }

    // Shift remaining elements left using slice rotation
    // SAFETY:
    // - Loop invariant: `mid <= i < len` is guaranteed by `mid..len` range
    // - `slice.as_ptr().add(i)` is safe because `i < len`
    // - `slice.as_mut_ptr().add(i - mid)` is safe because:
    //   - `i >= mid` (loop invariant), so `i - mid >= 0` (no underflow)
    //   - `i - mid < len - mid` (since `i < len`), so target is within bounds
    // - `ptr::copy` handles overlapping memory regions correctly
    for i in mid..len {
        unsafe {
            let src = slice.as_ptr().add(i);
            let dst = slice.as_mut_ptr().add(i - mid);
            core::ptr::copy(src, dst, 1);
        }
    }

    // Put saved elements at the end
    // SAFETY: All elements in temp are initialized, and we write to valid indices.
    for (i, item) in temp.into_iter().enumerate() {
        slice[len - mid + i] = unsafe { item.assume_init() };
    }
}

/// Rotates a slice right by `mid` positions.
pub fn rotate_right<T>(slice: &mut [T], mid: usize) {
    if mid == 0 || mid >= slice.len() {
        return;
    }
    rotate_left(slice, slice.len() - mid);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort() {
        let mut data = vec![5, 2, 8, 1, 9];
        sort(&mut data);
        assert_eq!(data, vec![1, 2, 5, 8, 9]);
    }

    #[test]
    fn test_sort_empty() {
        let mut data: Vec<i32> = vec![];
        sort(&mut data);
        assert!(data.is_empty());
    }

    #[test]
    fn test_sort_single() {
        let mut data = vec![42];
        sort(&mut data);
        assert_eq!(data, vec![42]);
    }

    #[test]
    fn test_is_sorted() {
        assert!(is_sorted(&[1, 2, 3, 4, 5]));
        assert!(!is_sorted(&[1, 3, 2, 4, 5]));
    }

    #[test]
    fn test_min_max() {
        let data = vec![5, 2, 8, 1, 9];
        let (min, max) = min_max(&data).unwrap();
        assert_eq!(*min, 1);
        assert_eq!(*max, 9);
    }

    #[test]
    fn test_select() {
        let mut data = vec![5, 2, 8, 1, 9];
        let third = select(&mut data, 2).unwrap();
        assert_eq!(*third, 5);
    }

    #[test]
    fn test_reverse() {
        let mut data = vec![1, 2, 3, 4, 5];
        reverse(&mut data);
        assert_eq!(data, vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_rotate_left() {
        let mut data = vec![1, 2, 3, 4, 5];
        rotate_left(&mut data, 2);
        assert_eq!(data, vec![3, 4, 5, 1, 2]);
    }

    #[test]
    fn test_rotate_right() {
        let mut data = vec![1, 2, 3, 4, 5];
        rotate_right(&mut data, 2);
        assert_eq!(data, vec![4, 5, 1, 2, 3]);
    }
}
