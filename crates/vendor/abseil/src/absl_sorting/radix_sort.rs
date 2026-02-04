//! Radix sort implementation for integers.

extern crate alloc;

/// LSD radix sort for unsigned integers.
pub fn radix_sort(slice: &mut [u32]) {
    if slice.len() <= 1 {
        return;
    }

    // SAFETY: Prevent overflow in counting arrays by validating slice size
    // The maximum value in count array is slice.len(), and prefix sum could
    // reach slice.len() * 256 in worst case. We ensure this won't overflow usize.
    // This check prevents DoS from extremely large (but still theoretically valid) slices.
    const MAX_SAFE_LEN: usize = usize::MAX / 256;
    if slice.len() > MAX_SAFE_LEN {
        panic!(
            "radix_sort: slice too large ({} elements), maximum is {} to prevent overflow",
            slice.len(),
            MAX_SAFE_LEN
        );
    }

    // Sort by each digit (byte) from least to most significant
    for shift in (0..32).step_by(8) {
        counting_sort_by_byte(slice, shift);
    }
}

/// Radix sort for u8.
pub fn radix_sort_u8(slice: &mut [u8]) {
    if slice.len() <= 1 {
        return;
    }

    // SAFETY: Prevent overflow in counting arrays (same logic as radix_sort)
    const MAX_SAFE_LEN: usize = usize::MAX / 256;
    if slice.len() > MAX_SAFE_LEN {
        panic!(
            "radix_sort_u8: slice too large ({} elements), maximum is {} to prevent overflow",
            slice.len(),
            MAX_SAFE_LEN
        );
    }

    counting_sort_u8(slice);
}

/// Radix sort for u16.
pub fn radix_sort_u16(slice: &mut [u16]) {
    if slice.len() <= 1 {
        return;
    }

    // SAFETY: Prevent overflow in counting arrays (same logic as radix_sort)
    const MAX_SAFE_LEN: usize = usize::MAX / 256;
    if slice.len() > MAX_SAFE_LEN {
        panic!(
            "radix_sort_u16: slice too large ({} elements), maximum is {} to prevent overflow",
            slice.len(),
            MAX_SAFE_LEN
        );
    }

    for shift in (0..16).step_by(8) {
        counting_sort_by_byte_u16(slice, shift);
    }
}

/// Radix sort for u32.
pub fn radix_sort_u32(slice: &mut [u32]) {
    radix_sort(slice)
}

fn counting_sort_by_byte(slice: &mut [u32], shift: u32) {
    const COUNT_SIZE: usize = 256;
    let mut count = [0usize; COUNT_SIZE];
    let mut output = vec![0u32; slice.len()];

    // Count occurrences
    for &val in slice.iter() {
        let byte = ((val >> shift) & 0xFF) as usize;
        count[byte] += 1;
    }

    // Convert to prefix sums
    for i in 1..COUNT_SIZE {
        count[i] += count[i - 1];
    }

    // Build output array (stable)
    for &val in slice.iter().rev() {
        let byte = ((val >> shift) & 0xFF) as usize;
        count[byte] -= 1;
        output[count[byte]] = val;
    }

    // Copy back
    slice.copy_from_slice(&output);
}

fn counting_sort_by_byte_u16(slice: &mut [u16], shift: u32) {
    const COUNT_SIZE: usize = 256;
    let mut count = [0usize; COUNT_SIZE];
    let mut output = vec![0u16; slice.len()];

    for &val in slice.iter() {
        let byte = ((val >> shift) & 0xFF) as usize;
        count[byte] += 1;
    }

    for i in 1..COUNT_SIZE {
        count[i] += count[i - 1];
    }

    for &val in slice.iter().rev() {
        let byte = ((val >> shift) & 0xFF) as usize;
        count[byte] -= 1;
        output[count[byte]] = val;
    }

    slice.copy_from_slice(&output);
}

fn counting_sort_u8(slice: &mut [u8]) {
    const COUNT_SIZE: usize = 256;
    let mut count = [0usize; COUNT_SIZE];
    let mut output = vec![0u8; slice.len()];

    // SAFETY: Prevent overflow - same check as other radix sort functions
    // This is called from radix_sort_u8 which already validates size
    for &val in slice.iter() {
        count[val as usize] += 1;
    }

    for i in 1..COUNT_SIZE {
        count[i] += count[i - 1];
    }

    for &val in slice.iter().rev() {
        let idx = val as usize;
        count[idx] -= 1;
        output[count[idx]] = val;
    }

    slice.copy_from_slice(&output);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radix_sort() {
        let mut data = vec![5u32, 2, 8, 1, 9, 3, 0, 255, 1000];
        radix_sort(&mut data);
        assert_eq!(data, vec![0, 1, 2, 3, 5, 8, 9, 255, 1000]);
    }

    #[test]
    fn test_radix_sort_u8() {
        let mut data = vec![5u8, 2, 8, 1, 9, 3, 255, 0];
        radix_sort_u8(&mut data);
        assert_eq!(data, vec![0, 1, 2, 3, 5, 8, 9, 255]);
    }

    #[test]
    fn test_radix_sort_u16() {
        let mut data = vec![5u16, 2, 8, 1, 9, 3, 65535, 0];
        radix_sort_u16(&mut data);
        assert_eq!(data, vec![0, 1, 2, 3, 5, 8, 9, 65535]);
    }

    #[test]
    fn test_radix_sort_u32() {
        let mut data = vec![100u32, 50, 200, 25];
        radix_sort_u32(&mut data);
        assert_eq!(data, vec![25, 50, 100, 200]);
    }
}
