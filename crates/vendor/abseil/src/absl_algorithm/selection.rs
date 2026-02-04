//! Selection utilities - nth_element, partition_pivot (helper)

/// Finds the nth element that would be in sorted position.
///
/// This is a partial sorting operation - after this call, the element at
/// index `n` will be in its sorted position, with all smaller elements
/// before it and all larger elements after it (not necessarily sorted).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_algorithm::nth_element;
///
/// let mut data = [5, 2, 8, 1, 9];
/// nth_element(&mut data, 2);
/// // data[2] is now 5, the third smallest element
/// assert!(data[2] == 5 || data[2] == 5); // In sorted position
/// ```
#[inline]
pub fn nth_element<T: PartialOrd>(slice: &mut [T], n: usize) {
    if slice.len() <= 1 {
        return;
    }

    let len = slice.len();
    if n >= len {
        return;
    }

    // Simple quickselect implementation
    let mut left = 0;
    let mut right = len;

    while right > left + 1 {
        let pivot_idx = partition_pivot(slice, left, right);

        if n < pivot_idx {
            right = pivot_idx;
        } else if n > pivot_idx {
            left = pivot_idx + 1;
        } else {
            break;
        }
    }
}

/// Helper for nth_element - partitions around a pivot.
#[inline]
pub(crate) fn partition_pivot<T: PartialOrd>(slice: &mut [T], left: usize, right: usize) -> usize {
    let pivot_idx = left + (right - left) / 2;
    let pivot_value = unsafe { &*(slice.as_ptr().add(pivot_idx) as *const T) };

    let mut i = left;
    let mut j = right - 1;

    slice.swap(pivot_idx, right - 1);

    loop {
        while i < j && unsafe { &*(slice.as_ptr().add(i) as *const T) < pivot_value } {
            i += 1;
        }

        j = match (i + 1..right)
            .rev()
            .find(|&k| unsafe { &*(slice.as_ptr().add(k) as *const T) >= pivot_value })
        {
            Some(k) => k,
            None => break,
        };

        if i >= j {
            break;
        }

        slice.swap(i, j);
    }

    slice.swap(i, right - 1);
    i
}
