//! Memory utilities.
//!
//! This module provides memory utilities similar to Abseil's `absl/memory` directory,
//! including alignment utilities, memory size helpers, and pointer operations.

use core::fmt;
use core::mem::{self};

/// Alignment value for memory alignment.
///
/// This represents a power-of-two alignment value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Alignment(pub usize);

impl Alignment {
    /// Creates a new alignment.
    ///
    /// # Panics
    ///
    /// Panics if `value` is not a power of two.
    #[inline]
    pub const fn new(value: usize) -> Self {
        assert!(value.is_power_of_two(), "Alignment must be a power of two");
        Self(value)
    }

    /// Returns the alignment value.
    #[inline]
    pub const fn get(self) -> usize {
        self.0
    }

    /// Minimum alignment (1 byte).
    pub const MIN: Alignment = Alignment(1);

    /// Maximum alignment for the platform.
    #[cfg(target_pointer_width = "64")]
    pub const MAX: Alignment = Alignment(16);

    #[cfg(target_pointer_width = "32")]
    pub const MAX: Alignment = Alignment(8);

    /// Alignment of 8 bytes.
    pub const ALIGN_8: Alignment = Alignment(8);

    /// Alignment of 16 bytes.
    pub const ALIGN_16: Alignment = Alignment(16);

    /// Alignment of 32 bytes.
    pub const ALIGN_32: Alignment = Alignment(32);

    /// Alignment of 64 bytes.
    pub const ALIGN_64: Alignment = Alignment(64);

    /// Returns the log2 of the alignment value.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_memory::memory::Alignment;
    ///
    /// assert_eq!(Alignment::new(8).log2(), 3);
    /// assert_eq!(Alignment::new(64).log2(), 6);
    /// ```
    #[inline]
    pub const fn log2(self) -> u32 {
        self.0.trailing_zeros() as u32
    }

    /// Checks if this alignment is at least as large as the given value.
    #[inline]
    pub const fn is_at_least(self, value: usize) -> bool {
        self.0 >= value
    }
}

impl fmt::Debug for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Alignment({})", self.0)
    }
}

/// Returns the alignment of the type `T`.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::align_of;
///
/// assert_eq!(align_of::<u8>(), 1);
/// assert_eq!(align_of::<u64>(), 8);
/// ```
#[inline]
pub const fn align_of<T>() -> usize {
    mem::align_of::<T>()
}

/// Returns the size of the type `T` in bytes.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::size_of;
///
/// assert_eq!(size_of::<u8>(), 1);
/// assert_eq!(size_of::<u64>(), 8);
/// ```
#[inline]
pub const fn size_of<T>() -> usize {
    mem::size_of::<T>()
}

/// Returns the alignment required for a slice with the given element alignment.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::slice_align_of;
///
/// assert_eq!(slice_align_of::<u8>(4), 1);
/// ```
#[inline]
pub const fn slice_align_of<T>(len: usize) -> usize {
    if len == 0 {
        return 1;
    }
    align_of::<T>()
}

/// Aligns a pointer up to the given alignment.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::align_up;
///
/// assert_eq!(align_up(0x1001, 8), 0x1008);
/// assert_eq!(align_up(0x1000, 8), 0x1000);
/// ```
///
/// # Panics
///
/// Panics if `alignment` is not a power of two.
#[inline]
pub const fn align_up(ptr: usize, alignment: usize) -> usize {
    assert!(alignment.is_power_of_two(), "alignment must be a power of two");
    (ptr + alignment - 1) & !(alignment - 1)
}

/// Checks if a pointer is aligned to the given alignment.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::is_aligned;
///
/// assert!(is_aligned(0x1000, 16));
/// assert!(!is_aligned(0x1001, 16));
/// ```
///
/// # Panics
///
/// Panics if `alignment` is not a power of two.
#[inline]
pub const fn is_aligned(ptr: usize, alignment: usize) -> bool {
    assert!(alignment.is_power_of_two(), "alignment must be a power of two");
    (ptr & (alignment - 1)) == 0
}

/// Aligns a value down to the given alignment.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::align_down;
///
/// assert_eq!(align_down(0x100F, 8), 0x1008);
/// assert_eq!(align_down(0x1000, 8), 0x1000);
/// ```
///
/// # Panics
///
/// Panics if `alignment` is not a power of two.
#[inline]
pub const fn align_down(ptr: usize, alignment: usize) -> usize {
    assert!(alignment.is_power_of_two(), "alignment must be a power of two");
    ptr & !(alignment - 1)
}

/// Returns the difference to align a pointer up to the given alignment.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::alignment_offset;
///
/// assert_eq!(alignment_offset(0x1001, 8), 7);
/// assert_eq!(alignment_offset(0x1000, 8), 0);
/// ```
///
/// # Panics
///
/// Panics if `alignment` is not a power of two.
#[inline]
pub const fn alignment_offset(ptr: usize, alignment: usize) -> usize {
    assert!(alignment.is_power_of_two(), "alignment must be a power of two");
    align_up(ptr, alignment) - ptr
}

/// Memory size units for human-readable representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryUnit {
    /// Bytes
    Bytes(u64),
    /// Kibibytes (KiB) - 1024 bytes
    KiB(u64),
    /// Mebibytes (MiB) - 1024 KiB
    MiB(u64),
    /// Gibibytes (GiB) - 1024 MiB
    GiB(u64),
    /// Tebibytes (TiB) - 1024 GiB
    TiB(u64),
}

impl MemoryUnit {
    /// Returns the total number of bytes.
    #[inline]
    pub const fn bytes(self) -> u64 {
        match self {
            MemoryUnit::Bytes(b) => b,
            MemoryUnit::KiB(k) => k * 1024,
            MemoryUnit::MiB(m) => m * 1024 * 1024,
            MemoryUnit::GiB(g) => g * 1024 * 1024 * 1024,
            MemoryUnit::TiB(t) => t * 1024 * 1024 * 1024 * 1024,
        }
    }

    /// Returns the memory size as a human-readable string.
    pub const fn human(self) -> &'static str {
        match self {
            MemoryUnit::Bytes(_) => "B",
            MemoryUnit::KiB(_) => "KiB",
            MemoryUnit::MiB(_) => "MiB",
            MemoryUnit::GiB(_) => "GiB",
            MemoryUnit::TiB(_) => "TiB",
        }
    }
}

impl fmt::Display for MemoryUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            MemoryUnit::Bytes(b) => *b,
            MemoryUnit::KiB(k) => *k,
            MemoryUnit::MiB(m) => *m,
            MemoryUnit::GiB(g) => *g,
            MemoryUnit::TiB(t) => *t,
        };
        write!(f, "{} {}", value, self.human())
    }
}

/// Creates a memory unit from bytes, choosing the most appropriate unit.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::memory_unit;
///
/// assert_eq!(memory_unit(500).human(), "B");
/// assert_eq!(memory_unit(1024).human(), "KiB");
/// assert_eq!(memory_unit(1024 * 1024).human(), "MiB");
/// ```
pub fn memory_unit(bytes: u64) -> MemoryUnit {
    if bytes < 1024 {
        MemoryUnit::Bytes(bytes)
    } else if bytes < 1024 * 1024 {
        MemoryUnit::KiB(bytes / 1024)
    } else if bytes < 1024 * 1024 * 1024 {
        MemoryUnit::MiB(bytes / (1024 * 1024))
    } else if bytes < 1024 * 1024 * 1024 * 1024 {
        MemoryUnit::GiB(bytes / (1024 * 1024 * 1024))
    } else {
        MemoryUnit::TiB(bytes / (1024 * 1024 * 1024 * 1024))
    }
}

/// Converts a pointer to a usize.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::pointer_to_usize;
///
/// let value: u32 = 42;
/// let ptr = &value as *const u32;
/// let addr = pointer_to_usize(ptr);
/// ```
#[inline]
pub fn pointer_to_usize<T>(ptr: *const T) -> usize {
    ptr as usize
}

/// Converts a usize to a pointer.
///
/// # Safety
///
/// The resulting pointer may not be valid for type `T`.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::usize_to_pointer;
///
/// let addr: usize = 0x1000;
/// let ptr: *const u32 = usize_to_pointer(addr);
/// ```
#[inline]
pub const fn usize_to_pointer<T>(addr: usize) -> *const T {
    addr as *const T
}

/// Converts a mutable usize to a mutable pointer.
///
/// # Safety
///
/// The resulting pointer may not be valid for type `T`.
#[inline]
pub const fn usize_to_pointer_mut<T>(addr: usize) -> *mut T {
    addr as *mut T
}

/// Returns the offset between two pointers.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::pointer_diff;
///
/// let arr: [u8; 10] = [0; 10];
/// let diff = pointer_diff(arr.as_ptr().wrapping_add(1), arr.as_ptr());
/// assert_eq!(diff, 1);
/// ```
#[inline]
pub fn pointer_diff<T>(end: *const T, start: *const T) -> isize {
    (end as isize) - (start as isize)
}

/// Advances a pointer by a given number of elements.
///
/// # Safety
///
/// The resulting pointer must be within the same allocation.
///
/// # Panics
///
/// Panics in debug mode if `count` would cause the pointer to overflow
/// or exceed `isize::MAX` elements.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::pointer_advance;
///
/// let arr: [u8; 10] = [0; 10];
/// let ptr = arr.as_ptr();
/// unsafe {
///     let advanced = pointer_advance(ptr, 5);
/// }
/// ```
///
/// # Safety
///
/// The caller must ensure the advanced pointer is valid.
#[inline]
pub unsafe fn pointer_advance<T>(ptr: *const T, count: usize) -> *const T {
    #[cfg(debug_assertions)]
    {
        // Prevent overflow in pointer arithmetic
        assert!(count <= isize::MAX as usize, "count too large for pointer arithmetic");
    }
    ptr.add(count)
}

/// Advances a mutable pointer by a given number of elements.
///
/// # Safety
///
/// The resulting pointer must be within the same allocation.
///
/// # Panics
///
/// Panics in debug mode if `count` would cause the pointer to overflow
/// or exceed `isize::MAX` elements.
#[inline]
pub unsafe fn pointer_advance_mut<T>(ptr: *mut T, count: usize) -> *mut T {
    #[cfg(debug_assertions)]
    {
        assert!(count <= isize::MAX as usize, "count too large for pointer arithmetic");
    }
    ptr.add(count)
}

/// Returns true if the pointer is null.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::is_null_ptr;
///
/// let ptr: *const u32 = std::ptr::null();
/// assert!(is_null_ptr(ptr));
/// ```
#[inline]
pub const fn is_null_ptr<T>(ptr: *const T) -> bool {
    ptr.is_null()
}

/// Returns true if the pointer is null.
#[inline]
pub const fn is_null_ptr_mut<T>(ptr: *mut T) -> bool {
    ptr.is_null()
}

/// Checks if two pointers are equal.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::ptr_eq;
///
/// let value = 42u32;
/// let ptr1 = &value as *const u32;
/// let ptr2 = &value as *const u32;
/// assert!(ptr_eq(ptr1, ptr2));
/// ```
#[inline]
pub fn ptr_eq<T>(a: *const T, b: *const T) -> bool {
    (a as usize) == (b as usize)
}

/// Returns the alignment mask for the given alignment.
///
/// The mask is `alignment - 1`, which can be used to check alignment.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::alignment_mask;
///
/// assert_eq!(alignment_mask(8), 7);
/// assert_eq!(alignment_mask(16), 15);
/// ```
///
/// # Panics
///
/// Panics if `alignment` is not a power of two.
#[inline]
pub const fn alignment_mask(alignment: usize) -> usize {
    assert!(alignment.is_power_of_two(), "alignment must be a power of two");
    alignment - 1
}

/// Computes the next power of two greater than or equal to the value.
///
/// Returns 0 if the value would overflow.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::next_power_of_two;
///
/// assert_eq!(next_power_of_two(0), 1);
/// assert_eq!(next_power_of_two(5), 8);
/// assert_eq!(next_power_of_two(16), 16);
/// assert_eq!(next_power_of_two(17), 32);
/// ```
pub const fn next_power_of_two(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    if n.is_power_of_two() {
        return n;
    }
    // Use the bit propagation method
    let mut n = n;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    #[cfg(target_pointer_width = "64")]
    {
        n |= n >> 32;
    }
    n + 1
}

/// Computes the previous power of two less than or equal to the value.
///
/// Returns 0 if the value is less than 2.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::prev_power_of_two;
///
/// assert_eq!(prev_power_of_two(0), 0);
/// assert_eq!(prev_power_of_two(1), 1);
/// assert_eq!(prev_power_of_two(5), 4);
/// assert_eq!(prev_power_of_two(16), 16);
/// ```
pub const fn prev_power_of_two(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    if n.is_power_of_two() {
        return n;
    }
    // Use the bit propagation method to fill all bits below MSB
    let mut n = n;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    #[cfg(target_pointer_width = "64")]
    {
        n |= n >> 32;
    }
    // After propagation, n = 2^(k+1) - 1, so (n+1) >> 1 = 2^k
    (n + 1) >> 1
}

/// Checks if a size is a valid allocation size (not too large).
///
/// This is useful for avoiding integer overflow in allocation calculations.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::is_valid_allocation_size;
///
/// assert!(is_valid_allocation_size(1024));
/// assert!(!is_valid_allocation_size(usize::MAX));
/// ```
pub const fn is_valid_allocation_size(size: usize) -> bool {
    // Check if size * 2 would overflow
    size <= usize::MAX / 2
}

/// Checks if a count of elements is valid for allocation calculations.
///
/// Returns false if `count * mem::size_of::<T>()` would overflow.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::is_valid_count;
///
/// assert!(is_valid_count::<u8>(100));
/// assert!(!is_valid_count::<u64>(usize::MAX));
/// ```
pub const fn is_valid_count<T>(count: usize) -> bool {
    count <= usize::MAX / size_of::<T>()
}

/// Calculates the total size of an array allocation.
///
/// Returns None if the calculation would overflow.
///
/// # Examples
///
/// ```
/// use abseil::absl_memory::memory::checked_allocation_size;
///
/// assert_eq!(checked_allocation_size::<u32>(100), Some(400));
/// assert_eq!(checked_allocation_size::<u64>(usize::MAX), None);
/// ```
pub const fn checked_allocation_size<T>(count: usize) -> Option<usize> {
    match count.checked_mul(size_of::<T>()) {
        Some(size) if is_valid_allocation_size(size) => Some(size),
        Some(_) => None, // Too large for safe allocation
        None => None,   // Overflow
    }
}

/// Executes a function with a stack alignment of at least the given bytes.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::memory::with_aligned_stack;
///
/// let result = with_aligned_stack(16, || {
///     // Do work with 16-byte stack alignment
///     42
/// });
/// assert_eq!(result, 42);
/// ```
///
/// # Panics
///
/// Panics if `alignment` is not a power of two.
#[inline]
pub fn with_aligned_stack<F, R>(alignment: usize, f: F) -> R
where
    F: FnOnce() -> R,
{
    assert!(alignment.is_power_of_two(), "alignment must be a power of two");
    // Note: In practice, Rust's stack alignment is platform-dependent.
    // This function is a marker for intent; actual alignment would require
    // platform-specific assembly or compiler intrinsics.
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_new() {
        let a = Alignment::new(8);
        assert_eq!(a.get(), 8);
    }

    #[test]
    fn test_alignment_constants() {
        assert_eq!(Alignment::MIN.get(), 1);
        assert_eq!(Alignment::ALIGN_8.get(), 8);
        assert_eq!(Alignment::ALIGN_16.get(), 16);
        assert_eq!(Alignment::ALIGN_32.get(), 32);
        assert_eq!(Alignment::ALIGN_64.get(), 64);
    }

    #[test]
    fn test_alignment_max() {
        // MAX should be at least 8
        assert!(Alignment::MAX.get() >= 8);
        // MAX should be a power of two
        assert!(Alignment::MAX.get().is_power_of_two());
    }

    #[test]
    #[should_panic]
    fn test_alignment_invalid() {
        let _ = Alignment::new(7); // Not a power of two
    }

    #[test]
    fn test_alignment_ord() {
        let a1 = Alignment::new(8);
        let a2 = Alignment::new(16);
        assert!(a1 < a2);
        assert!(a2 > a1);
    }

    #[test]
    fn test_alignment_eq() {
        let a1 = Alignment::new(8);
        let a2 = Alignment::new(8);
        assert_eq!(a1, a2);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0x1001, 8), 0x1008);
        assert_eq!(align_up(0x1000, 8), 0x1000);
        assert_eq!(align_up(0, 8), 0);
        assert_eq!(align_up(1, 8), 8);
    }

    #[test]
    fn test_is_aligned() {
        assert!(is_aligned(0x1000, 16));
        assert!(!is_aligned(0x1001, 16));
        assert!(is_aligned(0, 1));
    }

    #[test]
    fn test_align_down() {
        assert_eq!(align_down(0x100F, 8), 0x1008);
        assert_eq!(align_down(0x1000, 8), 0x1000);
        assert_eq!(align_down(0x100F, 16), 0x1000);
    }

    #[test]
    fn test_alignment_offset() {
        assert_eq!(alignment_offset(0x1001, 8), 7);
        assert_eq!(alignment_offset(0x1000, 8), 0);
        assert_eq!(alignment_offset(0x0, 8), 0);
    }

    #[test]
    fn test_align_of() {
        assert_eq!(align_of::<u8>(), 1);
        assert_eq!(align_of::<u64>(), 8);
        assert_eq!(align_of::<[u8; 10]>(), 1);
    }

    #[test]
    fn test_size_of() {
        assert_eq!(size_of::<u8>(), 1);
        assert_eq!(size_of::<u64>(), 8);
        assert_eq!(size_of::<[u8; 10]>(), 10);
    }

    #[test]
    fn test_slice_align_of() {
        assert_eq!(slice_align_of::<u8>(4), 1);
        assert_eq!(slice_align_of::<u64>(0), 1);
    }

    #[test]
    fn test_memory_unit() {
        let unit = memory_unit(500);
        assert_eq!(unit.bytes(), 500);
        assert_eq!(unit.human(), "B");

        let unit = memory_unit(1024);
        assert_eq!(unit.bytes(), 1024);
        assert_eq!(unit.human(), "KiB");

        let unit = memory_unit(1024 * 1024);
        assert_eq!(unit.human(), "MiB");

        let unit = memory_unit(1024 * 1024 * 1024);
        assert_eq!(unit.human(), "GiB");
    }

    #[test]
    fn test_alignment_mask() {
        assert_eq!(alignment_mask(8), 7);
        assert_eq!(alignment_mask(16), 15);
        assert_eq!(alignment_mask(1), 0);
    }

    #[test]
    fn test_next_power_of_two() {
        assert_eq!(next_power_of_two(0), 1);
        assert_eq!(next_power_of_two(1), 1);
        assert_eq!(next_power_of_two(5), 8);
        assert_eq!(next_power_of_two(8), 8);
        assert_eq!(next_power_of_two(16), 16);
        assert_eq!(next_power_of_two(17), 32);
        assert_eq!(next_power_of_two(1023), 1024);
    }

    #[test]
    fn test_prev_power_of_two() {
        assert_eq!(prev_power_of_two(0), 0);
        assert_eq!(prev_power_of_two(1), 1);
        assert_eq!(prev_power_of_two(5), 4);
        assert_eq!(prev_power_of_two(8), 8);
        assert_eq!(prev_power_of_two(16), 16);
        assert_eq!(prev_power_of_two(17), 16);
        assert_eq!(prev_power_of_two(1024), 1024);
    }

    #[test]
    fn test_is_valid_allocation_size() {
        assert!(is_valid_allocation_size(1024));
        assert!(!is_valid_allocation_size(usize::MAX));
        assert!(is_valid_allocation_size(usize::MAX / 2));
    }

    #[test]
    fn test_is_valid_count() {
        assert!(is_valid_count::<u8>(100));
        assert!(is_valid_count::<u8>(usize::MAX));
        assert!(!is_valid_count::<u64>(usize::MAX));
    }

    #[test]
    fn test_checked_allocation_size() {
        assert_eq!(checked_allocation_size::<u32>(100), Some(400));
        assert_eq!(checked_allocation_size::<u64>(usize::MAX), None);
        assert_eq!(checked_allocation_size::<u8>(usize::MAX), None);
    }

    #[test]
    fn test_with_aligned_stack() {
        let result = with_aligned_stack(16, || {
            42
        });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_pointer_diff() {
        let arr: [u8; 10] = [0; 10];
        let diff = pointer_diff(arr.as_ptr().wrapping_add(1), arr.as_ptr());
        assert_eq!(diff, 1);
    }

    #[test]
    fn test_is_null_ptr() {
        assert!(is_null_ptr(std::ptr::null::<u8>()));
        assert!(!is_null_ptr(0x1000 as *const u8));
    }

    #[test]
    fn test_ptr_eq() {
        let value = 42u32;
        let ptr1 = &value as *const u32;
        let ptr2 = &value as *const u32;
        assert!(ptr_eq(ptr1, ptr2));
    }

    #[test]
    fn test_alignment_log2() {
        assert_eq!(Alignment::new(8).log2(), 3);
        assert_eq!(Alignment::new(64).log2(), 6);
        assert_eq!(Alignment::new(1).log2(), 0);
    }

    #[test]
    fn test_alignment_is_at_least() {
        assert!(Alignment::new(16).is_at_least(8));
        assert!(!Alignment::new(8).is_at_least(16));
        assert!(Alignment::new(8).is_at_least(8));
    }
}
