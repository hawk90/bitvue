// Vendored code from Abseil C++ library - suppress Clippy warnings
#![allow(clippy::needless_range_loop)]

//! Additional sorting algorithms.
//!
//! This module provides additional sorting algorithms including:
//! - Selection sort
//! - Shell sort
//! - Cycle sort
//! - Comb sort
//! - Gnome sort
//! - Cocktail sort (shaker sort)

extern crate alloc;

/// Selection sort - O(n²) simple sorting algorithm.
///
/// Finds the minimum element and places it at the beginning.
pub fn selection_sort<T: Ord>(slice: &mut [T]) {
    let len = slice.len();
    for i in 0..len.saturating_sub(1) {
        let mut min_idx = i;
        for j in (i + 1)..len {
            if slice[j] < slice[min_idx] {
                min_idx = j;
            }
        }
        if min_idx != i {
            slice.swap(i, min_idx);
        }
    }
}

/// Selection sort with custom comparison function.
pub fn selection_sort_by<T, F>(slice: &mut [T], mut compare: F)
where
    F: FnMut(&T, &T) -> core::cmp::Ordering,
{
    let len = slice.len();
    for i in 0..len.saturating_sub(1) {
        let mut min_idx = i;
        for j in (i + 1)..len {
            if compare(&slice[j], &slice[min_idx]) == core::cmp::Ordering::Less {
                min_idx = j;
            }
        }
        if min_idx != i {
            slice.swap(i, min_idx);
        }
    }
}

/// Shell sort - Generalization of insertion sort.
///
/// Uses gap sequence to sort elements far apart first.
pub fn shell_sort<T: Ord>(slice: &mut [T]) {
    let len = slice.len();
    if len <= 1 {
        return;
    }

    // Ciura gap sequence (known to be good)
    let gaps = [701, 301, 132, 57, 23, 10, 4, 1];

    for &gap in gaps.iter() {
        if gap > len {
            continue;
        }
        for i in gap..len {
            // Shift elements within the slice
            let mut j = i;
            while j >= gap && slice[j] < slice[j - gap] {
                slice.swap(j, j - gap);
                j -= gap;
            }
        }
    }
}

/// Cycle sort - Minimizes the number of writes to memory.
///
/// Good for situations where write operations are expensive.
pub fn cycle_sort<T: Ord + Clone>(slice: &mut [T]) -> usize {
    let len = slice.len();
    if len <= 1 {
        return 0;
    }

    let mut writes = 0;

    for cycle_start in 0..len - 1 {
        let item = slice[cycle_start].clone();

        // Find where to put the item
        let mut pos = cycle_start;
        for i in (cycle_start + 1)..len {
            if slice[i] < item {
                pos += 1;
            }
        }

        // If item is already in correct position
        if pos == cycle_start {
            continue;
        }

        // Skip duplicates
        while pos < len && slice[pos] == item {
            pos += 1;
        }

        // Put item to its right position
        if pos != cycle_start && pos < len {
            slice.swap(pos, cycle_start);
            writes += 1;
        }

        // Rotate the rest of the cycle (only if not already positioned)
        let mut current_pos = pos;
        while current_pos != cycle_start {
            // Find position for the item at cycle_start
            let mut pos = cycle_start;
            for i in (cycle_start + 1)..len {
                if slice[i] < slice[cycle_start] {
                    pos += 1;
                }
            }

            // Skip duplicates
            while pos < len && slice[pos] == slice[cycle_start] {
                pos += 1;
            }

            // Swap if needed
            if pos != cycle_start && pos < len && pos != current_pos {
                slice.swap(pos, cycle_start);
                writes += 1;
                current_pos = pos;
            } else {
                // Break to avoid infinite loop
                break;
            }
        }
    }

    writes
}

/// Comb sort - Improvement over bubble sort.
///
/// Uses a shrinking gap factor to eliminate turtles.
pub fn comb_sort<T: Ord>(slice: &mut [T]) {
    let len = slice.len();
    if len <= 1 {
        return;
    }

    let mut gap = len;
    let shrink = 1.3;
    let mut sorted = false;

    while !sorted {
        gap = (gap as f64 / shrink).floor() as usize;
        if gap <= 1 {
            gap = 1;
            sorted = true;
        }

        for i in 0..len.saturating_sub(gap) {
            if slice[i] > slice[i + gap] {
                slice.swap(i, i + gap);
                sorted = false;
            }
        }
    }
}

/// Gnome sort - Similar to insertion sort but moving elements like a garden gnome.
///
/// Simple and stable, O(n²) time complexity.
pub fn gnome_sort<T: Ord>(slice: &mut [T]) {
    let len = slice.len();
    if len <= 1 {
        return;
    }

    let mut i = 1;
    while i < len {
        if slice[i] >= slice[i - 1] {
            i += 1;
        } else {
            slice.swap(i, i - 1);
            if i > 1 {
                i -= 1;
            }
        }
    }
}

/// Cocktail sort (shaker sort) - Bidirectional bubble sort.
///
/// Sorts in both directions, improving on bubble sort.
pub fn cocktail_sort<T: Ord>(slice: &mut [T]) {
    let len = slice.len();
    if len <= 1 {
        return;
    }

    let mut start = 0;
    let mut end = len - 1;
    let mut swapped = true;

    while swapped {
        swapped = false;

        // Forward pass (left to right)
        for i in start..end {
            if slice[i] > slice[i + 1] {
                slice.swap(i, i + 1);
                swapped = true;
            }
        }

        if !swapped {
            break;
        }

        swapped = false;
        end -= 1;

        // Backward pass (right to left)
        for i in (start..=end).rev() {
            if i > 0 && slice[i] < slice[i - 1] {
                slice.swap(i, i - 1);
                swapped = true;
            }
        }

        start += 1;
    }
}

/// Odd-even sort - Parallel sorting algorithm.
///
/// Compares odd-even indexed pairs in alternating phases.
pub fn odd_even_sort<T: Ord>(slice: &mut [T]) {
    let len = slice.len();
    if len <= 1 {
        return;
    }

    let mut sorted = false;
    while !sorted {
        sorted = true;

        // Odd phase
        for i in (1..len.saturating_sub(1)).step_by(2) {
            if slice[i] > slice[i + 1] {
                slice.swap(i, i + 1);
                sorted = false;
            }
        }

        // Even phase
        for i in (0..len.saturating_sub(1)).step_by(2) {
            if slice[i] > slice[i + 1] {
                slice.swap(i, i + 1);
                sorted = false;
            }
        }
    }
}

/// Stooge sort - Recursive sorting algorithm (mostly for educational purposes).
///
/// Has O(n^2.7) time complexity, very inefficient.
pub fn stooge_sort<T: Ord>(slice: &mut [T]) {
    fn stooge<T: Ord>(slice: &mut [T], start: usize, end: usize) {
        // SAFETY: Validate range to prevent integer underflow
        if start > end {
            panic!("stooge_sort: invalid range start={} > end={}", start, end);
        }
        if end >= slice.len() {
            panic!("stooge_sort: end={} >= slice.len()={}", end, slice.len());
        }

        if slice[start] > slice[end] {
            slice.swap(start, end);
        }

        // SAFETY: start <= end is validated above, so end - start won't underflow
        // However, we add 1, so we need to ensure end - start + 1 doesn't overflow
        // This is safe because usize::MAX + 1 would overflow, but if end = usize::MAX
        // and start = 0, then end >= slice.len() would have already been caught
        if end - start + 1 > 2 {
            let t = (end - start + 1) / 3;
            // SAFETY: end - t won't underflow because t = (end - start + 1) / 3,
            // so t < (end - start + 1) which means end - t >= start
            stooge(slice, start, end - t);
            // SAFETY: start + t <= end because t < (end - start + 1)
            stooge(slice, start + t, end);
            stooge(slice, start, end - t);
        }
    }

    if slice.len() > 1 {
        stooge(slice, 0, slice.len() - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_sort() {
        let mut data = vec![64, 34, 25, 12, 22, 11, 90];
        selection_sort(&mut data);
        assert_eq!(data, vec![11, 12, 22, 25, 34, 64, 90]);
    }

    #[test]
    fn test_shell_sort() {
        let mut data = vec![64, 34, 25, 12, 22, 11, 90];
        shell_sort(&mut data);
        assert_eq!(data, vec![11, 12, 22, 25, 34, 64, 90]);
    }

    #[test]
    fn test_cycle_sort() {
        let mut data = vec![64, 34, 25, 12, 22, 11, 90];
        let writes = cycle_sort(&mut data);
        assert_eq!(data, vec![11, 12, 22, 25, 34, 64, 90]);
        assert!(writes > 0);
    }

    #[test]
    fn test_comb_sort() {
        let mut data = vec![64, 34, 25, 12, 22, 11, 90];
        comb_sort(&mut data);
        assert_eq!(data, vec![11, 12, 22, 25, 34, 64, 90]);
    }

    #[test]
    fn test_gnome_sort() {
        let mut data = vec![64, 34, 25, 12, 22, 11, 90];
        gnome_sort(&mut data);
        assert_eq!(data, vec![11, 12, 22, 25, 34, 64, 90]);
    }

    #[test]
    fn test_cocktail_sort() {
        let mut data = vec![64, 34, 25, 12, 22, 11, 90];
        cocktail_sort(&mut data);
        assert_eq!(data, vec![11, 12, 22, 25, 34, 64, 90]);
    }

    #[test]
    fn test_odd_even_sort() {
        let mut data = vec![64, 34, 25, 12, 22, 11, 90];
        odd_even_sort(&mut data);
        assert_eq!(data, vec![11, 12, 22, 25, 34, 64, 90]);
    }

    #[test]
    fn test_stooge_sort() {
        let mut data = vec![64, 34, 25, 12, 22, 11];
        stooge_sort(&mut data);
        assert_eq!(data, vec![11, 12, 22, 25, 34, 64]);
    }
}
