//! Pointer utilities module - alignment and pointer manipulation.

/// Checks if a pointer is aligned to the given alignment.
///
/// # Panics
///
/// Panics if alignment is not a power of two or is zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::is_aligned_ptr;
///
/// let aligned = 0x1000 as *const u8;
/// assert!(is_aligned_ptr(aligned, 16));
///
/// let unaligned = 0x1001 as *const u8;
/// assert!(!is_aligned_ptr(unaligned, 16));
/// ```
pub fn is_aligned_ptr<T>(ptr: *const T, alignment: usize) -> bool {
    // SAFETY: Validate alignment is power of two and not zero
    if alignment == 0 || !alignment.is_power_of_two() {
        panic!(
            "is_aligned_ptr: alignment must be a non-zero power of two, got {}",
            alignment
        );
    }
    (ptr as usize) & (alignment - 1) == 0
}

/// Aligns a pointer up to the given alignment.
///
/// # Safety
///
/// The resulting pointer must be within the same allocation and valid.
///
/// # Panics
///
/// Panics if alignment is not a power of two or is zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::align_ptr_up;
///
/// let ptr = 0x1001 as *const u8;
/// let aligned = unsafe { align_ptr_up(ptr, 16) };
/// assert_eq!(aligned as usize, 0x1010);
/// ```
pub unsafe fn align_ptr_up<T>(ptr: *const T, alignment: usize) -> *const T {
    // SAFETY: Validate alignment is power of two and not zero
    // to prevent underflow in (alignment - 1) and incorrect masking
    if alignment == 0 || !alignment.is_power_of_two() {
        panic!(
            "align_ptr_up: alignment must be a non-zero power of two, got {}",
            alignment
        );
    }
    let addr = ptr as usize;
    let aligned = (addr + alignment - 1) & !(alignment - 1);
    aligned as *const T
}

/// Aligns a pointer down to the given alignment.
///
/// # Safety
///
/// The resulting pointer must be valid.
///
/// # Panics
///
/// Panics if alignment is not a power of two or is zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::align_ptr_down;
///
/// let ptr = 0x101F as *const u8;
/// let aligned = unsafe { align_ptr_down(ptr, 16) };
/// assert_eq!(aligned as usize, 0x1010);
/// ```
pub unsafe fn align_ptr_down<T>(ptr: *const T, alignment: usize) -> *const T {
    // SAFETY: Validate alignment is power of two and not zero
    // to prevent underflow in (alignment - 1) and incorrect masking
    if alignment == 0 || !alignment.is_power_of_two() {
        panic!(
            "align_ptr_down: alignment must be a non-zero power of two, got {}",
            alignment
        );
    }
    let addr = ptr as usize;
    let aligned = addr & !(alignment - 1);
    aligned as *const T
}

/// Returns the alignment offset needed to align a pointer.
///
/// # Panics
///
/// Panics if alignment is not a power of two or is zero.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::align_offset_for_ptr;
///
/// let ptr = 0x1001 as *const u8;
/// assert_eq!(align_offset_for_ptr(ptr, 16), 15);
/// ```
pub fn align_offset_for_ptr<T>(ptr: *const T, alignment: usize) -> usize {
    // SAFETY: Validate alignment is power of two and not zero
    if alignment == 0 || !alignment.is_power_of_two() {
        panic!(
            "align_offset_for_ptr: alignment must be a non-zero power of two, got {}",
            alignment
        );
    }
    let addr = ptr as usize;
    let aligned_up = (addr + alignment - 1) & !(alignment - 1);
    aligned_up - addr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_aligned_ptr() {
        let aligned = 0x1000 as *const u8;
        assert!(is_aligned_ptr(aligned, 16));

        let unaligned = 0x1001 as *const u8;
        assert!(!is_aligned_ptr(unaligned, 16));
    }

    #[test]
    fn test_align_ptr_up() {
        let ptr = 0x1001 as *const u8;
        let aligned = unsafe { align_ptr_up(ptr, 16) };
        assert_eq!(aligned as usize, 0x1010);
    }

    #[test]
    fn test_align_ptr_down() {
        let ptr = 0x101F as *const u8;
        let aligned = unsafe { align_ptr_down(ptr, 16) };
        assert_eq!(aligned as usize, 0x1010);
    }

    #[test]
    fn test_align_offset_for_ptr() {
        let ptr = 0x1001 as *const u8;
        assert_eq!(align_offset_for_ptr(ptr, 16), 15);
    }
}
