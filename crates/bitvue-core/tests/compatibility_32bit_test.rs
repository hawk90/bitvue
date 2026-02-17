#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! 32-bit vs 64-bit compatibility tests
//!
//! CRITICAL: These tests verify that code works correctly on both 32-bit and 64-bit platforms.
//! Many assumptions about pointer sizes and integer sizes can cause subtle bugs.
//!
//! Priority: P0 (Critical) - 32-bit overflow issues cause crashes or data corruption

// ============================================================================
// Category 1: Pointer Size Assumptions
// ============================================================================

#[test]
fn test_usize_size() {
    // Verify usize is appropriate for platform

    #[cfg(target_pointer_width = "32")]
    {
        assert_eq!(std::mem::size_of::<usize>(), 4);
        assert_eq!(usize::MAX, u32::MAX as usize);
    }

    #[cfg(target_pointer_width = "64")]
    {
        assert_eq!(std::mem::size_of::<usize>(), 8);
        assert_eq!(usize::MAX, u64::MAX as usize);
    }
}

#[test]
fn test_isize_size() {
    // Verify isize is appropriate for platform

    #[cfg(target_pointer_width = "32")]
    {
        assert_eq!(std::mem::size_of::<isize>(), 4);
        assert_eq!(isize::MIN, i32::MIN as isize);
        assert_eq!(isize::MAX, i32::MAX as isize);
    }

    #[cfg(target_pointer_width = "64")]
    {
        assert_eq!(std::mem::size_of::<isize>(), 8);
        assert_eq!(isize::MIN, i64::MIN as isize);
        assert_eq!(isize::MAX, i64::MAX as isize);
    }
}

#[test]
fn test_usize_to_u64_conversion_safe() {
    // Test safe usize to u64 conversion

    let test_values: Vec<usize> = vec![0, 1, 100, 1000, 1_000_000];

    for value in test_values {
        let converted = value as u64;
        assert_eq!(converted as usize, value);
    }

    // Maximum safe value
    #[cfg(target_pointer_width = "32")]
    {
        let max_safe = u32::MAX as usize;
        let converted = max_safe as u64;
        assert_eq!(converted, u32::MAX as u64);
    }

    #[cfg(target_pointer_width = "64")]
    {
        let max_safe = usize::MAX;
        let converted = max_safe as u64;
        assert_eq!(converted, usize::MAX as u64);
    }
}

#[test]
fn test_u64_to_usize_conversion_safe() {
    // Test safe u64 to usize conversion

    // Values that fit in usize on both platforms
    let safe_values: Vec<u64> = vec![0, 1, 100, 1000, 1_000_000, u32::MAX as u64];

    for value in safe_values {
        let converted = value as usize;
        assert_eq!(converted as u64, value);
    }

    // Value that doesn't fit on 32-bit
    let too_large: u64 = u32::MAX as u64 + 1;

    #[cfg(target_pointer_width = "32")]
    {
        let wrapped = too_large as usize;
        // Wrapped value (implementation-defined)
        // In practice, this wraps to 0
        assert_eq!(wrapped, 0);
    }

    #[cfg(target_pointer_width = "64")]
    {
        let converted = too_large as usize;
        assert_eq!(converted as u64, too_large);
    }
}

#[test]
fn test_checked_u64_to_usize() {
    // Test checked conversion from u64 to usize

    // Safe conversion
    let safe: u64 = 1000;
    let result = usize::try_from(safe);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1000);

    // Unsafe conversion on 32-bit
    let unsafe_value: u64 = u32::MAX as u64 + 1;

    #[cfg(target_pointer_width = "32")]
    {
        let result = usize::try_from(unsafe_value);
        assert!(result.is_err());
    }

    #[cfg(target_pointer_width = "64")]
    {
        let result = usize::try_from(unsafe_value);
        assert!(result.is_ok());
    }
}

// ============================================================================
// Category 2: File Offset Handling
// ============================================================================

#[test]
fn test_file_offset_32bit() {
    // Test file offsets that fit in u32

    let offsets: Vec<u64> = vec![0, 1, 1000, 1_000_000, 100_000_000, u32::MAX as u64];

    for offset in offsets {
        // Should always be safe to convert to usize if value fits
        if offset <= usize::MAX as u64 {
            let as_usize = offset as usize;
            assert_eq!(as_usize as u64, offset);
        }
    }
}

#[test]
fn test_file_offset_overflow_32bit() {
    // Test file offsets that overflow on 32-bit

    let large_offset: u64 = u32::MAX as u64 + 1;

    #[cfg(target_pointer_width = "32")]
    {
        // Cannot convert to usize without overflow
        let result = usize::try_from(large_offset);
        assert!(result.is_err());

        // Using `as` would wrap
        let wrapped = large_offset as usize;
        assert_eq!(wrapped, 0); // Wrapped to 0
    }

    #[cfg(target_pointer_width = "64")]
    {
        let result = usize::try_from(large_offset);
        assert!(result.is_ok());
    }
}

#[test]
fn test_seek_beyond_32bit_limit() {
    // Test seeking beyond 4GB on 32-bit

    let offset_5gb: u64 = 5 * 1024 * 1024 * 1024;

    #[cfg(target_pointer_width = "32")]
    {
        // Cannot represent this offset as usize
        let result = usize::try_from(offset_5gb);
        assert!(result.is_err());

        // Seeking would need to use u64 directly, not usize
        // e.g., `seek(SeekFrom::Start(offset_5gb))` not `seek(offset_5gb as usize)`
    }

    #[cfg(target_pointer_width = "64")]
    {
        let result = usize::try_from(offset_5gb);
        assert!(result.is_ok());
    }
}

// ============================================================================
// Category 3: Buffer Size Calculations
// ============================================================================

#[test]
fn test_buffer_size_multiplication_no_overflow() {
    // Test buffer size calculations that don't overflow

    let width: u32 = 1920;
    let height: u32 = 1080;
    let bytes_per_pixel: u32 = 2;

    // Calculate buffer size
    let size = (width * height) as usize * bytes_per_pixel as usize;

    // Should fit in usize on both platforms
    assert!(size < usize::MAX);

    #[cfg(target_pointer_width = "32")]
    {
        // On 32-bit, this should still fit
        assert!(size < u32::MAX as usize);
    }
}

#[test]
fn test_buffer_size_multiplication_overflow() {
    // Test buffer size calculations that overflow

    let width: u32 = 100_000;
    let height: u32 = 100_000;
    let bytes_per_pixel: u32 = 4;

    // This would overflow u32 and usize on 32-bit
    let product = (width as u64) * (height as u64) * (bytes_per_pixel as u64);

    #[cfg(target_pointer_width = "32")]
    {
        // Check if it fits in usize
        let result = usize::try_from(product);
        assert!(result.is_err(), "Should overflow on 32-bit");

        // Using u32 would overflow
        let product_u32 = width.checked_mul(height);
        assert!(product_u32.is_some()); // Fits in u32

        let with_bpp = product_u32.unwrap().checked_mul(bytes_per_pixel);
        assert!(with_bpp.is_none(), "Should overflow with bytes_per_pixel");
    }

    #[cfg(target_pointer_width = "64")]
    {
        let result = usize::try_from(product);
        assert!(result.is_ok(), "Should fit on 64-bit");
    }
}

#[test]
fn test_checked_buffer_size_calculation() {
    // Test checked arithmetic for buffer sizes

    let width: u32 = 1920;
    let height: u32 = 1080;
    let bytes_per_pixel: u32 = 4;

    // Safe calculation with checks
    let area = width.checked_mul(height);
    assert!(area.is_some());

    let total = area.unwrap().checked_mul(bytes_per_pixel);
    assert!(total.is_some());

    // Convert to usize
    let result = usize::try_from(total.unwrap());
    assert!(result.is_ok());
}

// ============================================================================
// Category 4: Array and Vector Indexing
// ============================================================================

#[test]
fn test_vector_index_safe() {
    // Test vector indexing with safe values

    let data = vec![0u8; 1000];

    // Safe indices
    for i in [0usize, 1, 100, 999] {
        let _ = data[i]; // Should not panic
    }

    // Out of bounds
    let result = std::panic::catch_unwind(|| {
        let _ = data[1000]; // Out of bounds
    });
    assert!(result.is_err());
}

#[test]
fn test_vector_index_as_usize() {
    // Test that indices are correctly converted to usize

    let data = vec![1u8, 2, 3, 4, 5];

    // Use u32 as index (common in video codecs)
    let index_u32: u32 = 2;
    let value = data[index_u32 as usize];
    assert_eq!(value, 3);

    // Use i32 as index (can be negative)
    let index_i32: i32 = 3;
    if index_i32 >= 0 {
        let value = data[index_i32 as usize];
        assert_eq!(value, 4);
    }
}

#[test]
fn test_large_vector_allocation() {
    // Test allocating vectors that might overflow on 32-bit

    #[cfg(target_pointer_width = "32")]
    {
        // Maximum safe size on 32-bit
        let max_safe = isize::MAX as usize / 2;
        let result = Vec::<u8>::with_capacity(max_safe);
        // May fail due to memory limits, but shouldn't overflow

        // This would overflow
        let too_large = usize::MAX;
        let result = std::panic::catch_unwind(|| {
            let _ = Vec::<u8>::with_capacity(too_large);
        });
        assert!(result.is_err());
    }

    #[cfg(target_pointer_width = "64")]
    {
        // On 64-bit, we can theoretically allocate more
        // (though limited by RAM)
        let large_size = 10_000_000_000usize; // 10GB
        let result = std::panic::catch_unwind(|| {
            let _ = Vec::<u8>::with_capacity(large_size);
        });
        // Will likely fail due to memory, not overflow
    }
}

// ============================================================================
// Category 5: Frame Size Calculations
// ============================================================================

#[test]
fn test_4k_frame_size() {
    // Test 4K frame size calculations

    let width: u32 = 3840;
    let height: u32 = 2160;

    // Y plane (8-bit)
    let y_size = (width * height) as usize;

    // UV planes (4:2:0 subsampling)
    let uv_width = width / 2;
    let uv_height = height / 2;
    let uv_size = (uv_width * uv_height) as usize;

    // Total size
    let total = y_size + uv_size * 2;

    // Should fit in usize on both platforms
    #[cfg(target_pointer_width = "32")]
    {
        assert!(total < u32::MAX as usize);
        assert!(y_size < u32::MAX as usize);
        assert!(uv_size < u32::MAX as usize);
    }

    #[cfg(target_pointer_width = "64")]
    {
        assert!(total < usize::MAX);
    }
}

#[test]
fn test_8k_frame_size() {
    // Test 8K frame size calculations

    let width: u32 = 7680;
    let height: u32 = 4320;

    let y_size = (width * height) as u64;
    let uv_width = width / 2;
    let uv_height = height / 2;
    let uv_size = (uv_width * uv_height) as u64;
    let total = y_size + uv_size * 2;

    #[cfg(target_pointer_width = "32")]
    {
        // 8K frame: ~33MB for Y, ~8.3MB for each UV
        // Total: ~50MB - fits on 32-bit
        let result = usize::try_from(total);
        assert!(result.is_ok(), "8K frame should fit in 32-bit usize");
    }

    #[cfg(target_pointer_width = "64")]
    {
        let result = usize::try_from(total);
        assert!(result.is_ok());
    }
}

// ============================================================================
// Category 6: Timestamp and Duration Handling
// ============================================================================

#[test]
fn test_timestamp_in_i64() {
    // Test timestamp values that fit in i64

    let timestamps: Vec<i64> = vec![0, 1, 1000, 1_000_000, i32::MAX as i64, i64::MAX - 1];

    for ts in timestamps {
        // Convert to u64 (if non-negative)
        if ts >= 0 {
            let as_u64 = ts as u64;
            assert_eq!(as_u64 as i64, ts);
        }
    }
}

#[test]
fn test_timestamp_u64_max() {
    // Test maximum timestamp value

    let ts = u64::MAX;

    #[cfg(target_pointer_width = "32")]
    {
        // Cannot convert to usize
        let result = usize::try_from(ts);
        assert!(result.is_err());

        // Converting to i64 overflows
        let as_i64 = ts as i64;
        assert_eq!(as_i64, -1); // Wrapped

        // Use checked conversion
        let checked = i64::try_from(ts);
        assert!(checked.is_err());
    }

    #[cfg(target_pointer_width = "64")]
    {
        let result = usize::try_from(ts);
        assert!(result.is_ok());

        let checked = i64::try_from(ts);
        assert!(checked.is_err()); // u64::MAX doesn't fit in i64
    }
}

// ============================================================================
// Category 7: Memory Allocation Limits
// ============================================================================

#[test]
fn test_max_allocation() {
    // Test maximum allocation size

    #[cfg(target_pointer_width = "32")]
    {
        // On 32-bit, practical limit is < 2GB due to signed isize
        let max_safe = isize::MAX as usize / 2;

        // Try to allocate a large buffer
        let result = std::panic::catch_unwind(|| Vec::<u8>::with_capacity(max_safe));

        // Will likely fail due to memory limits, not overflow
        // But the calculation itself should not overflow
    }

    #[cfg(target_pointer_width = "64")]
    {
        // On 64-bit, we can theoretically allocate much more
        let max_safe = isize::MAX as usize / 2;

        // Still limited by actual RAM
        let result = std::panic::catch_unwind(|| Vec::<u8>::with_capacity(max_safe));

        // Will fail due to memory limits
    }
}

// ============================================================================
// Category 8: Bitstream Parsing
// ============================================================================

#[test]
fn test_bit_offset_32bit() {
    // Test bit offsets that fit in platform limits

    let offsets: Vec<u64> = vec![0, 1, 8, 32, 1000, 1_000_000, u32::MAX as u64];

    for offset in offsets {
        // Should be able to convert to usize
        if offset <= usize::MAX as u64 {
            let as_usize = offset as usize;
            assert_eq!(as_usize as u64, offset);
        }
    }
}

#[test]
fn test_large_bit_offset() {
    // Test very large bit offsets

    let large_offset: u64 = u32::MAX as u64 + 1000;

    #[cfg(target_pointer_width = "32")]
    {
        // Cannot convert to usize
        let result = usize::try_from(large_offset);
        assert!(result.is_err());

        // Would need to use u64 throughout, not usize
        // e.g., `let offset_bits: u64` not `let offset_bits: usize`
    }

    #[cfg(target_pointer_width = "64")]
    {
        let result = usize::try_from(large_offset);
        assert!(result.is_ok());
    }
}

// ============================================================================
// Category 9: Slice and Array Operations
// ============================================================================

#[test]
fn test_slice_len() {
    // Test slice length operations

    let data = vec![0u8; 1000];
    let slice = &data[..];

    // Length is usize
    let len = slice.len();
    assert_eq!(len, 1000);

    // Convert to other types safely
    let len_u32 = u32::try_from(len);
    assert!(len_u32.is_ok());
    assert_eq!(len_u32.unwrap(), 1000);

    #[cfg(target_pointer_width = "32")]
    {
        // On 32-bit, usize is u32
        assert_eq!(len as u32, 1000);
    }
}

#[test]
fn test_slice_offset() {
    // Test slice offset operations

    let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Offset using usize
    let offset: usize = 5;
    let slice = &data[offset..];
    assert_eq!(slice[0], 6);

    // Offset using u32 (common in codecs)
    let offset_u32: u32 = 5;
    let slice = &data[offset_u32 as usize..];
    assert_eq!(slice[0], 6);
}

// ============================================================================
// Category 10: Format String and Display
// ============================================================================

#[test]
fn test_format_pointer_values() {
    // Test formatting pointer-sized values

    let value: usize = 0x12345678;

    // Format as hex
    let hex = format!("{:X}", value);
    assert!(hex.contains("12345678"));

    #[cfg(target_pointer_width = "32")]
    {
        assert_eq!(hex.len(), 8); // 8 hex digits for 32-bit
    }

    #[cfg(target_pointer_width = "64")]
    {
        // May have leading zeros or not
        assert!(hex.len() <= 16);
    }
}

// ============================================================================
// Category 11: Atomic Operations
// ============================================================================

#[test]
fn test_atomic_usize() {
    // Test atomic operations on usize

    use std::sync::atomic::{AtomicUsize, Ordering};

    let value = AtomicUsize::new(0);

    value.store(100, Ordering::SeqCst);
    assert_eq!(value.load(Ordering::SeqCst), 100);

    value.fetch_add(50, Ordering::SeqCst);
    assert_eq!(value.load(Ordering::SeqCst), 150);
}

// ============================================================================
// Category 12: Alignment and Padding
// ============================================================================

#[test]
fn test_struct_alignment() {
    // Test struct alignment on different platforms

    #[repr(C)]
    struct TestStruct {
        a: u8,
        b: u32,
        c: u16,
    }

    assert_eq!(std::mem::align_of::<TestStruct>(), 4);

    #[cfg(target_pointer_width = "32")]
    {
        assert_eq!(std::mem::size_of::<TestStruct>(), 8); // 1 + 3(padding) + 4 + 2
    }

    #[cfg(target_pointer_width = "64")]
    {
        assert_eq!(std::mem::size_of::<TestStruct>(), 8); // Same alignment
    }
}

#[test]
fn test_slice_alignment() {
    // Test slice alignment requirements

    let data = vec![0u8; 100];

    // Aligned slice
    let slice = &data[0..100];
    let ptr = slice.as_ptr();

    // Check alignment
    let alignment = ptr as usize % 4;
    assert_eq!(alignment, 0); // Should be 4-byte aligned
}

// ============================================================================
// Helper Functions
// ============================================================================

#[cfg(test)]
mod helpers {
    use super::*;

    pub fn safe_u64_to_usize(value: u64) -> Option<usize> {
        usize::try_from(value).ok()
    }

    pub fn safe_usize_to_u64(value: usize) -> u64 {
        value as u64
    }

    pub fn calculate_frame_size(width: u32, height: u32, bpp: u32) -> Option<u64> {
        let area = (width as u64).checked_mul(height as u64)?;
        area.checked_mul(bpp as u64)
    }
}
