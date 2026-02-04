//! Heap sort implementation.

/// Heap sort (unstable, in-place, O(n log n)).
pub fn heapsort<T: Ord>(slice: &mut [T]) {
    if slice.len() <= 1 {
        return;
    }

    // Build max heap
    heapify(slice);

    // Extract elements from heap
    for end in (1..slice.len()).rev() {
        slice.swap(0, end);
        sift_down(slice, 0, end);
    }
}

fn heapify<T: Ord>(slice: &mut [T]) {
    let n = slice.len();
    for start in (0..n / 2).rev() {
        sift_down(slice, start, n);
    }
}

fn sift_down<T: Ord>(slice: &mut [T], start: usize, end: usize) {
    let mut root = start;

    while let Some(child) = left_child(root, end) {
        let mut max_child = child;

        let right = child + 1;
        if right < end && slice[right] > slice[child] {
            max_child = right;
        }

        if slice[root] >= slice[max_child] {
            break;
        }

        slice.swap(root, max_child);
        root = max_child;
    }
}

fn left_child(index: usize, end: usize) -> Option<usize> {
    let child = 2 * index + 1;
    if child < end {
        Some(child)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heapsort() {
        let mut data = vec![5, 2, 8, 1, 9, 3];
        heapsort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 5, 8, 9]);
    }

    #[test]
    fn test_heapsort_empty() {
        let mut data: Vec<i32> = vec![];
        heapsort(&mut data);
        assert!(data.is_empty());
    }

    #[test]
    fn test_heapsort_single() {
        let mut data = vec![42];
        heapsort(&mut data);
        assert_eq!(data, vec![42]);
    }

    #[test]
    fn test_heapsort_reverse() {
        let mut data = vec![5, 4, 3, 2, 1];
        heapsort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }
}
