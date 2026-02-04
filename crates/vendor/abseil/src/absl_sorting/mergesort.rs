//! Merge sort implementation.

extern crate alloc;

use alloc::vec::Vec;
use core::cmp::Ordering;
use core::mem::MaybeUninit;

/// Stable merge sort.
pub fn mergesort<T: Ord>(slice: &mut [T]) {
    if slice.len() <= 1 {
        return;
    }

    let mid = slice.len() / 2;
    mergesort(&mut slice[..mid]);
    mergesort(&mut slice[mid..]);

    // Merge in place using temporary buffer
    // SAFETY: We use MaybeUninit to safely handle types with destructors.
    // The values are moved from slice to temp, then moved back to slice.
    // Each value is moved exactly once, ensuring proper drop semantics.
    let mut temp: Vec<MaybeUninit<T>> = Vec::with_capacity(slice.len());
    let mut left = 0;
    let mut right = mid;

    while left < mid && right < slice.len() {
        if slice[left] <= slice[right] {
            // SAFETY: left is in bounds (0..mid)
            unsafe {
                temp.push(MaybeUninit::new(core::ptr::read(&slice[left])));
            }
            left += 1;
        } else {
            // SAFETY: right is in bounds (mid..slice.len())
            unsafe {
                temp.push(MaybeUninit::new(core::ptr::read(&slice[right])));
            }
            right += 1;
        }
    }

    while left < mid {
        // SAFETY: left is in bounds (0..mid)
        unsafe {
            temp.push(MaybeUninit::new(core::ptr::read(&slice[left])));
        }
        left += 1;
    }

    while right < slice.len() {
        // SAFETY: right is in bounds (mid..slice.len())
        unsafe {
            temp.push(MaybeUninit::new(core::ptr::read(&slice[right])));
        }
        right += 1;
    }

    // Copy back
    // SAFETY: All elements in temp are initialized, and we write to valid indices.
    for (i, item) in temp.into_iter().enumerate() {
        slice[i] = unsafe { item.assume_init() };
    }
}

/// Merge sort with custom comparison function.
pub fn mergesort_by<T, F>(slice: &mut [T], mut compare: F)
where
    F: FnMut(&T, &T) -> Ordering,
{
    // Use iterative merge sort to avoid recursive reference issues
    let n = slice.len();
    if n <= 1 {
        return;
    }

    let mut width = 1;
    while width < n {
        for i in (0..n).step_by(2 * width) {
            let left = i;
            let mid = core::cmp::min(i + width, n);
            let right = core::cmp::min(i + 2 * width, n);

            if mid < right {
                merge_by(slice, left, mid, right, &mut compare);
            }
        }
        width *= 2;
    }
}

/// Merge two sorted ranges using a comparison function.
fn merge_by<T, F>(slice: &mut [T], left: usize, mid: usize, right: usize, compare: &mut F)
where
    F: FnMut(&T, &T) -> Ordering,
{
    // SAFETY: We use MaybeUninit to safely handle types with destructors.
    // The values are moved from slice to temp, then moved back to slice.
    // Each value is moved exactly once, ensuring proper drop semantics.
    let mut temp: Vec<MaybeUninit<T>> = Vec::with_capacity(right - left);

    let mut i = left;
    let mut j = mid;

    while i < mid && j < right {
        if compare(&slice[i], &slice[j]) != Ordering::Greater {
            // SAFETY: i is in bounds (left..mid)
            unsafe {
                temp.push(MaybeUninit::new(core::ptr::read(&slice[i])));
            }
            i += 1;
        } else {
            // SAFETY: j is in bounds (mid..right)
            unsafe {
                temp.push(MaybeUninit::new(core::ptr::read(&slice[j])));
            }
            j += 1;
        }
    }

    while i < mid {
        // SAFETY: i is in bounds (left..mid)
        unsafe {
            temp.push(MaybeUninit::new(core::ptr::read(&slice[i])));
        }
        i += 1;
    }

    while j < right {
        // SAFETY: j is in bounds (mid..right)
        unsafe {
            temp.push(MaybeUninit::new(core::ptr::read(&slice[j])));
        }
        j += 1;
    }

    // SAFETY: All elements in temp are initialized, and we write to valid indices.
    for (k, item) in temp.into_iter().enumerate() {
        slice[left + k] = unsafe { item.assume_init() };
    }
}

/// Alias for mergesort (stable sort).
pub fn stable_sort<T: Ord>(slice: &mut [T]) {
    mergesort(slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mergesort() {
        let mut data = vec![5, 2, 8, 1, 9, 3];
        mergesort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 5, 8, 9]);
    }

    #[test]
    fn test_mergesort_by() {
        let mut data = vec![5, 2, 8, 1, 9];
        mergesort_by(&mut data, |a, b| b.cmp(a)); // reverse
        assert_eq!(data, vec![9, 8, 5, 2, 1]);
    }

    #[test]
    fn test_stable_sort() {
        let mut data = vec![5, 2, 8, 1, 9];
        stable_sort(&mut data);
        assert_eq!(data, vec![1, 2, 5, 8, 9]);
    }
}
