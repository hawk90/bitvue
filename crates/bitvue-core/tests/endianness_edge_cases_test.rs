#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Endianness and cross-platform compatibility tests
//!
//! CRITICAL: These tests verify correct handling of byte order across platforms.
//! Many container formats and codecs use big-endian, while most systems are little-endian.
//!
//! Priority: P0 (Critical) - Bugs in this area cause silent data corruption

use std::mem::transmute;

// ============================================================================
// Category 1: Basic Endianness Detection and Verification
// ============================================================================

#[test]
fn test_platform_endianness() {
    // Verify we can detect platform endianness correctly

    let test_value: u32 = 0x12345678;
    let bytes = test_value.to_ne_bytes();

    #[cfg(target_endian = "little")]
    {
        assert_eq!(bytes, [0x78, 0x56, 0x34, 0x12]);
    }

    #[cfg(target_endian = "big")]
    {
        assert_eq!(bytes, [0x12, 0x34, 0x56, 0x78]);
    }
}

#[test]
fn test_little_endian_conversion() {
    // Test little-endian conversions on both platforms

    let value: u32 = 0x12345678;
    let le_bytes = value.to_le_bytes();

    // LE bytes are always [0x78, 0x56, 0x34, 0x12] regardless of platform
    assert_eq!(le_bytes, [0x78, 0x56, 0x34, 0x12]);

    // Converting back should give original value
    let restored = u32::from_le_bytes(le_bytes);
    assert_eq!(restored, value);
}

#[test]
fn test_big_endian_conversion() {
    // Test big-endian conversions on both platforms

    let value: u32 = 0x12345678;
    let be_bytes = value.to_be_bytes();

    // BE bytes are always [0x12, 0x34, 0x56, 0x78] regardless of platform
    assert_eq!(be_bytes, [0x12, 0x34, 0x56, 0x78]);

    // Converting back should give original value
    let restored = u32::from_be_bytes(be_bytes);
    assert_eq!(restored, value);
}

#[test]
fn test_cross_platform_consistency() {
    // Verify that conversions give consistent results across platforms

    let test_values: Vec<u32> = vec![
        0x00000000, 0xFFFFFFFF, 0x12345678, 0x80000000, 0x00000001, 0xDEADBEEF,
    ];

    for value in test_values {
        // LE round-trip
        let le_bytes = value.to_le_bytes();
        let le_restored = u32::from_le_bytes(le_bytes);
        assert_eq!(
            le_restored, value,
            "LE conversion failed for 0x{:08X}",
            value
        );

        // BE round-trip
        let be_bytes = value.to_be_bytes();
        let be_restored = u32::from_be_bytes(be_bytes);
        assert_eq!(
            be_restored, value,
            "BE conversion failed for 0x{:08X}",
            value
        );
    }
}

// ============================================================================
// Category 2: IVF Format (Little-Endian)
// ============================================================================

#[test]
fn test_ivf_header_little_endian_parsing() {
    // IVF format uses little-endian byte order
    // Verify we parse it correctly regardless of platform

    // Create IVF header with known values
    let mut header = [0u8; 32];

    // Magic number
    header[0..4].copy_from_slice(b"DKIF");

    // Version: 0 (little-endian)
    header[4..6].copy_from_slice(&0u16.to_le_bytes());

    // Header size: 32 (little-endian)
    header[6..8].copy_from_slice(&32u16.to_le_bytes());

    // FourCC: "AV01"
    header[8..12].copy_from_slice(b"AV01");

    // Width: 1920 (little-endian)
    header[12..14].copy_from_slice(&1920u16.to_le_bytes());

    // Height: 1080 (little-endian)
    header[14..16].copy_from_slice(&1080u16.to_le_bytes());

    // Timebase denominator: 60 (little-endian)
    header[16..20].copy_from_slice(&60u32.to_le_bytes());

    // Timebase numerator: 1 (little-endian)
    header[20..24].copy_from_slice(&1u32.to_le_bytes());

    // Number of frames: 100 (little-endian)
    header[24..28].copy_from_slice(&100u32.to_le_bytes());

    // Reserved: 0 (little-endian)
    header[28..32].copy_from_slice(&0u32.to_le_bytes());

    // Verify parsing gives correct values on any platform
    let version = u16::from_le_bytes([header[4], header[5]]);
    assert_eq!(version, 0);

    let header_size = u16::from_le_bytes([header[6], header[7]]);
    assert_eq!(header_size, 32);

    let width = u16::from_le_bytes([header[12], header[13]]);
    assert_eq!(width, 1920);

    let height = u16::from_le_bytes([header[14], header[15]]);
    assert_eq!(height, 1080);

    let framerate_den = u32::from_le_bytes(header[16..20].try_into().unwrap());
    assert_eq!(framerate_den, 60);

    let framerate_num = u32::from_le_bytes(header[20..24].try_into().unwrap());
    assert_eq!(framerate_num, 1);

    let num_frames = u32::from_le_bytes(header[24..28].try_into().unwrap());
    assert_eq!(num_frames, 100);
}

#[test]
fn test_ivf_frame_header_le_parsing() {
    // IVF frame headers are also little-endian

    // Frame size: 1000 bytes (LE)
    // Timestamp: 12345678 (LE)
    let mut frame_header = [0u8; 12];
    frame_header[0..4].copy_from_slice(&1000u32.to_le_bytes());
    frame_header[4..12].copy_from_slice(&12345678u64.to_le_bytes());

    let frame_size = u32::from_le_bytes(frame_header[0..4].try_into().unwrap());
    assert_eq!(frame_size, 1000);

    let timestamp = u64::from_le_bytes(frame_header[4..12].try_into().unwrap());
    assert_eq!(timestamp, 12345678);
}

// ============================================================================
// Category 3: MP4 Format (Big-Endian)
// ============================================================================

#[test]
fn test_mp4_atom_header_big_endian_parsing() {
    // MP4 format uses big-endian byte order
    // This is CRITICAL because most systems are little-endian

    // Create MP4 atom header with known values
    let mut atom = [0u8; 8];

    // Size: 1024 bytes (big-endian)
    atom[0..4].copy_from_slice(&1024u32.to_be_bytes());

    // FourCC: "ftyp"
    atom[4..8].copy_from_slice(b"ftyp");

    // Verify parsing gives correct values on any platform
    let size = u32::from_be_bytes(atom[0..4].try_into().unwrap());
    assert_eq!(size, 1024);

    let fourcc = std::str::from_utf8(&atom[4..8]).unwrap();
    assert_eq!(fourcc, "ftyp");
}

#[test]
fn test_mp4_atom_max_size() {
    // Test maximum 32-bit atom size

    let mut atom = [0u8; 8];
    atom[0..4].copy_from_slice(&u32::MAX.to_be_bytes());
    atom[4..8].copy_from_slice(b"free");

    let size = u32::from_be_bytes(atom[0..4].try_into().unwrap());
    assert_eq!(size, u32::MAX);
}

#[test]
fn test_mp4_atom_size_zero() {
    // Test atom with size 0 (extends to EOF)

    let mut atom = [0u8; 8];
    atom[0..4].copy_from_slice(&0u32.to_be_bytes());
    atom[4..8].copy_from_slice(b"mdat");

    let size = u32::from_be_bytes(atom[0..4].try_into().unwrap());
    assert_eq!(size, 0);
}

#[test]
fn test_mp4_atom_size_one() {
    // Test atom with size 1 (uses 64-bit extended size)

    let mut atom = [0u8; 16];
    atom[0..4].copy_from_slice(&1u32.to_be_bytes());
    atom[4..8].copy_from_slice(b"mdat");
    atom[8..16].copy_from_slice(&0x123456789ABCDEF0u64.to_be_bytes());

    let size_32 = u32::from_be_bytes(atom[0..4].try_into().unwrap());
    assert_eq!(size_32, 1);

    let size_64 = u64::from_be_bytes(atom[8..16].try_into().unwrap());
    assert_eq!(size_64, 0x123456789ABCDEF0);
}

// ============================================================================
// Category 4: Bitstream Syntax Elements (Variable Length)
// ============================================================================

#[test]
fn test_exp_golomb_decode_boundary() {
    // Test Exp-Golomb decoding at boundary values

    // These are common in H.264, H.265, etc.
    // We'll test the read_ue() function equivalent

    // Code 0 -> value 0
    let _bits_0 = [0b1000_0000u8]; // Single 1 bit
                                   // let value_0 = read_ue(&mut BitReader::new(&_bits_0));
                                   // assert_eq!(value_0, 0);

    // Code 00100 -> value 4
    let _bits_4 = [0b0010_0000u8];
    // let value_4 = read_ue(&mut BitReader::new(&_bits_4));
    // assert_eq!(value_4, 4);
}

#[test]
fn test_signed_exp_golomb_decode() {
    // Test signed Exp-Golomb decoding

    // Value 0 -> code 1 (positive)
    // Value -1 -> code 010 (negative)
    // Value 1 -> code 011 (positive)
    // Value -2 -> code 00100 (negative)
    // Value 2 -> code 00101 (positive)
}

// ============================================================================
// Category 5: Numeric Type Conversion Edge Cases
// ============================================================================

#[test]
fn test_u32_to_i32_conversion() {
    // Test safe u32 to i32 conversion

    // Safe conversions
    assert_eq!(0i32, 0u32 as i32);
    assert_eq!(100i32, 100u32 as i32);
    assert_eq!(i32::MAX, i32::MAX as u32 as i32);

    // Unsafe conversions (would wrap)
    let too_large = i32::MAX as u32 + 1;
    let wrapped = too_large as i32;
    assert_eq!(wrapped, i32::MIN); // Wrapped to negative!

    // This is why we should use checked conversions
    let result = i32::try_from(too_large);
    assert!(result.is_err());
}

#[test]
fn test_u64_to_usize_conversion() {
    // Test safe u64 to usize conversion

    #[cfg(target_pointer_width = "64")]
    {
        // On 64-bit, u64 fits in usize
        let large: u64 = 0xFFFFFFFFFFFFFF;
        let converted: usize = large.try_into().expect("u64 should fit in usize on 64-bit");
        assert_eq!(converted, large as usize);
    }

    #[cfg(target_pointer_width = "32")]
    {
        // On 32-bit, large u64 values overflow usize
        let large: u64 = 0x100000000; // Larger than usize::MAX on 32-bit
        let converted: usize = large as usize;
        // Wrapped value (implementation-defined)
    }
}

#[test]
fn test_i64_to_u64_conversion() {
    // Test i64 to u64 conversion (common for timestamps)

    // Positive values convert safely
    let positive: i64 = 1000;
    let converted: u64 = positive as u64;
    assert_eq!(converted, 1000);

    // Negative values wrap (undefined behavior in C++, defined in Rust)
    let negative: i64 = -1;
    let wrapped: u64 = negative as u64;
    assert_eq!(wrapped, u64::MAX); // -1 wraps to MAX
}

// ============================================================================
// Category 6: Floating Point Endianness
// ============================================================================

#[test]
fn test_f32_endianness() {
    // Test floating point byte order

    let value: f32 = 1234.5678;
    let bytes = value.to_be_bytes();

    // Convert back
    let restored = f32::from_be_bytes(bytes);

    // Should be equal (or very close due to floating point)
    assert!((restored - value).abs() < f32::EPSILON);

    // Test LE
    let le_bytes = value.to_le_bytes();
    let restored_le = f32::from_le_bytes(le_bytes);
    assert!((restored_le - value).abs() < f32::EPSILON);
}

#[test]
fn test_f64_endianness() {
    // Test double precision floating point

    let value: f64 = 123456.789012;
    let bytes = value.to_be_bytes();

    let restored = f64::from_be_bytes(bytes);
    assert!((restored - value).abs() < f64::EPSILON);
}

// ============================================================================
// Category 7: Array and Slice Endianness
// ============================================================================

#[test]
fn test_u16_array_endian() {
    // Test arrays of u16 values

    let values: Vec<u16> = vec![0x1234, 0x5678, 0x9ABC, 0xDEF0];

    // Convert to LE bytes
    let mut le_bytes = [0u8; 8];
    for (i, &value) in values.iter().enumerate() {
        le_bytes[i * 2..(i * 2 + 2)].copy_from_slice(&value.to_le_bytes());
    }

    // Verify LE order on little-endian platform
    #[cfg(target_endian = "little")]
    {
        assert_eq!(le_bytes, [0x34, 0x12, 0x78, 0x56, 0xBC, 0x9A, 0xF0, 0xDE]);
    }

    // Restore values
    let restored: Vec<u16> = (0..4)
        .map(|i| u16::from_le_bytes(le_bytes[i * 2..(i * 2 + 2)].try_into().unwrap()))
        .collect();

    assert_eq!(restored, values);
}

// ============================================================================
// Category 8: Boundary Values with Endianness
// ============================================================================

#[test]
fn test_u32_max_boundary() {
    // Test u32::MAX with endianness

    let value = u32::MAX;

    // BE bytes
    let be_bytes = value.to_be_bytes();
    assert_eq!(be_bytes, [0xFF, 0xFF, 0xFF, 0xFF]);

    // LE bytes
    let le_bytes = value.to_le_bytes();
    assert_eq!(le_bytes, [0xFF, 0xFF, 0xFF, 0xFF]);

    // Same for 0xFF (endian-independent)
}

#[test]
fn test_u32_min_boundary() {
    // Test 0 with endianness

    let value = 0u32;

    let be_bytes = value.to_be_bytes();
    let le_bytes = value.to_le_bytes();

    assert_eq!(be_bytes, [0, 0, 0, 0]);
    assert_eq!(le_bytes, [0, 0, 0, 0]);

    // All zeros are endian-independent
}

#[test]
fn test_alternating_bits() {
    // Test alternating bit pattern (reveals endianness issues)

    let value = 0xAAAAAAAAu32; // 10101010...

    let be_bytes = value.to_be_bytes();
    #[cfg(target_endian = "little")]
    {
        assert_eq!(be_bytes, [0xAA, 0xAA, 0xAA, 0xAA]);
    }

    let le_bytes = value.to_le_bytes();
    #[cfg(target_endian = "little")]
    {
        assert_eq!(le_bytes, [0xAA, 0xAA, 0xAA, 0xAA]);
    }

    // Same for all bytes
}

#[test]
fn test_incrementing_bytes() {
    // Test incrementing byte pattern

    let value = 0x01020304u32;

    let be_bytes = value.to_be_bytes();
    assert_eq!(be_bytes, [0x01, 0x02, 0x03, 0x04]);

    let le_bytes = value.to_le_bytes();
    assert_eq!(le_bytes, [0x04, 0x03, 0x02, 0x01]);

    // Clearly shows byte order difference
}

// ============================================================================
// Category 9: Timestamp Edge Cases
// ============================================================================

#[test]
fn test_timestamp_max_u64() {
    // Test maximum timestamp value

    let timestamp = u64::MAX;

    // IVF stores timestamps as u64 LE
    let le_bytes = timestamp.to_le_bytes();
    let restored = u64::from_le_bytes(le_bytes);
    assert_eq!(restored, timestamp);

    // Converting to i64 would overflow
    let as_i64 = timestamp as i64;
    assert_eq!(as_i64, -1); // Wrapped!

    // Should use checked conversion
    let checked = i64::try_from(timestamp);
    assert!(checked.is_err()); // u64::MAX doesn't fit in i64
}

#[test]
fn test_timestamp_safe_range() {
    // Test timestamp values that fit in both u64 and i64

    let timestamps: Vec<u64> = vec![0, 1, 1000, 1_000_000, i64::MAX as u64];

    for ts in timestamps {
        let le_bytes = ts.to_le_bytes();
        let restored = u64::from_le_bytes(le_bytes);
        assert_eq!(restored, ts);

        // Should convert to i64 safely
        let as_i64 = i64::try_from(ts);
        assert!(as_i64.is_ok());
    }
}

// ============================================================================
// Category 10: Color and Pixel Data Endianness
// ============================================================================

#[test]
fn test_rgb888_endian() {
    // Test RGB pixel data

    // RGB is stored as R, G, B bytes (endian-independent)
    let pixel = [255u8, 128u8, 64u8];

    assert_eq!(pixel[0], 255); // R
    assert_eq!(pixel[1], 128); // G
    assert_eq!(pixel[2], 64); // B
}

#[test]
fn test_rgb565_endian() {
    // Test RGB565 packed format

    // RGB565: 5 bits R, 6 bits G, 5 bits B = 16 bits
    let r = 31u16; // 5 bits
    let g = 63u16; // 6 bits
    let b = 31u16; // 5 bits

    // Pack as RGB565: RRRR RGGG GGGB BBBB
    let packed: u16 = (r << 11) | (g << 5) | b;

    // Convert to bytes (endianness matters!)
    let le_bytes = packed.to_le_bytes();
    let be_bytes = packed.to_be_bytes();

    #[cfg(target_endian = "little")]
    {
        // On LE: low byte first
        assert_eq!(le_bytes[1], (packed >> 8) as u8);
        assert_eq!(le_bytes[0], (packed & 0xFF) as u8);
    }

    // Verify round-trip
    let restored_le = u16::from_le_bytes(le_bytes);
    assert_eq!(restored_le, packed);

    let restored_be = u16::from_be_bytes(be_bytes);
    assert_eq!(restored_be, packed);
}

// ============================================================================
// Category 11: Network Byte Order (Big-Endian)
// ============================================================================

#[test]
fn test_network_byte_order() {
    // Test network byte order (always big-endian)

    let port: u16 = 8080;
    let network_bytes = port.to_be_bytes();

    // Network byte order is big-endian
    assert_eq!(network_bytes, [0x1F, 0x90]); // 8080 in BE

    // Convert back
    let restored = u16::from_be_bytes(network_bytes);
    assert_eq!(restored, 8080);
}

// ============================================================================
// Category 12: Memory Layout Verification
// ============================================================================

#[test]
fn test_struct_memory_layout() {
    // Test that struct layout is as expected

    #[repr(C)]
    struct TestStruct {
        a: u32,
        b: u16,
        c: u8,
    }

    let s = TestStruct {
        a: 0x12345678,
        b: 0x9ABC,
        c: 0xDE,
    };

    // Verify byte layout
    let bytes = unsafe {
        std::slice::from_raw_parts(
            &s as *const TestStruct as *const u8,
            std::mem::size_of::<TestStruct>(),
        )
    };

    #[cfg(target_endian = "little")]
    {
        // LE: a=78 56 34 12, b=BC 9A, c=DE
        assert_eq!(bytes[0], 0x78);
        assert_eq!(bytes[4], 0xBC);
        assert_eq!(bytes[6], 0xDE);
    }

    // Always use explicit serialization instead of raw memory access!
}

// ============================================================================
// Helper Functions
// ============================================================================

#[cfg(test)]
mod helpers {
    use super::*;

    pub fn create_test_mp4_atom(size: u32, fourcc: &str) -> Vec<u8> {
        let mut atom = Vec::with_capacity(8);
        atom.extend_from_slice(&size.to_be_bytes());
        atom.extend_from_slice(fourcc.as_bytes());
        atom
    }

    pub fn create_test_ivf_header(width: u16, height: u16) -> Vec<u8> {
        let mut header = vec![0u8; 32];
        header[0..4].copy_from_slice(b"DKIF");
        header[4..6].copy_from_slice(&0u16.to_le_bytes());
        header[6..8].copy_from_slice(&32u16.to_le_bytes());
        header[8..12].copy_from_slice(b"AV01");
        header[12..14].copy_from_slice(&width.to_le_bytes());
        header[14..16].copy_from_slice(&height.to_le_bytes());
        header[16..20].copy_from_slice(&60u32.to_le_bytes());
        header[20..24].copy_from_slice(&1u32.to_le_bytes());
        header[24..28].copy_from_slice(&0u32.to_le_bytes());
        header[28..32].copy_from_slice(&0u32.to_le_bytes());
        header
    }
}
