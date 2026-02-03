//! Tagged pointer module - pointer with embedded tag bit.

use core::marker::PhantomData;

/// A tagged pointer that stores additional bits of information in the pointer itself.
///
/// This requires that the alignment of T is at least 2, allowing the lowest bit
/// to be used for tagging.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_memory::TaggedPtr;
///
/// let value = Box::new(42u64);
/// let mut tagged = TaggedPtr::new(value, false).unwrap();
/// assert_eq!(unsafe { *tagged.as_ptr() }, 42);
/// assert!(!tagged.tag());
///
/// tagged.set_tag(true);
/// assert!(tagged.tag());
/// ```
#[repr(transparent)]
pub struct TaggedPtr<T> {
    ptr: usize,
    _phantom: PhantomData<T>,
}

impl<T> TaggedPtr<T> {
    /// Creates a new tagged pointer.
    ///
    /// Returns None if the alignment of T is less than 2.
    ///
    /// This is the safe way to create a TaggedPtr - it validates alignment
    /// before performing any unsafe operations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::TaggedPtr;
    ///
    /// let value = Box::new(42u64);
    /// let tagged = TaggedPtr::new(value, false).unwrap();
    /// ```
    pub fn new(ptr: Box<T>, tag: bool) -> Option<Self> {
        Self::try_new(ptr, tag)
    }

    /// Creates a new tagged pointer without alignment checking.
    ///
    /// # Safety
    ///
    /// The alignment of T must be at least 2 for the tagging to work correctly.
    /// If alignment is less than 2, this will cause undefined behavior.
    ///
    /// This is provided for performance-critical code where alignment is
    /// guaranteed by other means. Prefer using `new()` or `try_new()` instead.
    pub unsafe fn new_unchecked(ptr: Box<T>, tag: bool) -> Self {
        let addr = Box::into_raw(ptr) as usize;
        let tagged = if tag { addr | 1 } else { addr & !1 };
        Self {
            ptr: tagged,
            _phantom: PhantomData,
        }
    }

    /// Creates a new tagged pointer with alignment validation.
    ///
    /// Returns None if the alignment of T is less than 2.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_memory::TaggedPtr;
    ///
    /// let value = Box::new(42u64);
    /// let tagged = TaggedPtr::try_new(value, false).unwrap();
    /// ```
    pub fn try_new(ptr: Box<T>, tag: bool) -> Option<Self> {
        if core::mem::align_of::<T>() < 2 {
            return None;
        }
        // SAFETY: Alignment is validated, so new_unchecked is safe to call
        unsafe { Some(Self::new_unchecked(ptr, tag)) }
    }

    /// Recreates the box, consuming the tagged pointer.
    ///
    /// # Safety
    ///
    /// The tagged pointer must have been created from a valid Box.
    pub unsafe fn into_box(self) -> Box<T> {
        let addr = self.ptr & !1;
        Box::from_raw(addr as *mut T)
    }

    /// Returns a reference to the tagged value.
    ///
    /// # Safety
    ///
    /// The pointer must be valid and dereferenceable.
    pub unsafe fn as_ptr(&self) -> &T {
        // SAFETY: Clear the tag bit before dereferencing to prevent memory corruption
        // The lowest bit is used for tagging and must be removed to get the valid pointer
        let addr = self.ptr & !1;
        &*(addr as *const T)
    }

    /// Returns a mutable reference to the tagged value.
    ///
    /// # Safety
    ///
    /// The pointer must be valid and dereferenceable, and no other
    /// references to the same data must exist.
    pub unsafe fn as_mut_ptr(&mut self) -> &mut T {
        // SAFETY: Clear the tag bit before dereferencing to prevent memory corruption
        // The lowest bit is used for tagging and must be removed to get the valid pointer
        let addr = self.ptr & !1;
        &mut *(addr as *mut T)
    }

    /// Returns the tag bit.
    pub fn tag(&self) -> bool {
        self.ptr & 1 == 1
    }

    /// Sets the tag bit.
    pub fn set_tag(&mut self, tag: bool) {
        if tag {
            self.ptr |= 1;
        } else {
            self.ptr &= !1;
        }
    }

    /// Returns the raw address (without tag).
    pub fn addr(&self) -> usize {
        self.ptr & !1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Alignment >= 2 types

    #[test]
    fn test_tagged_ptr_with_u64() {
        // u64 has alignment >= 2, so tagging is safe
        let value = Box::new(42u64);
        let mut tagged = TaggedPtr::new(value, false).expect("u64 has alignment >= 2");
        assert_eq!(unsafe { *tagged.as_ptr() }, 42);
        assert!(!tagged.tag());

        tagged.set_tag(true);
        assert!(tagged.tag());
        assert_eq!(unsafe { *tagged.as_ptr() }, 42);

        // Clean up
        unsafe { tagged.into_box() };
    }

    #[test]
    fn test_tagged_ptr_set_tag_preserves_value() {
        let value = Box::new(123u64);
        let mut tagged = TaggedPtr::new(value, false).expect("u64 has alignment >= 2");

        tagged.set_tag(true);
        assert_eq!(unsafe { *tagged.as_ptr() }, 123);

        tagged.set_tag(false);
        assert_eq!(unsafe { *tagged.as_ptr() }, 123);

        unsafe { tagged.into_box() };
    }

    #[test]
    fn test_tagged_ptr_into_box_reconstructs() {
        let original = Box::new(456u64);
        let ptr_value = Box::into_raw(original) as usize;

        let value = Box::new(456u64);
        let tagged = TaggedPtr::new(value, true).expect("u64 has alignment >= 2");
        let reconstructed = unsafe { tagged.into_box() };

        assert_eq!(*reconstructed, 456);
        assert_eq!(Box::into_raw(reconstructed) as usize, ptr_value);
    }

    #[test]
    fn test_tagged_ptr_addr_returns_clean_address() {
        let value = Box::new(789u64);
        let tagged = TaggedPtr::new(value, true).expect("u64 has alignment >= 2");

        // addr should return the address without the tag bit
        let addr = tagged.addr();
        assert_eq!(addr & 1, 0, "Address should have tag bit cleared");

        unsafe { tagged.into_box() };
    }

    #[test]
    fn test_try_new_with_valid_alignment() {
        // u64 has alignment >= 2, should succeed
        let value = Box::new(42u64);
        let tagged = TaggedPtr::try_new(value, false);
        assert!(tagged.is_some());

        let tagged = tagged.unwrap();
        assert_eq!(unsafe { *tagged.as_ptr() }, 42);
        assert!(!tagged.tag());

        unsafe { tagged.into_box() };
    }

    #[test]
    fn test_try_new_with_tag() {
        let value = Box::new(999u64);
        let tagged = TaggedPtr::try_new(value, true);
        assert!(tagged.is_some());

        let tagged = tagged.unwrap();
        assert!(tagged.tag());
        assert_eq!(unsafe { *tagged.as_ptr() }, 999);

        unsafe { tagged.into_box() };
    }

    #[test]
    fn test_new_unchecked_skips_validation() {
        // new_unchecked should skip alignment validation
        let value = Box::new(42u64);
        let tagged = unsafe { TaggedPtr::new_unchecked(value, true) };
        assert!(tagged.tag());
        assert_eq!(unsafe { *tagged.as_ptr() }, 42);

        unsafe { tagged.into_box() };
    }

    // Edge case tests for CRITICAL security fixes

    #[test]
    fn test_try_new_rejects_u8_alignment() {
        // u8 has alignment 1, which is insufficient for tagging
        let value = Box::new(42u8);
        let tagged = TaggedPtr::try_new(value, false);
        // Should return None because alignment < 2
        assert!(tagged.is_none(), "Should reject types with alignment < 2");
    }

    #[test]
    fn test_new_rejects_u8_alignment() {
        // new() should also reject types with alignment < 2
        let value = Box::new(42u8);
        let tagged = TaggedPtr::new(value, false);
        // Should return None because alignment < 2
        assert!(tagged.is_none(), "new() should reject types with alignment < 2");
    }

    #[test]
    fn test_tagged_ptr_with_struct_having_alignment() {
        // Most structs have alignment >= 2
        #[repr(C)]
        struct AlignedStruct {
            a: u32,
            b: u32,
        }

        let value = Box::new(AlignedStruct { a: 1, b: 2 });
        let tagged = TaggedPtr::new(value, false);
        assert!(tagged.is_some(), "Struct with alignment >= 2 should work");

        let tagged = tagged.unwrap();
        assert_eq!(unsafe { tagged.as_ptr().a }, 1);
        assert_eq!(unsafe { tagged.as_ptr().b }, 2);

        unsafe { tagged.into_box() };
    }

    #[test]
    fn test_tagged_ptr_zero_value() {
        // Test with zero value to ensure pointer tagging doesn't affect value
        let value = Box::new(0u64);
        let tagged = TaggedPtr::new(value, false).expect("u64 has alignment >= 2");
        assert_eq!(unsafe { *tagged.as_ptr() }, 0);

        unsafe { tagged.into_box() };
    }

    #[test]
    fn test_tagged_ptr_max_value() {
        // Test with max value to ensure pointer tagging doesn't affect value
        let value = Box::new(u64::MAX);
        let tagged = TaggedPtr::new(value, false).expect("u64 has alignment >= 2");
        assert_eq!(unsafe { *tagged.as_ptr() }, u64::MAX);

        unsafe { tagged.into_box() };
    }

    #[test]
    fn test_tagged_ptr_alternating_tags() {
        // Test rapid tag changes to ensure value is preserved
        let value = Box::new(111u64);
        let mut tagged = TaggedPtr::new(value, false).expect("u64 has alignment >= 2");

        for _ in 0..10 {
            tagged.set_tag(true);
            assert_eq!(unsafe { *tagged.as_ptr() }, 111);

            tagged.set_tag(false);
            assert_eq!(unsafe { *tagged.as_ptr() }, 111);
        }

        unsafe { tagged.into_box() };
    }

    #[test]
    fn test_multiple_tagged_pointers_same_value() {
        // Test multiple tagged pointers pointing to the same value
        let value = Box::new(222u64);
        let ptr = Box::leak(value) as *const u64;

        let tagged1 = unsafe { TaggedPtr::new_unchecked(Box::from_raw(ptr as *mut u64), false) };
        let tagged2 = unsafe { TaggedPtr::new_unchecked(Box::from_raw(ptr as *mut u64), true) };

        assert_eq!(unsafe { *tagged1.as_ptr() }, 222);
        assert_eq!(unsafe { *tagged2.as_ptr() }, 222);
        assert!(!tagged1.tag());
        assert!(tagged2.tag());

        unsafe { tagged1.into_box() };
        unsafe { tagged2.into_box() };
    }
}
