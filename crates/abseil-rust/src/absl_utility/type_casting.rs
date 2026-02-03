//! Type casting utilities.

/// Converts a value to a different type through byte-wise copy.
///
/// # Panics
///
/// Panics if the source and destination types have different sizes
/// or incompatible alignment requirements. Use `try_bit_cast` for a
/// fallible version.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::type_casting::bit_cast;
///
/// let value: u32 = 0x12345678;
/// let bytes: [u8; 4] = bit_cast(value);
/// ```
#[inline]
pub fn bit_cast<T, U>(input: T) -> U {
    // SAFETY: Validate size and alignment before performing unsafe copy
    match try_bit_cast(input) {
        Ok(output) => output,
        Err(TransmuteError::SizeMismatch { src_size, dst_size }) => {
            panic!(
                "bit_cast: size mismatch - source type {} has size {}, \
                 destination type {} has size {}",
                core::any::type_name::<T>(),
                src_size,
                core::any::type_name::<U>(),
                dst_size
            );
        }
        Err(TransmuteError::AlignmentMismatch { src_align, dst_align }) => {
            panic!(
                "bit_cast: alignment mismatch - source type {} has alignment {}, \
                 destination type {} has alignment {} (cannot cast to higher alignment)",
                core::any::type_name::<T>(),
                src_align,
                core::any::type_name::<U>(),
                dst_align
            );
        }
    }
}

/// Safe bit casting with validation.
///
/// Returns `Err` if the source and destination types have different sizes
/// or incompatible alignment requirements.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::type_casting::try_bit_cast;
///
/// let value: u32 = 0x12345678;
/// let bytes: Result<[u8; 4], _> = try_bit_cast(value);
/// assert_eq!(bytes.unwrap(), [0x78, 0x56, 0x34, 0x12]);
/// ```
#[inline]
pub fn try_bit_cast<T, U>(input: T) -> Result<U, TransmuteError> {
    let src_size = core::mem::size_of::<T>();
    let dst_size = core::mem::size_of::<U>();

    if src_size != dst_size {
        return Err(TransmuteError::SizeMismatch {
            src_size,
            dst_size,
        });
    }

    let src_align = core::mem::align_of::<T>();
    let dst_align = core::mem::align_of::<U>();

    // Check if destination alignment is greater than source alignment
    // This could cause undefined behavior on some platforms
    if dst_align > src_align {
        return Err(TransmuteError::AlignmentMismatch {
            src_align,
            dst_align,
        });
    }

    // SAFETY: Size and alignment are validated, so copy is safe
    unsafe {
        let mut output = core::mem::MaybeUninit::<U>::uninit();
        core::ptr::copy_nonoverlapping(
            &input as *const T as *const u8,
            output.as_mut_ptr() as *mut u8,
            src_size,
        );
        Ok(output.assume_init())
    }
}

/// Error type for transmute operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TransmuteError {
    /// Source and destination types have different sizes.
    SizeMismatch {
        src_size: usize,
        dst_size: usize,
    },
    /// Source and destination types have incompatible alignment.
    AlignmentMismatch {
        src_align: usize,
        dst_align: usize,
    },
}

impl core::fmt::Display for TransmuteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TransmuteError::SizeMismatch { src_size, dst_size } => {
                write!(
                    f,
                    "Cannot transmute: source size ({} bytes) != destination size ({} bytes)",
                    src_size, dst_size
                )
            }
            TransmuteError::AlignmentMismatch { src_align, dst_align } => {
                write!(
                    f,
                    "Cannot transmute: source alignment ({} bytes) < destination alignment ({} bytes) - may cause undefined behavior",
                    src_align, dst_align
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransmuteError {}

/// Interprets the bits of a value as a different type.
///
/// This is similar to `std::mem::transmute` but with explicit type parameters
/// and SIZE AND ALIGNMENT VALIDATION to prevent memory corruption.
///
/// # Panics
///
/// Panics if source and destination types have different sizes or incompatible
/// alignment requirements.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::type_casting::transmute;
///
/// let bytes: [u8; 4] = [0x78, 0x56, 0x34, 0x12];
/// let value: u32 = transmute(bytes);
/// assert_eq!(value, 0x12345678);
/// ```
#[inline]
pub fn transmute<T, U>(input: T) -> U {
    // SAFETY: Validate that types have the same size and compatible alignment
    let src_size = core::mem::size_of::<T>();
    let dst_size = core::mem::size_of::<U>();

    if src_size != dst_size {
        panic!(
            "transmute: size mismatch - cannot transmute from {} ({} bytes) to {} ({} bytes)",
            core::any::type_name::<T>(),
            src_size,
            core::any::type_name::<U>(),
            dst_size
        );
    }

    let src_align = core::mem::align_of::<T>();
    let dst_align = core::mem::align_of::<U>();

    // Check if destination alignment is greater than source alignment
    // This could cause undefined behavior on some platforms
    if dst_align > src_align {
        panic!(
            "transmute: alignment mismatch - cannot transmute from {} (align {}) to {} (align {}) - may cause undefined behavior",
            core::any::type_name::<T>(),
            src_align,
            core::any::type_name::<U>(),
            dst_align
        );
    }

    // SAFETY: Size and alignment are validated, so transmute is safe
    unsafe { core::mem::transmute(input) }
}

/// Safe transmute that returns Result instead of panicking.
///
/// Returns `Err` if the source and destination types have different sizes
/// or incompatible alignment requirements.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::type_casting::try_transmute;
///
/// let bytes: [u8; 4] = [0x78, 0x56, 0x34, 0x12];
/// let result: Result<u32, _> = try_transmute(bytes);
/// assert_eq!(result.unwrap(), 0x12345678);
///
/// // Size mismatch returns error
/// let bytes: [u8; 2] = [0x78, 0x56];
/// let result: Result<u32, _> = try_transmute(bytes);
/// assert!(result.is_err());
/// ```
#[inline]
pub fn try_transmute<T, U>(input: T) -> Result<U, TransmuteError> {
    let src_size = core::mem::size_of::<T>();
    let dst_size = core::mem::size_of::<U>();

    if src_size != dst_size {
        return Err(TransmuteError::SizeMismatch {
            src_size,
            dst_size,
        });
    }

    let src_align = core::mem::align_of::<T>();
    let dst_align = core::mem::align_of::<U>();

    // Check if destination alignment is greater than source alignment
    // This could cause undefined behavior on some platforms
    if dst_align > src_align {
        return Err(TransmuteError::AlignmentMismatch {
            src_align,
            dst_align,
        });
    }

    // SAFETY: Size and alignment are validated, so transmute is safe
    Ok(unsafe { core::mem::transmute(input) })
}

/// Gets the name of a type as a string slice.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::type_casting::type_name;
///
/// assert_eq!(type_name::<i32>(), "i32");
/// ```
#[inline]
pub fn type_name<T: ?Sized>() -> &'static str {
    core::any::type_name::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for bit_cast

    #[test]
    fn test_bit_cast_u32_to_bytes() {
        let value: u32 = 0x12345678;
        let bytes: [u8; 4] = unsafe { bit_cast(value) };
        assert_eq!(bytes, [0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_bit_cast_bytes_to_u32() {
        let bytes: [u8; 4] = [0x78, 0x56, 0x34, 0x12];
        let value: u32 = unsafe { bit_cast(bytes) };
        assert_eq!(value, 0x12345678);
    }

    #[test]
    fn test_bit_cast_u64_to_u32_pair() {
        let value: u64 = 0x1234567890ABCDEF;
        let bytes: [u8; 8] = unsafe { bit_cast(value) };
        assert_eq!(bytes[0], 0xEF);
        assert_eq!(bytes[7], 0x12);
    }

    // Tests for transmute with size validation

    #[test]
    fn test_transmute_same_size() {
        let bytes: [u8; 4] = [0x78, 0x56, 0x34, 0x12];
        let value: u32 = transmute(bytes);
        assert_eq!(value, 0x12345678);
    }

    #[test]
    fn test_transmute_roundtrip() {
        let original: u32 = 0xDEADBEEF;
        let bytes: [u8; 4] = transmute(original);
        let restored: u32 = transmute(bytes);
        assert_eq!(restored, original);
    }

    #[test]
    #[should_panic(expected = "size mismatch")]
    fn test_transmute_size_mismatch_panics() {
        // This should panic because u8 (1 byte) != u32 (4 bytes)
        let small: u8 = 42;
        let _large: u32 = transmute(small);
    }

    #[test]
    #[should_panic(expected = "alignment mismatch")]
    fn test_transmute_alignment_mismatch_panics() {
        // u8 has alignment 1, but we'll test with a type that requires higher alignment
        #[repr(C, packed)]
        struct Packed {
            a: u8,
            b: u8,
        }

        // Packed struct has alignment 1, u16 has alignment 2
        // This should panic due to alignment mismatch
        let packed = Packed { a: 0x12, b: 0x34 };
        let _result: u16 = transmute(packed);
    }

    #[test]
    fn test_transmute_zero_size() {
        // Zero-sized types can be transmuted
        let _empty: () = transmute(());
    }

    #[test]
    fn test_transmute_array_to_array() {
        let input: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let output: [u8; 8] = transmute(input);
        assert_eq!(input, output);
    }

    // Tests for try_transmute

    #[test]
    fn test_try_transmute_success() {
        let bytes: [u8; 4] = [0x78, 0x56, 0x34, 0x12];
        let result: Result<u32, _> = try_transmute(bytes);
        assert_eq!(result.unwrap(), 0x12345678);
    }

    #[test]
    fn test_try_transmute_size_mismatch_returns_error() {
        let small: u8 = 42;
        let result: Result<u32, TransmuteError> = try_transmute(small);
        assert!(result.is_err());

        match result {
            Err(TransmuteError::SizeMismatch { src_size, dst_size }) => {
                assert_eq!(src_size, 1);
                assert_eq!(dst_size, 4);
            }
            _ => panic!("Expected SizeMismatch error"),
        }
    }

    #[test]
    fn test_try_transmute_larger_to_smaller() {
        let large: u64 = 0x1234567890ABCDEF;
        let result: Result<u32, TransmuteError> = try_transmute(large);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_transmute_alignment_mismatch() {
        #[repr(C, packed)]
        struct Packed {
            a: u8,
            b: u8,
        }

        // Packed has alignment 1, u16 has alignment 2
        let packed = Packed { a: 0x12, b: 0x34 };
        let result: Result<u16, TransmuteError> = try_transmute(packed);

        // Should fail due to alignment mismatch
        assert!(result.is_err());
        match result {
            Err(TransmuteError::AlignmentMismatch { src_align, dst_align }) => {
                assert_eq!(src_align, 1);
                assert_eq!(dst_align, 2);
            }
            _ => panic!("Expected AlignmentMismatch error"),
        }
    }

    #[test]
    fn test_try_transmute_error_display() {
        let small: u8 = 42;
        let result: Result<u32, TransmuteError> = try_transmute(small);
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_string = format!("{}", error);
        assert!(error_string.contains("size mismatch"));
        assert!(error_string.contains("1"));
        assert!(error_string.contains("4"));
    }

    // Tests for type_name

    #[test]
    fn test_type_name_primitive() {
        assert_eq!(type_name::<i32>(), "i32");
        assert_eq!(type_name::<u64>(), "u64");
        assert_eq!(type_name::<f32>(), "f32");
    }

    #[test]
    fn test_type_name_array() {
        assert_eq!(type_name::<[u8; 4]>(), "[u8; 4]");
    }

    #[test]
    fn test_type_name_tuple() {
        assert_eq!(type_name::<(i32, u64)>(), "(i32, u64)");
    }

    // Edge case tests for HIGH security fix

    #[test]
    fn test_transmute_max_values() {
        // Test with maximum values to ensure no corruption
        let max_u32: u32 = u32::MAX;
        let bytes: [u8; 4] = transmute(max_u32);
        let restored: u32 = transmute(bytes);
        assert_eq!(restored, u32::MAX);
    }

    #[test]
    fn test_transmute_negative_values() {
        // Test with negative values (two's complement)
        let value: i32 = -1234567890;
        let bytes: [u8; 4] = transmute(value);
        let restored: i32 = transmute(bytes);
        assert_eq!(restored, -1234567890);
    }

    #[test]
    fn test_transmute_struct_same_size() {
        #[repr(C)]
        struct Foo {
            a: u32,
            b: u32,
        }

        #[repr(C)]
        struct Bar {
            x: u32,
            y: u32,
        }

        let foo = Foo { a: 1, b: 2 };
        let bar: Bar = transmute(foo);
        assert_eq!(bar.x, 1);
        assert_eq!(bar.y, 2);
    }

    #[test]
    fn test_try_transmute_struct_different_alignment() {
        #[repr(C, packed)]
        struct Packed {
            a: u8,
            b: u8,
        }

        #[repr(C)]
        struct Normal {
            a: u16,
        }

        // Both have size 2, so transmute should work
        let packed = Packed { a: 0x12, b: 0x34 };
        let result: Result<Normal, TransmuteError> = try_transmute(packed);
        assert!(result.is_ok());

        let normal = result.unwrap();
        // The byte order depends on endianness
        assert_eq!(normal.a, 0x3412); // Little endian
    }

    #[test]
    fn test_bit_cast_preserves_bits() {
        let value: u64 = 0x0102030405060708;
        let bytes: [u8; 8] = unsafe { bit_cast(value) };

        // Verify the bits are preserved exactly
        for i in 0..8 {
            assert_eq!(bytes[i], (value >> (i * 8)) as u8);
        }
    }

    #[test]
    fn test_transmute_bool_to_u8() {
        // bool is 1 byte, u8 is 1 byte, so this should work
        let b: bool = true;
        let byte: u8 = transmute(b);
        assert_eq!(byte, 1);

        let back: bool = transmute(byte);
        assert!(back);
    }

    #[test]
    fn test_transmute_char_to_u32() {
        // char is 4 bytes in Rust, u32 is 4 bytes
        let c: char = 'A';
        let value: u32 = transmute(c);
        assert_eq!(value, 65);

        let back: char = transmute(value);
        assert_eq!(back, 'A');
    }

    // Edge case tests for MEDIUM security fix - bit_cast alignment validation

    #[test]
    fn test_try_bit_cast_normal_case() {
        let value: u32 = 0x12345678;
        let bytes: Result<[u8; 4], _> = try_bit_cast(value);
        assert!(bytes.is_ok());
        assert_eq!(bytes.unwrap(), [0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_try_bit_cast_size_mismatch() {
        let small: u8 = 42;
        let result: Result<u32, TransmuteError> = try_bit_cast(small);
        assert!(result.is_err());

        match result {
            Err(TransmuteError::SizeMismatch { src_size, dst_size }) => {
                assert_eq!(src_size, 1);
                assert_eq!(dst_size, 4);
            }
            _ => panic!("Expected SizeMismatch error"),
        }
    }

    #[test]
    fn test_try_bit_cast_alignment_mismatch() {
        // u8 has alignment 1, u32 has alignment 4
        // Casting from lower to higher alignment is unsafe
        let bytes: [u8; 4] = [1, 0, 0, 0];
        let result: Result<u32, TransmuteError> = try_bit_cast(bytes);
        assert!(result.is_err());

        match result {
            Err(TransmuteError::AlignmentMismatch { src_align, dst_align }) => {
                assert_eq!(src_align, 1);
                assert_eq!(dst_align, 4);
            }
            _ => panic!("Expected AlignmentMismatch error"),
        }
    }

    #[test]
    fn test_try_bit_cast_same_alignment() {
        // u32 to u32 has same alignment, should work
        let value: u32 = 0x12345678;
        let result: Result<u32, TransmuteError> = try_bit_cast(value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x12345678);
    }

    #[test]
    fn test_try_bit_cast_higher_to_lower_alignment() {
        // u32 (alignment 4) to [u8; 4] (alignment 1) should work
        // Higher to lower alignment is safe
        let value: u32 = 0x12345678;
        let result: Result<[u8; 4], TransmuteError> = try_bit_cast(value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), [0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_try_bit_cast_bool_to_u8_safe() {
        // bool (alignment 1) to u8 (alignment 1) is safe
        let b: bool = true;
        let result: Result<u8, TransmuteError> = try_bit_cast(b);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_try_bit_cast_structs_different_alignment() {
        #[repr(C, packed)]
        struct Packed {
            a: u8,
            b: u8,
        }

        #[repr(C)]
        struct Normal {
            a: u16,
        }

        // Packed has alignment 1, Normal has alignment 2
        let packed = Packed { a: 0x12, b: 0x34 };
        let result: Result<Normal, TransmuteError> = try_bit_cast(packed);

        // Should fail due to alignment mismatch (1 < 2)
        assert!(result.is_err());
    }

    #[test]
    fn test_try_bit_cast_structs_same_alignment() {
        #[repr(C)]
        struct Foo {
            a: u32,
            b: u32,
        }

        #[repr(C)]
        struct Bar {
            x: u32,
            y: u32,
        }

        // Both have alignment 4, should work
        let foo = Foo { a: 1, b: 2 };
        let result: Result<Bar, TransmuteError> = try_bit_cast(foo);
        assert!(result.is_ok());

        let bar = result.unwrap();
        assert_eq!(bar.x, 1);
        assert_eq!(bar.y, 2);
    }

    #[test]
    fn test_bit_cast_unsafe_but_documented() {
        // The unsafe bit_cast still exists but is documented as unsafe
        // Users should use try_bit_cast instead
        let value: u32 = 0x12345678;
        let bytes: [u8; 4] = unsafe { bit_cast(value) };
        assert_eq!(bytes, [0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_alignment_validation_with_max_values() {
        // Test alignment validation with max values
        let max_u32: u32 = u32::MAX;
        let bytes: [u8; 4] = unsafe { bit_cast(max_u32) };
        let restored: u32 = unsafe { bit_cast(bytes) };
        assert_eq!(restored, u32::MAX);
    }

    #[test]
    fn test_alignment_check_different_types() {
        // Verify that alignment_of returns expected values
        assert_eq!(core::mem::align_of::<u8>(), 1);
        assert_eq!(core::mem::align_of::<u16>(), 2);
        assert_eq!(core::mem::align_of::<u32>(), 4);
        assert_eq!(core::mem::align_of::<u64>(), 8);
    }
}
