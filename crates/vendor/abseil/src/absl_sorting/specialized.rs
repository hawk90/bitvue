//! Specialized sorting algorithms for small arrays.

/// Optimized sort for small arrays (uses insertion sort).
pub fn small_sort<T: Ord>(slice: &mut [T]) {
    const SMALL_SORT_THRESHOLD: usize = 16;

    if slice.len() <= SMALL_SORT_THRESHOLD {
        insertion_sort(slice);
    } else {
        // Fall back to standard sort for larger arrays
        super::mergesort(slice);
    }
}

/// Insertion sort (stable, efficient for small arrays).
pub fn insertion_sort<T: Ord>(slice: &mut [T]) {
    for i in 1..slice.len() {
        let mut j = i;
        while j > 0 && slice[j - 1] > slice[j] {
            slice.swap(j - 1, j);
            j -= 1;
        }
    }
}

/// Bubble sort (simple but inefficient).
pub fn bubble_sort<T: Ord>(slice: &mut [T]) {
    let n = slice.len();
    for i in 0..n {
        let mut swapped = false;
        for j in 0..n.saturating_sub(i + 1) {
            if j + 1 < n && slice[j] > slice[j + 1] {
                slice.swap(j, j + 1);
                swapped = true;
            }
        }
        if !swapped {
            break;
        }
    }
}

/// Selection sort (simple but inefficient).
pub fn selection_sort<T: Ord>(slice: &mut [T]) {
    let n = slice.len();
    for i in 0..n.saturating_sub(1) {
        let mut min_idx = i;
        for j in (i + 1)..n {
            if slice[j] < slice[min_idx] {
                min_idx = j;
            }
        }
        if min_idx != i {
            slice.swap(i, min_idx);
        }
    }
}

/// Shell sort (generalization of insertion sort).
pub fn shell_sort<T: Ord>(slice: &mut [T]) {
    let n = slice.len();
    let mut gap = n / 2;

    while gap > 0 {
        for i in gap..n {
            let mut j = i;
            while j >= gap && slice[j - gap] > slice[j] {
                slice.swap(j - gap, j);
                j -= gap;
            }
        }
        gap /= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insertion_sort() {
        let mut data = vec![5, 2, 8, 1, 9];
        insertion_sort(&mut data);
        assert_eq!(data, vec![1, 2, 5, 8, 9]);
    }

    #[test]
    fn test_bubble_sort() {
        let mut data = vec![5, 2, 8, 1, 9];
        bubble_sort(&mut data);
        assert_eq!(data, vec![1, 2, 5, 8, 9]);
    }

    #[test]
    fn test_selection_sort() {
        let mut data = vec![5, 2, 8, 1, 9];
        selection_sort(&mut data);
        assert_eq!(data, vec![1, 2, 5, 8, 9]);
    }

    #[test]
    fn test_shell_sort() {
        let mut data = vec![5, 2, 8, 1, 9, 3];
        shell_sort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 5, 8, 9]);
    }

    #[test]
    fn test_small_sort() {
        let mut data = vec![5, 2, 8, 1, 9];
        small_sort(&mut data);
        assert_eq!(data, vec![1, 2, 5, 8, 9]);
    }

    #[test]
    fn test_insertion_sort_empty() {
        let mut data: Vec<i32> = vec![];
        insertion_sort(&mut data);
        assert!(data.is_empty());
    }
}
