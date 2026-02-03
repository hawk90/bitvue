//! Memory operations module - basic memory manipulation functions.

use core::ptr;

/// Trait for comparing memory regions.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::MemEq;
///
/// let a = &[1, 2, 3, 4];
/// let b = &[1, 2, 3, 4];
/// assert!(a.mem_eq(b));
/// ```
pub trait MemEq {
    /// Returns true if two memory regions are equal.
    fn mem_eq(&self, other: &Self) -> bool;
}

impl<T: PartialEq> MemEq for [T] {
    fn mem_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl MemEq for [u8] {
    fn mem_eq(&self, other: &Self) -> bool {
        self == other
    }
}

/// Compares two memory regions byte by byte.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::memcmp;
///
/// let a = b"hello";
/// let b = b"hello";
/// assert!(memcmp(a, b));
/// ```
pub fn memcmp(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

/// Compares two memory regions and returns the ordering.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::memcmp_ord;
///
/// let a = b"abc";
/// let b = b"abd";
/// assert!(memcmp_ord(a, b) < 0);
/// ```
pub fn memcmp_ord(a: &[u8], b: &[u8]) -> i32 {
    let min_len = a.len().min(b.len());
    for i in 0..min_len {
        match a[i].cmp(&b[i]) {
            core::cmp::Ordering::Equal => {}
            other => return other as i32,
        }
    }
    a.len().cmp(&b.len()) as i32
}

/// Fills a memory region with a byte value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::memset;
///
/// let mut buffer = [0u8; 16];
/// memset(&mut buffer, 0xFF);
/// assert!(buffer.iter().all(|&b| b == 0xFF));
/// ```
pub fn memset(data: &mut [u8], value: u8) {
    data.fill(value);
}

/// Copies memory from source to destination.
///
/// # Panics
///
/// Panics if the source and destination overlap.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::memcpy;
///
/// let src = [1, 2, 3, 4];
/// let mut dst = [0u8; 4];
/// memcpy(&src, &mut dst);
/// assert_eq!(dst, src);
/// ```
pub fn memcpy<T: Copy>(src: &[T], dst: &mut [T]) {
    assert!(dst.len() >= src.len(), "destination too small");
    dst[..src.len()].copy_from_slice(src);
}

/// Moves memory from source to destination.
///
/// Handles overlapping regions correctly.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::memmove;
///
/// let mut buffer = [1, 2, 3, 4, 5];
/// memmove(&buffer[0..3], &mut buffer[2..5]);
/// ```
pub fn memmove<T: Copy>(src: &[T], dst: &mut [T]) {
    assert!(dst.len() >= src.len(), "destination too small");
    dst[..src.len()].copy_from_slice(src);
}

/// Zeroes a memory region.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::bzero;
///
/// let mut buffer = [1u8, 2, 3, 4];
/// bzero(&mut buffer);
/// assert!(buffer.iter().all(|&b| b == 0));
/// ```
pub fn bzero(data: &mut [u8]) {
    data.fill(0);
}

/// Returns the length of a null-terminated string.
///
/// # Safety
///
/// The pointer must point to a valid null-terminated string.
/// The function will panic if the string exceeds `isize::MAX` bytes
/// to prevent pointer arithmetic overflow.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::strlen;
///
/// let s = b"hello\0world";
/// unsafe {
///     assert_eq!(strlen(s.as_ptr()), 5);
/// }
/// ```
pub unsafe fn strlen(mut ptr: *const u8) -> usize {
    let mut len = 0;
    // Safety limit to prevent infinite loops and pointer overflow
    // This also ensures we don't exceed the maximum offset for ptr.add
    const MAX_STRLEN: usize = isize::MAX as usize;
    while *ptr != 0 {
        len += 1;
        if len > MAX_STRLEN {
            panic!("strlen: string exceeds maximum length (missing null terminator?)");
        }
        ptr = ptr.add(1);
    }
    len
}

/// Searches for a byte value in a memory region.
///
/// Returns a pointer to the first occurrence, or null if not found.
///
/// # Safety
///
/// The pointer must be valid for reading `len` bytes.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::memchr;
///
/// let data = b"hello world";
/// let ptr = unsafe { memchr(data.as_ptr(), data.len(), b'o') };
/// assert!(!ptr.is_null());
/// ```
pub unsafe fn memchr(ptr: *const u8, len: usize, value: u8) -> *const u8 {
    // Use offset_from style iteration to avoid pointer arithmetic overflow
    // Iterate by incrementing pointer directly, which is safer than ptr.add(i)
    let mut current = ptr;
    for _ in 0..len {
        if *current == value {
            return current;
        }
        // Increment pointer by 1 element - this won't overflow like ptr.add(i) could
        current = current.offset(1);
    }
    ptr::null()
}

/// Searches for the last occurrence of a byte value.
///
/// # Safety
///
/// The pointer must be valid for reading `len` bytes.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::memrchr;
///
/// let data = b"hello world";
/// let ptr = unsafe { memrchr(data.as_ptr(), data.len(), b'l') };
/// assert!(!ptr.is_null());
/// ```
pub unsafe fn memrchr(ptr: *const u8, len: usize, value: u8) -> *const u8 {
    // Use offset from end to avoid pointer arithmetic overflow
    // Calculate the end pointer once, then iterate backwards
    if len == 0 {
        return ptr::null();
    }
    // Start from the last valid byte (len - 1 offset from ptr)
    let mut current = ptr.offset(len as isize - 1);
    loop {
        if *current == value {
            return current;
        }
        // Check if we've reached the start
        if current == ptr {
            break;
        }
        current = current.offset(-1);
    }
    ptr::null()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memcmp() {
        let a = b"hello";
        let b = b"hello";
        assert!(memcmp(a, b));
    }

    #[test]
    fn test_memcmp_ord() {
        let a = b"abc";
        let b = b"abd";
        assert!(memcmp_ord(a, b) < 0);
    }

    #[test]
    fn test_memset() {
        let mut buffer = [0u8; 16];
        memset(&mut buffer, 0xFF);
        assert!(buffer.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_memcpy() {
        let src = [1, 2, 3, 4];
        let mut dst = [0u8; 4];
        memcpy(&src, &mut dst);
        assert_eq!(dst, src);
    }

    #[test]
    fn test_bzero() {
        let mut buffer = [1u8, 2, 3, 4];
        bzero(&mut buffer);
        assert!(buffer.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_strlen() {
        let s = b"hello\0world";
        unsafe {
            assert_eq!(strlen(s.as_ptr()), 5);
        }
    }

    #[test]
    fn test_memchr() {
        let data = b"hello world";
        let ptr = unsafe { memchr(data.as_ptr(), data.len(), b'o') };
        assert!(!ptr.is_null());
    }

    #[test]
    fn test_memrchr() {
        let data = b"hello world";
        let ptr = unsafe { memrchr(data.as_ptr(), data.len(), b'l') };
        assert!(!ptr.is_null());
    }
}
