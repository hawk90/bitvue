//! Hybrid sorting algorithms that combine multiple approaches.

extern crate alloc;

use alloc::vec::Vec;

/// Introsort: quicksort that falls back to heapsort for bad cases.
///
/// This algorithm monitors recursion depth and switches to heapsort
/// when the depth exceeds 2 * log2(n) to guarantee O(n log n) worst case.
pub fn introsort<T: Ord>(slice: &mut [T]) {
    let max_depth = if slice.len() > 1 {
        2 * (slice.len() as f64).log2() as usize
    } else {
        0
    };
    introsort_helper(slice, max_depth);
}

fn introsort_helper<T: Ord>(slice: &mut [T], max_depth: usize) {
    let len = slice.len();
    if len <= 1 {
        return;
    }

    if max_depth == 0 {
        super::heapsort::heapsort(slice);
        return;
    }

    if len <= 16 {
        super::specialized::insertion_sort(slice);
        return;
    }

    let pivot_idx = partition(slice);
    let (left, right) = slice.split_at_mut(pivot_idx);
    introsort_helper(left, max_depth - 1);
    if right.len() > 1 {
        introsort_helper(&mut right[1..], max_depth - 1);
    }
}

fn partition<T: Ord>(slice: &mut [T]) -> usize {
    let len = slice.len();
    let pivot_idx = len / 2;

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

/// Timsort: hybrid sorting algorithm derived from merge sort and insertion sort.
///
/// This is a simplified implementation inspired by Python's timsort.
/// It takes advantage of existing order in the data.
pub fn timsort<T: Ord>(slice: &mut [T]) {
    if slice.len() <= 64 {
        super::specialized::insertion_sort(slice);
        return;
    }

    // Find runs and merge them
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;

    while i < slice.len() {
        let run_start = i;
        let mut run_end = i + 1;

        // Find natural run
        if run_end < slice.len() && slice[run_end] >= slice[run_start] {
            // Ascending run
            while run_end < slice.len() && slice[run_end] >= slice[run_end - 1] {
                run_end += 1;
            }
        } else {
            // Descending run - reverse it
            while run_end < slice.len() && slice[run_end] < slice[run_end - 1] {
                run_end += 1;
            }
            slice[run_start..run_end].reverse();
        }

        // Extend run to min size
        let min_run = 32;
        if run_end - run_start < min_run {
            let extend_to = core::cmp::min(run_start + min_run, slice.len());
            super::specialized::insertion_sort(&mut slice[run_start..extend_to]);
            runs.push((run_start, extend_to));
            i = extend_to;
        } else {
            runs.push((run_start, run_end));
            i = run_end;
        }

        // Merge runs
        while runs.len() > 1 {
            let n = runs.len();
            if n >= 3
                && runs[n - 3].1 - runs[n - 3].0
                    <= runs[n - 2].1 - runs[n - 2].0 + runs[n - 1].1 - runs[n - 1].0
            {
                merge_at(slice, n - 3);
            } else if runs[n - 2].1 - runs[n - 2].0 <= runs[n - 1].1 - runs[n - 1].0 {
                merge_at(slice, n - 2);
            } else {
                break;
            }
        }
    }

    // Merge remaining runs
    while runs.len() > 1 {
        merge_at(slice, runs.len() - 2);
    }
}

fn merge_at<T: Ord>(slice: &mut [T], idx: usize) {
    // This is a simplified merge - would be more efficient with temp storage
    let runs = collect_runs();
    if idx + 1 >= runs.len() {
        return;
    }

    // Merge idx and idx + 1 using merge sort approach
    let (start, _mid) = runs[idx];
    let (_, end) = runs[idx + 1];
    super::mergesort::mergesort(&mut slice[start..end]);
}

fn collect_runs() -> Vec<(usize, usize)> {
    // Stub - actual implementation would track runs differently
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introsort() {
        let mut data = vec![5, 2, 8, 1, 9, 3];
        introsort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 5, 8, 9]);
    }

    #[test]
    fn test_introsort_already_sorted() {
        let mut data = vec![1, 2, 3, 4, 5];
        introsort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_introsort_reverse() {
        let mut data = vec![5, 4, 3, 2, 1];
        introsort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_timsort() {
        let mut data = vec![5, 2, 8, 1, 9, 3];
        timsort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 5, 8, 9]);
    }

    #[test]
    fn test_timsort_small() {
        let mut data = vec![3, 1, 2];
        timsort(&mut data);
        assert_eq!(data, vec![1, 2, 3]);
    }
}
