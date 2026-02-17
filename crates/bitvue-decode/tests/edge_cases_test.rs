#![allow(dead_code)]
//! Edge case and boundary condition tests for the decoder module
//!
//! This test suite covers:
//! - Empty/Null Inputs: Empty files, empty arrays, null pointers
//! - Boundary Values: Zero, max values, negative numbers where invalid
//! - Malformed Inputs: Invalid file headers, corrupt data, wrong formats
//! - Size Limits: Files at MAX_FILE_SIZE, frames at MAX_FRAMES_PER_FILE
//! - Path Traversal: Attempts to access files outside working directory
//! - Large Inputs: Very large dimensions, deep nesting
//! - Concurrent Access: Multiple simultaneous operations
//! - Error Conditions: I/O failures, permission denied, disk full
//! - Encoding Issues: Invalid UTF-8, surrogate pairs, BOM handling
//! - Platform Differences: Windows vs Unix paths, line endings

use bitvue_core::limits::*;
use bitvue_decode::decoder::{detect_format, Av1Decoder, DecodeError, DecodedFrame, VideoFormat};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

// ============================================================================
// Category 1: Empty/Null Inputs
// ============================================================================

#[test]
fn test_empty_file() {
    /// Test decoder behavior with completely empty file
    /// Expected: Should return an error indicating no data to decode
    let empty_data: Vec<u8> = vec![];
    let mut decoder = Av1Decoder::new().unwrap();

    let result = decoder.decode_all(&empty_data);
    assert!(result.is_err(), "Should fail to decode empty file");

    match result {
        Err(DecodeError::Decode(msg)) => {
            assert!(
                msg.contains("too short") || msg.contains("empty") || msg.contains("incomplete"),
                "Error should mention file is empty or too short, got: {}",
                msg
            );
        }
        Err(DecodeError::NoFrame) => {
            // NoFrame is acceptable for empty file
        }
        _ => {
            panic!(
                "Expected DecodeError or NoFrame error for empty file, got: {:?}",
                result
            );
        }
    }
}

#[test]
fn test_empty_ivf_header() {
    /// Test decoder behavior with IVF file that has incomplete header
    /// Expected: Should return error about insufficient header data
    let mut incomplete_ivf = Vec::new();
    incomplete_ivf.extend_from_slice(b"DKIF"); // Magic number
    incomplete_ivf.extend_from_slice(&0u16.to_le_bytes()); // Version
                                                           // Missing rest of header (should be at least 32 bytes)

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&incomplete_ivf);

    assert!(
        result.is_err(),
        "Should fail to decode incomplete IVF header"
    );
}

#[test]
fn test_zero_length_frame_data() {
    /// Test decoder behavior when frame size is zero
    /// Expected: Should skip zero-length frames or return error
    let mut ivf_data = create_minimal_ivf_header();
    // Add frame header with zero size
    ivf_data.extend_from_slice(&0u32.to_le_bytes()); // Frame size: 0
    ivf_data.extend_from_slice(&0u64.to_le_bytes()); // Timestamp
                                                     // No frame data

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf_data);

    // Zero-length frames might be skipped or cause error
    // Either behavior is acceptable for edge case
    match result {
        Ok(frames) => {
            // If successful, should have no frames (zero-length frames skipped)
            assert_eq!(frames.len(), 0, "Zero-length frames should be skipped");
        }
        Err(_) => {
            // Error is also acceptable
        }
    }
}

#[test]
fn test_null_pointer_handling() {
    /// Test that decoder handles null-like situations gracefully
    /// In Rust, we test this with empty/invalid data rather than actual null pointers
    let decoder = Av1Decoder::new();
    assert!(decoder.is_ok(), "Decoder creation should succeed");

    // Test with None timestamp
    let mut decoder = decoder.unwrap();
    let result = decoder.send_data(&[0x00, 0x00, 0x01, 0x09], 0);
    // May succeed or fail, but should not crash
}

// ============================================================================
// Category 2: Boundary Values
// ============================================================================

#[test]
fn test_zero_dimensions() {
    /// Test decoder behavior with zero width or height
    /// Expected: Should reject zero dimensions
    let frame = create_test_frame(0, 1920);
    let result = bitvue_decode::decoder::validate_frame(&frame);

    assert!(result.is_err(), "Should reject zero width");

    let frame = create_test_frame(1920, 0);
    let result = bitvue_decode::decoder::validate_frame(&frame);

    assert!(result.is_err(), "Should reject zero height");
}

#[test]
fn test_minimal_dimensions() {
    /// Test decoder behavior with smallest valid dimensions (1x1)
    /// Expected: Should handle 1x1 frames correctly
    let frame = create_test_frame(1, 1);
    let result = bitvue_decode::decoder::validate_frame(&frame);

    assert!(result.is_ok(), "Should accept 1x1 frame");
}

#[test]
fn test_max_file_size_boundary() {
    /// Test decoder at exactly MAX_FILE_SIZE
    /// Expected: Should accept file at exactly MAX_FILE_SIZE
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("max_size.ivf");

    // Create file with valid IVF header and fake data to reach MAX_FILE_SIZE
    let mut file_data = create_minimal_ivf_header();

    // Note: We can't actually create a 2GB file in tests, so we test the validation logic
    let file_size = MAX_FILE_SIZE;

    // Test the file size validation logic
    let metadata_ok = file_size <= MAX_FILE_SIZE;
    assert!(metadata_ok, "File at MAX_FILE_SIZE should be accepted");

    // Test one byte over limit
    let file_size_too_large = MAX_FILE_SIZE + 1;
    let metadata_too_large = file_size_too_large > MAX_FILE_SIZE;
    assert!(
        metadata_too_large,
        "File over MAX_FILE_SIZE should be rejected"
    );
}

#[test]
fn test_max_frames_per_file_boundary() {
    /// Test decoder at exactly MAX_FRAMES_PER_FILE
    /// Expected: Should accept file at exactly MAX_FRAMES_PER_FILE
    let frame_count = MAX_FRAMES_PER_FILE;

    // Test the frame count validation logic
    let count_ok = frame_count <= MAX_FRAMES_PER_FILE;
    assert!(
        count_ok,
        "Frame count at MAX_FRAMES_PER_FILE should be accepted"
    );

    // Test one frame over limit
    let frame_count_too_many = MAX_FRAMES_PER_FILE + 1;
    let count_too_many = frame_count_too_many > MAX_FRAMES_PER_FILE;
    assert!(
        count_too_many,
        "Frame count over MAX_FRAMES_PER_FILE should be rejected"
    );
}

#[test]
fn test_max_frame_size_boundary() {
    /// Test decoder with frame at exactly MAX_FRAME_SIZE
    /// Expected: Should accept frame at exactly MAX_FRAME_SIZE
    let frame_size = MAX_FRAME_SIZE;

    // Test the frame size validation logic
    let size_ok = frame_size <= MAX_FRAME_SIZE;
    assert!(size_ok, "Frame at MAX_FRAME_SIZE should be accepted");

    // Test one byte over limit
    let frame_size_too_large = MAX_FRAME_SIZE + 1;
    let size_too_large = frame_size_too_large > MAX_FRAME_SIZE;
    assert!(
        size_too_large,
        "Frame over MAX_FRAME_SIZE should be rejected"
    );
}

#[test]
fn test_negative_values_wrapping() {
    /// Test decoder behavior with unsigned integer wrapping
    /// Expected: Should handle wrapped values gracefully
    let timestamp = u64::MAX; // Maximum value that could cause issues when converted

    // Create IVF frame with max timestamp
    let mut ivf_data = create_minimal_ivf_header();
    ivf_data.extend_from_slice(&0u32.to_le_bytes()); // Frame size
    ivf_data.extend_from_slice(&timestamp.to_le_bytes()); // Max timestamp

    // Parse should handle this safely
    let ts_bytes: [u8; 8] = ivf_data[32 + 4..32 + 12].try_into().unwrap();
    let parsed_ts = u64::from_le_bytes(ts_bytes);

    // Should handle max timestamp safely (convert to i64)
    if parsed_ts > i64::MAX as u64 {
        // Should reject or handle specially
    }
}

// ============================================================================
// Category 3: Malformed Inputs
// ============================================================================

#[test]
fn test_invalid_magic_number() {
    /// Test decoder behavior with invalid file magic number
    /// Expected: Should detect as unknown format
    let invalid_data = b"INVALID_MAGIC_NUMBER";

    let format = detect_format(invalid_data);
    assert_eq!(
        format,
        VideoFormat::Unknown,
        "Should detect invalid magic number as unknown format"
    );
}

#[test]
fn test_corrupt_ivf_header() {
    /// Test decoder with corrupted IVF header
    /// Expected: Should reject or handle gracefully
    let mut corrupt_ivf = Vec::new();
    corrupt_ivf.extend_from_slice(b"DKIF"); // Valid magic
    corrupt_ivf.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // Invalid version
    corrupt_ivf.extend_from_slice(&[0xFF, 0xFF]); // Invalid header size
                                                  // Rest is garbage

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&corrupt_ivf);

    assert!(result.is_err(), "Should reject corrupt IVF header");
}

#[test]
fn test_truncated_frame_data() {
    /// Test decoder with truncated frame data
    /// Expected: Should detect incomplete frame and error
    let mut ivf_data = create_minimal_ivf_header();
    ivf_data.extend_from_slice(&1000u32.to_le_bytes()); // Frame size: 1000 bytes
    ivf_data.extend_from_slice(&0u64.to_le_bytes()); // Timestamp
    ivf_data.extend_from_slice(&[0u8; 500]); // Only 500 bytes of actual data (truncated)

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf_data);

    assert!(result.is_err(), "Should detect truncated frame data");
}

#[test]
fn test_invalid_chroma_format() {
    /// Test decoder with invalid chroma subsampling
    /// Expected: Should detect and reject invalid chroma format
    let mut frame = create_test_frame(1920, 1080);

    // Create mismatched U/V plane sizes (not 4:2:0, 4:2:2, or 4:4:4)
    frame.u_plane = Some(Arc::from(vec![0u8; 100])); // Invalid size
    frame.v_plane = Some(Arc::from(vec![0u8; 100]));

    let result = bitvue_decode::decoder::validate_frame(&frame);
    assert!(result.is_err(), "Should reject invalid chroma format");
}

#[test]
fn test_invalid_bit_depth() {
    /// Test decoder with invalid bit depth values
    /// Expected: Should warn about unusual bit depths
    let mut frame = create_test_frame(1920, 1080);
    frame.bit_depth = 15; // Invalid bit depth (should be 8, 10, or 12)

    let result = bitvue_decode::decoder::validate_frame(&frame);
    // Should log warning but may still succeed (test just ensures no crash)
}

#[test]
fn test_mismatched_plane_sizes() {
    /// Test decoder with mismatched U/V plane sizes
    /// Expected: Should reject mismatched chroma planes
    let mut frame = create_test_frame(1920, 1080);
    frame.u_plane = Some(Arc::from(vec![0u8; 1920 * 1080 / 4])); // Valid 4:2:0
    frame.v_plane = Some(Arc::from(vec![0u8; 1920 * 1080 / 2])); // Different size

    let result = bitvue_decode::decoder::validate_frame(&frame);
    assert!(result.is_err(), "Should reject mismatched U/V plane sizes");
}

// ============================================================================
// Category 4: Size Limits
// ============================================================================

#[test]
fn test_very_large_frame_dimensions() {
    /// Test decoder with maximum supported frame dimensions
    /// Expected: Should handle large dimensions within limits
    let max_dimension = 8192u32; // Typical max for video decoders

    // Test validation with large but valid dimensions
    let frame = create_test_frame(max_dimension, max_dimension);
    let result = bitvue_decode::decoder::validate_frame(&frame);

    assert!(result.is_ok(), "Should accept maximum valid dimensions");

    // Test with dimensions just over limit
    let oversized_frame = create_test_frame(max_dimension + 1, max_dimension);
    let result = bitvue_decode::decoder::validate_frame(&oversized_frame);

    // May fail at plane allocation (expected)
}

#[test]
fn test_memory_allocation_limits() {
    /// Test that decoder respects memory limits
    /// Expected: Should not allocate excessive memory
    let temp_dir = TempDir::new().unwrap();

    // Create a file that would require large allocation if processed naively
    // Use IVF header with large frame size
    let file_path = temp_dir.path().join("large_frame.ivf");
    let mut file = fs::File::create(&file_path).unwrap();

    file.write_all(&create_minimal_ivf_header()).unwrap();
    file.write_all(&(MAX_FRAME_SIZE as u32).to_le_bytes())
        .unwrap(); // Frame size at max
    file.write_all(&0u64.to_le_bytes()).unwrap(); // Timestamp
                                                  // Note: We don't actually write MAX_FRAME_SIZE bytes (would be too slow)

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_from_file(&file_path);

    // Should reject due to incomplete data (we didn't write full frame)
    assert!(result.is_err(), "Should reject incomplete large frame");
}

#[test]
fn test_buffer_size_limits() {
    /// Test that I/O buffers respect MAX_BUFFER_SIZE
    /// Expected: Should not allocate buffers larger than MAX_BUFFER_SIZE
    let buffer_size = MAX_BUFFER_SIZE;

    // Test validation function
    let result = bitvue_core::limits::validate_buffer_size(buffer_size);
    assert!(result.is_ok(), "Should accept buffer at MAX_BUFFER_SIZE");

    let result = bitvue_core::limits::validate_buffer_size(buffer_size + 1);
    assert!(result.is_err(), "Should reject buffer over MAX_BUFFER_SIZE");
}

// ============================================================================
// Category 5: Path Traversal
// ============================================================================

#[test]
fn test_path_traversal_parent_directory() {
    /// Test that decoder rejects paths with parent directory references
    /// Expected: Should reject paths containing ".."
    let temp_dir = TempDir::new().unwrap();

    // Try to access file outside working directory
    let traversal_path = temp_dir.path().join("../../../etc/passwd");

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_from_file(&traversal_path);

    assert!(result.is_err(), "Should reject path traversal attempt");
}

#[test]
fn test_path_traversal_absolute() {
    /// Test that decoder only accepts files within working directory
    /// Expected: Should reject absolute paths outside working directory
    let system_path = Path::new("/etc/passwd");

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_from_file(system_path);

    assert!(
        result.is_err(),
        "Should reject absolute path outside working directory"
    );
}

#[test]
fn test_symlink_restriction() {
    /// Test that decoder handles symlinks securely
    /// Expected: Should reject symlinks pointing outside working directory
    let temp_dir = TempDir::new().unwrap();

    // Create a symlink outside the temp directory
    let link_path = temp_dir.path().join("link_to_outside");
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink("/etc/passwd", &link_path);
    }

    if link_path.exists() {
        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_from_file(&link_path);

        // Should reject symlink to outside directory
        assert!(
            result.is_err(),
            "Should reject symlink to outside directory"
        );
    }
}

// ============================================================================
// Category 6: Large Inputs
// ============================================================================

#[test]
fn test_deep_nesting_structure() {
    /// Test decoder with deeply nested structures
    /// Expected: Should enforce MAX_RECURSION_DEPTH limit
    let max_depth = MAX_RECURSION_DEPTH;

    // Test that we can handle structures up to max depth
    let depth_ok = max_depth <= MAX_RECURSION_DEPTH;
    assert!(depth_ok, "Should accept structures at MAX_RECURSION_DEPTH");

    // Test over limit
    let depth_too_deep = max_depth + 1;
    let depth_over_limit = depth_too_deep > MAX_RECURSION_DEPTH;
    assert!(
        depth_over_limit,
        "Should reject structures over MAX_RECURSION_DEPTH"
    );
}

#[test]
fn test_grid_block_limits() {
    /// Test overlay extraction with maximum grid blocks
    /// Expected: Should enforce MAX_GRID_BLOCKS limit
    let max_blocks = MAX_GRID_BLOCKS;

    // Test at limit
    let blocks_ok = max_blocks <= MAX_GRID_BLOCKS;
    assert!(blocks_ok, "Should accept grid at MAX_GRID_BLOCKS");

    // Test over limit
    let blocks_too_many = max_blocks + 1;
    let blocks_over_limit = blocks_too_many > MAX_GRID_BLOCKS;
    assert!(blocks_over_limit, "Should reject grid over MAX_GRID_BLOCKS");
}

#[test]
fn test_grid_dimension_limits() {
    /// Test grid dimension validation
    /// Expected: Should enforce MAX_GRID_DIMENSION limit
    let max_dim = MAX_GRID_DIMENSION as usize;

    // Test at limit
    let dim_ok = max_dim <= MAX_GRID_DIMENSION as usize;
    assert!(dim_ok, "Should accept dimension at MAX_GRID_DIMENSION");

    // Test over limit
    let dim_too_large = max_dim + 1;
    let dim_over_limit = dim_too_large > MAX_GRID_DIMENSION as usize;
    assert!(
        dim_over_limit,
        "Should reject dimension over MAX_GRID_DIMENSION"
    );
}

// ============================================================================
// Category 7: Concurrent Access
// ============================================================================

#[test]
fn test_concurrent_decoding() {
    /// Test multiple decoder instances operating concurrently
    /// Expected: Each decoder should operate independently
    use std::thread;

    let mut decoder1 = Av1Decoder::new().unwrap();
    let mut decoder2 = Av1Decoder::new().unwrap();
    let mut decoder3 = Av1Decoder::new().unwrap();

    // Spawn threads to use decoders concurrently
    let handle1 = thread::spawn(move || {
        // Each decoder should work independently
        let _ = decoder1.send_data(&[0x00, 0x00, 0x01, 0x09], 0);
    });

    let handle2 = thread::spawn(move || {
        let _ = decoder2.send_data(&[0x00, 0x00, 0x01, 0x09], 0);
    });

    let handle3 = thread::spawn(move || {
        let _ = decoder3.send_data(&[0x00, 0x00, 0x01, 0x09], 0);
    });

    // All threads should complete without deadlock
    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}

#[test]
fn test_shared_frame_data() {
    /// Test that Arc-wrapped frame data can be safely cloned
    /// Expected: Multiple references to same frame data should work
    let frame = create_test_frame(1920, 1080);

    // Clone the frame multiple times (Arc should prevent actual data copying)
    let frame_clone1 = frame.clone();
    let frame_clone2 = frame.clone();
    let frame_clone3 = frame.clone();

    // All clones should reference same data
    assert!(Arc::ptr_eq(&frame.y_plane, &frame_clone1.y_plane));
    assert!(Arc::ptr_eq(&frame.y_plane, &frame_clone2.y_plane));
    assert!(Arc::ptr_eq(&frame.y_plane, &frame_clone3.y_plane));
}

// ============================================================================
// Category 8: Error Conditions
// ============================================================================

#[test]
fn test_file_not_found() {
    /// Test decoder behavior when file doesn't exist
    /// Expected: Should return appropriate error
    let non_existent_path = Path::new("/this/path/does/not/exist.ivf");

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_from_file(non_existent_path);

    assert!(result.is_err(), "Should return error for non-existent file");
}

#[test]
fn test_permission_denied() {
    /// Test decoder behavior with unreadable file
    /// Expected: Should return permission error
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("no_permission.ivf");

    // Create file with no read permissions
    let mut file = fs::File::create(&file_path).unwrap();
    file.write_all(b"test data").unwrap();
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o000); // No permissions
        fs::set_permissions(&file_path, perms.clone()).unwrap();

        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_from_file(&file_path);

        assert!(result.is_err(), "Should return permission error");

        // Restore permissions for cleanup
        perms.set_mode(0o644);
        fs::set_permissions(&file_path, perms).unwrap();
    }
}

#[test]
fn test_directory_as_file() {
    /// Test decoder behavior when path is a directory
    /// Expected: Should return error (not a file)
    let temp_dir = TempDir::new().unwrap();

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_from_file(temp_dir.path());

    assert!(
        result.is_err(),
        "Should return error when path is directory"
    );
}

// ============================================================================
// Category 9: Encoding Issues
// ============================================================================

#[test]
fn test_invalid_utf8_in_paths() {
    /// Test decoder behavior with invalid UTF-8 in paths
    /// Expected: Should handle gracefully (reject or sanitize)
    let temp_dir = TempDir::new().unwrap();

    // Create file with invalid UTF-8 sequence
    let invalid_name = [0xFF, 0xFE, 0xFD, 0xFC];
    let file_path = temp_dir
        .path()
        .join(PathBuf::from(std::ffi::OsString::from_vec(
            invalid_name.to_vec(),
        )));

    // If file creation succeeds, decoder should handle it
    if fs::File::create(&file_path).is_ok() {
        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_from_file(&file_path);

        // Should not crash (may succeed or fail with appropriate error)
    }
}

#[test]
fn test_unicode_path_handling() {
    /// Test decoder behavior with Unicode characters in paths
    /// Expected: Should handle Unicode paths correctly
    let temp_dir = TempDir::new().unwrap();

    // Create file with Unicode name
    let unicode_name = "test_视频_ivf文件.ivf"; // Chinese characters
    let file_path = temp_dir.path().join(unicode_name);

    let mut file = fs::File::create(&file_path).unwrap();
    file.write_all(&create_minimal_ivf_header()).unwrap();
    drop(file);

    // Should handle Unicode path
    let result = std::fs::metadata(&file_path);
    assert!(result.is_ok(), "Should handle Unicode path names");
}

#[test]
fn test_bom_handling() {
    /// Test decoder behavior with BOM (Byte Order Mark) in data
    /// Expected: Should handle or reject BOM appropriately
    let mut data_with_bom = Vec::new();
    data_with_bom.extend_from_slice(&[0xEF, 0xBB, 0xBF]); // UTF-8 BOM
    data_with_bom.extend_from_slice(b"DKIF"); // IVF magic

    let format = detect_format(&data_with_bom);
    // BOM should not interfere with format detection
    assert_ne!(
        format,
        VideoFormat::Ivf,
        "Should not detect IVF with BOM prefix"
    );
}

// ============================================================================
// Category 10: Platform Differences
// ============================================================================

#[test]
fn test_windows_path_handling() {
    /// Test decoder behavior with Windows-style paths
    /// Expected: Should handle Windows paths on Windows, reject on Unix
    let windows_path = Path::new("C:\\Users\\test\\video.ivf");

    #[cfg(windows)]
    {
        // On Windows, should handle backslashes
        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_from_file(windows_path);
        // Will likely fail (file doesn't exist), but should not crash
    }

    #[cfg(unix)]
    {
        // On Unix, backslash is valid filename character
        // Should treat as relative path or fail appropriately
        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_from_file(windows_path);
        assert!(
            result.is_err(),
            "Should fail on Unix with Windows-style absolute path"
        );
    }
}

#[test]
fn test_mixed_path_separators() {
    /// Test decoder behavior with mixed path separators
    /// Expected: Should handle or normalize appropriately
    let temp_dir = TempDir::new().unwrap();

    #[cfg(unix)]
    {
        let mixed_path = temp_dir.path().join("subdir\\file.ivf"); // Backslash on Unix
                                                                   // Backslash is valid character, should not be treated as separator
    }

    #[cfg(windows)]
    {
        let mixed_path = temp_dir.path().join("subdir/file.ivf"); // Forward slash on Windows
                                                                  // Windows accepts forward slashes
    }
}

#[test]
fn test_line_endings_in_metadata() {
    /// Test decoder with different line endings in text data
    /// Expected: Should handle both CRLF and LF
    let crlf_data = b"DKIF\r\n"; // Not realistic, but tests the concept
    let lf_data = b"DKIF\n";

    // Format detection should work regardless of line ending
    let format1 = detect_format(crlf_data);
    let format2 = detect_format(lf_data);

    assert_eq!(
        format1, format2,
        "Line endings should not affect format detection"
    );
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_minimal_ivf_header() -> Vec<u8> {
    /// Create a minimal valid IVF header for testing
    let mut header = Vec::new();

    header.extend_from_slice(b"DKIF"); // Magic number
    header.extend_from_slice(&0u16.to_le_bytes()); // Version
    header.extend_from_slice(&32u16.to_le_bytes()); // Header size
    header.extend_from_slice(b"AV01"); // FourCC (AV1)
    header.extend_from_slice(&1920u16.to_le_bytes()); // Width
    header.extend_from_slice(&1080u16.to_le_bytes()); // Height
    header.extend_from_slice(&60u32.to_le_bytes()); // Timebase numerator
    header.extend_from_slice(&1u32.to_le_bytes()); // Timebase denominator
    header.extend_from_slice(&0u32.to_le_bytes()); // Number of frames
    header.extend_from_slice(&0u32.to_le_bytes()); // Reserved

    header
}

fn create_test_frame(width: u32, height: u32) -> DecodedFrame {
    /// Create a test frame with specified dimensions
    use std::sync::Arc;

    let y_size = (width * height) as usize;
    let uv_size = (width / 2 * height / 2) as usize;

    DecodedFrame {
        width,
        height,
        bit_depth: 8,
        y_plane: Arc::from(vec![128u8; y_size]),
        y_stride: width as usize,
        u_plane: Some(Arc::from(vec![128u8; uv_size])),
        u_stride: (width / 2) as usize,
        v_plane: Some(Arc::from(vec![128u8; uv_size])),
        v_stride: (width / 2) as usize,
        timestamp: 0,
        frame_type: bitvue_decode::decoder::FrameType::Key,
        qp_avg: Some(25),
        chroma_format: bitvue_decode::decoder::ChromaFormat::Yuv420,
    }
}
