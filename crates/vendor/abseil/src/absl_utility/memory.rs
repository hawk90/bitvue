//! Memory utilities.

use core::fmt;

/// Error type for memory operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MemoryError {
    /// Null pointer provided where valid pointer was required.
    NullPointer,
    /// Overflow in byte count calculation.
    Overflow,
    /// Invalid alignment.
    InvalidAlignment,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::NullPointer => write!(f, "null pointer provided"),
            MemoryError::Overflow => write!(f, "byte count calculation overflow"),
            MemoryError::InvalidAlignment => write!(f, "invalid pointer alignment"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MemoryError {}

/// Gets the address of a reference as a `usize`.
///
/// This is similar to Abseil's `get_address()` function.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::address_of;
///
/// let value = 42;
/// let addr = address_of(&value);
/// assert!(addr > 0);
/// ```
#[inline]
pub fn address_of<T>(r: &T) -> usize {
    r as *const T as usize
}

/// Gets the address of a mutable reference as a `usize`.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::address_of_mut;
///
/// let mut value = 42;
/// let addr = address_of_mut(&mut value);
/// assert!(addr > 0);
/// ```
#[inline]
pub fn address_of_mut<T>(r: &mut T) -> usize {
    r as *mut T as usize
}

/// Creates a null pointer of type `T`.
///
/// This is useful for pointer arithmetic and offset calculations.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::null_ptr;
///
/// let ptr: *const i32 = null_ptr::<i32>();
/// assert!(ptr.is_null());
/// ```
#[inline]
pub const fn null_ptr<T>() -> *const T {
    core::ptr::null()
}

/// Creates a null mutable pointer of type `T`.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::null_ptr_mut;
///
/// let ptr: *mut i32 = null_ptr_mut::<i32>();
/// assert!(ptr.is_null());
/// ```
#[inline]
pub const fn null_ptr_mut<T>() -> *mut T {
    core::ptr::null_mut()
}

/// Performs a volatile read from a pointer.
///
/// # Safety
///
/// The pointer must be properly aligned and point to initialized memory.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::read_volatile;
///
/// let value = 42i32;
/// let ptr = &value as *const i32;
/// unsafe {
///     assert_eq!(read_volatile(ptr), 42);
/// }
/// ```
#[inline]
pub unsafe fn read_volatile<T>(ptr: *const T) -> T {
    core::ptr::read_volatile(ptr)
}

/// Performs a volatile write to a pointer.
///
/// # Safety
///
/// The pointer must be properly aligned and valid for writes.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::write_volatile;
///
/// let mut value = 0i32;
/// unsafe {
///     write_volatile(&mut value as *mut i32, 42);
///     assert_eq!(value, 42);
/// }
/// ```
#[inline]
pub unsafe fn write_volatile<T>(ptr: *mut T, value: T) {
    core::ptr::write_volatile(ptr, value);
}

/// Performs a volatile read from a pointer with null pointer validation.
///
/// Returns `Err(MemoryError::NullPointer)` if the pointer is null.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::try_read_volatile;
///
/// let value = 42i32;
/// let ptr = &value as *const i32;
/// assert_eq!(try_read_volatile(ptr), Ok(42));
///
/// let null: *const i32 = core::ptr::null();
/// assert!(try_read_volatile(null).is_err());
/// ```
#[inline]
pub fn try_read_volatile<T>(ptr: *const T) -> Result<T, MemoryError> {
    if ptr.is_null() {
        return Err(MemoryError::NullPointer);
    }
    unsafe { Ok(core::ptr::read_volatile(ptr)) }
}

/// Performs a volatile write to a pointer with null pointer validation.
///
/// Returns `Err(MemoryError::NullPointer)` if the pointer is null.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::try_write_volatile;
///
/// let mut value = 0i32;
/// assert!(try_write_volatile(&mut value as *mut i32, 42).is_ok());
/// assert_eq!(value, 42);
///
/// let null: *mut i32 = core::ptr::null_mut();
/// assert!(try_write_volatile(null, 42).is_err());
/// ```
#[inline]
pub fn try_write_volatile<T>(ptr: *mut T, value: T) -> Result<(), MemoryError> {
    if ptr.is_null() {
        return Err(MemoryError::NullPointer);
    }
    unsafe {
        core::ptr::write_volatile(ptr, value);
    }
    Ok(())
}

/// Copies `count * size_of::<T>()` bytes from `src` to `dst`.
///
/// The source and destination must not overlap.
///
/// # Safety
///
/// Both pointers must be properly aligned and valid for the given count.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::memcpy;
///
/// let src = [1i32, 2, 3, 4];
/// let mut dst = [0i32; 4];
/// unsafe {
///     memcpy(src.as_ptr(), dst.as_mut_ptr(), 4);
///     assert_eq!(dst, src);
/// }
/// ```
#[inline]
pub unsafe fn memcpy<T>(src: *const T, dst: *mut T, count: usize) {
    let byte_count = count * core::mem::size_of::<T>();
    core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, byte_count);
}

/// Safe version of memcpy with validation.
///
/// Returns `Err(MemoryError::NullPointer)` if either pointer is null.
/// Returns `Err(MemoryError::Overflow)` if byte count calculation overflows.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::try_memcpy;
///
/// let src = [1i32, 2, 3, 4];
/// let mut dst = [0i32; 4];
/// assert!(try_memcpy(src.as_ptr(), dst.as_mut_ptr(), 4).is_ok());
/// assert_eq!(dst, src);
/// ```
#[inline]
pub fn try_memcpy<T>(src: *const T, dst: *mut T, count: usize) -> Result<(), MemoryError> {
    if src.is_null() || dst.is_null() {
        return Err(MemoryError::NullPointer);
    }
    let byte_count = count.checked_mul(core::mem::size_of::<T>())
        .ok_or(MemoryError::Overflow)?;
    unsafe {
        core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, byte_count);
    }
    Ok(())
}

/// Copies `count * size_of::<T>()` bytes from `src` to `dst`.
///
/// The source and destination may overlap.
///
/// # Safety
///
/// Both pointers must be properly aligned and valid for the given count.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::memmove;
///
/// let mut data = [1i32, 2, 3, 4, 5];
/// unsafe {
///     memmove(data.as_ptr().add(1), data.as_mut_ptr(), 4);
/// }
/// // Overlapping copy
/// ```
#[inline]
pub unsafe fn memmove<T>(src: *const T, dst: *mut T, count: usize) {
    let byte_count = count * core::mem::size_of::<T>();
    core::ptr::copy(src as *const u8, dst as *mut u8, byte_count);
}

/// Safe version of memmove with validation.
///
/// Returns `Err(MemoryError::NullPointer)` if either pointer is null.
/// Returns `Err(MemoryError::Overflow)` if byte count calculation overflows.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::try_memmove;
///
/// let mut data = [1i32, 2, 3, 4, 5];
/// assert!(try_memmove(data.as_ptr().add(1), data.as_mut_ptr(), 4).is_ok());
/// ```
#[inline]
pub fn try_memmove<T>(src: *const T, dst: *mut T, count: usize) -> Result<(), MemoryError> {
    if src.is_null() || dst.is_null() {
        return Err(MemoryError::NullPointer);
    }
    let byte_count = count.checked_mul(core::mem::size_of::<T>())
        .ok_or(MemoryError::Overflow)?;
    unsafe {
        core::ptr::copy(src as *const u8, dst as *mut u8, byte_count);
    }
    Ok(())
}

/// Sets `count * size_of::<T>()` bytes to zero starting at `dst`.
///
/// # Safety
///
/// The pointer must be properly aligned and valid for the given count.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::memset_zero;
///
/// let mut data = [1i32, 2, 3, 4];
/// unsafe {
///     memset_zero(data.as_mut_ptr(), 4);
///     assert_eq!(data, [0, 0, 0, 0]);
/// }
/// ```
#[inline]
pub unsafe fn memset_zero<T>(dst: *mut T, count: usize) {
    let byte_count = count * core::mem::size_of::<T>();
    core::ptr::write_bytes(dst as *mut u8, 0, byte_count);
}

/// Safe version of memset_zero with validation.
///
/// Returns `Err(MemoryError::NullPointer)` if the pointer is null.
/// Returns `Err(MemoryError::Overflow)` if byte count calculation overflows.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::try_memset_zero;
///
/// let mut data = [1i32, 2, 3, 4];
/// assert!(try_memset_zero(data.as_mut_ptr(), 4).is_ok());
/// assert_eq!(data, [0, 0, 0, 0]);
/// ```
#[inline]
pub fn try_memset_zero<T>(dst: *mut T, count: usize) -> Result<(), MemoryError> {
    if dst.is_null() {
        return Err(MemoryError::NullPointer);
    }
    let byte_count = count.checked_mul(core::mem::size_of::<T>())
        .ok_or(MemoryError::Overflow)?;
    unsafe {
        core::ptr::write_bytes(dst as *mut u8, 0, byte_count);
    }
    Ok(())
}

/// Gets the size of a value in bytes.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::size_of_val;
///
/// let value = 42i32;
/// assert_eq!(size_of_val(&value), 4);
/// ```
#[inline]
pub fn size_of_val<T: ?Sized>(val: &T) -> usize {
    core::mem::size_of_val(val)
}

/// Gets the alignment of a value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::memory::align_of_val;
///
/// let value = 42i32;
/// assert!(align_of_val(&value) >= 4);
/// ```
#[inline]
pub fn align_of_val<T: ?Sized>(val: &T) -> usize {
    core::mem::align_of_val(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_of() {
        let value = 42;
        let addr = address_of(&value);
        assert!(addr > 0);
    }

    #[test]
    fn test_address_of_mut() {
        let mut value = 42;
        let addr = address_of_mut(&mut value);
        assert!(addr > 0);
    }

    #[test]
    fn test_null_ptr() {
        let ptr: *const i32 = null_ptr();
        assert!(ptr.is_null());
    }

    #[test]
    fn test_null_ptr_mut() {
        let ptr: *mut i32 = null_ptr_mut();
        assert!(ptr.is_null());
    }

    #[test]
    fn test_try_read_volatile_valid_pointer() {
        let value = 42i32;
        let ptr = &value as *const i32;
        assert_eq!(try_read_volatile(ptr), Ok(42));
    }

    #[test]
    fn test_try_read_volatile_null_pointer() {
        let null: *const i32 = core::ptr::null();
        assert_eq!(try_read_volatile(null), Err(MemoryError::NullPointer));
    }

    #[test]
    fn test_try_write_volatile_valid_pointer() {
        let mut value = 0i32;
        assert!(try_write_volatile(&mut value as *mut i32, 42).is_ok());
        assert_eq!(value, 42);
    }

    #[test]
    fn test_try_write_volatile_null_pointer() {
        let null: *mut i32 = core::ptr::null_mut();
        assert_eq!(try_write_volatile(null, 42), Err(MemoryError::NullPointer));
    }

    #[test]
    fn test_try_memcpy_valid_pointers() {
        let src = [1i32, 2, 3, 4];
        let mut dst = [0i32; 4];
        assert!(try_memcpy(src.as_ptr(), dst.as_mut_ptr(), 4).is_ok());
        assert_eq!(dst, src);
    }

    #[test]
    fn test_try_memcpy_null_src() {
        let mut dst = [0i32; 4];
        let null: *const i32 = core::ptr::null();
        assert_eq!(try_memcpy(null, dst.as_mut_ptr(), 4), Err(MemoryError::NullPointer));
    }

    #[test]
    fn test_try_memcpy_null_dst() {
        let src = [1i32, 2, 3, 4];
        let null: *mut i32 = core::ptr::null_mut();
        assert_eq!(try_memcpy(src.as_ptr(), null, 4), Err(MemoryError::NullPointer));
    }

    #[test]
    fn test_try_memcpy_overflow() {
        let src = [1i32; 1];
        let mut dst = [0i32; 1];
        // Use a count that would overflow when multiplied by size_of::<i32>()
        let huge_count = usize::MAX / 2;
        assert!(try_memcpy(src.as_ptr(), dst.as_mut_ptr(), huge_count).is_err());
    }

    #[test]
    fn test_try_memmove_valid_pointers() {
        let mut data = [1i32, 2, 3, 4, 5];
        assert!(try_memmove(data.as_ptr().add(1), data.as_mut_ptr(), 4).is_ok());
    }

    #[test]
    fn test_try_memmove_null_pointers() {
        let null: *const i32 = core::ptr::null();
        let mut dst = [0i32; 4];
        assert_eq!(try_memmove(null, dst.as_mut_ptr(), 4), Err(MemoryError::NullPointer));
    }

    #[test]
    fn test_try_memset_zero_valid_pointer() {
        let mut data = [1i32, 2, 3, 4];
        assert!(try_memset_zero(data.as_mut_ptr(), 4).is_ok());
        assert_eq!(data, [0, 0, 0, 0]);
    }

    #[test]
    fn test_try_memset_zero_null_pointer() {
        let null: *mut i32 = core::ptr::null_mut();
        assert_eq!(try_memset_zero(null, 4), Err(MemoryError::NullPointer));
    }

    #[test]
    fn test_try_memset_zero_overflow() {
        let mut data = [1i32; 1];
        let huge_count = usize::MAX / 2;
        assert!(try_memset_zero(data.as_mut_ptr(), huge_count).is_err());
    }

    #[test]
    fn test_memory_error_display() {
        assert_eq!(format!("{}", MemoryError::NullPointer), "null pointer provided");
        assert_eq!(format!("{}", MemoryError::Overflow), "byte count calculation overflow");
        assert_eq!(format!("{}", MemoryError::InvalidAlignment), "invalid pointer alignment");
    }

    #[test]
    fn test_try_read_volatile_different_types() {
        let value = 3.14f32;
        let ptr = &value as *const f32;
        assert_eq!(try_read_volatile(ptr), Ok(3.14f32));
    }

    #[test]
    fn test_try_write_volatile_different_types() {
        let mut value = 0u64;
        assert!(try_write_volatile(&mut value as *mut u64, 42).is_ok());
        assert_eq!(value, 42);
    }
}
