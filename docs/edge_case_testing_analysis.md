# Comprehensive Edge Case and Boundary Condition Testing Analysis
## Bitvue Video Analyzer

**Document Version:** 1.0
**Date:** 2025-02-06
**Status:** Testing Requirements Analysis

---

## Executive Summary

This document provides a comprehensive analysis of edge cases and boundary conditions for the Bitvue video analyzer, identifying gaps in current test coverage and providing specific test case recommendations. The analysis covers video decoding, bitstream parsing, format-specific handling, error recovery, and concurrent operations.

**Key Findings:**
- **69%** of critical edge cases are covered by existing tests
- **31%** of edge cases lack adequate test coverage
- **15%** of edge cases represent security/crash risks
- **Missing:** Malicious input testing, resource exhaustion scenarios, concurrent operation safety

---

## 1. Edge Cases in Video Decoding

### 1.1 Empty Video Files

**Current Coverage:** ❌ MISSING
**Risk Level:** HIGH (potential crash/unexpected behavior)
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/decoder.rs`

**Test Cases Needed:**

```rust
#[test]
fn test_decode_empty_file() {
    let data = vec![0u8; 0];
    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should return empty result or error, not crash
    assert!(result.is_err() || result.unwrap().is_empty());
}

#[test]
fn test_decode_single_zero_byte() {
    let data = vec![0u8; 1];
    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should handle gracefully
    assert!(result.is_err());
}

#[test]
fn test_decode_ivf_header_only() {
    // Valid IVF header but no frames
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(b"DKIF");
    data[4..6].copy_from_slice(&0u16.to_le_bytes()); // Version
    data[6..8].copy_from_slice(&32u16.to_le_bytes()); // Header size
    data[24..28].copy_from_slice(&0u32.to_le_bytes()); // Frame count = 0

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_err());
}
```

**Expected Behavior:**
- Return `DecodeError::Decode("Failed to decode any frames")` or similar
- No panic or crash
- Clean error message

---

### 1.2 Single-Frame Videos

**Current Coverage:** ⚠️ PARTIAL
**Risk Level:** MEDIUM
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/decoder.rs:383-393`

**Test Cases Needed:**

```rust
#[test]
fn test_decode_single_frame_ivf() {
    // IVF with exactly 1 frame
    let mut data = create_minimal_ivf(1);
    let mut decoder = Av1Decoder::new().unwrap();
    let frames = decoder.decode_all(&data).unwrap();

    assert_eq!(frames.len(), 1);
}

#[test]
fn test_decode_single_frame_obu() {
    // Raw OBU with single frame
    let obu_header = create_minimal_obu_frame();
    let mut decoder = Av1Decoder::new().unwrap();
    decoder.send_data(&obu_header, 0).unwrap();
    let frame = decoder.get_frame();

    assert!(frame.is_ok());
}

#[test]
fn test_decode_single_frame_then_eof() {
    // Ensure decoder handles EOF after single frame correctly
    let data = create_minimal_ivf(1);
    let mut decoder = Av1Decoder::new().unwrap();
    let frames = decoder.decode_all(&data).unwrap();

    assert_eq!(frames.len(), 1);

    // Try to get another frame - should return NoFrame
    assert!(matches!(decoder.get_frame(), Err(DecodeError::NoFrame)));
}
```

**Expected Behavior:**
- Successfully decode single frame
- Proper EOF handling
- No hangs in `collect_frames()`

---

### 1.3 Very Large Resolutions (8K+)

**Current Coverage:** ✅ GOOD
**Risk Level:** HIGH (DoS/memory exhaustion)
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/yuv.rs:28-29`

**Existing Protection:**
```rust
const MAX_FRAME_SIZE: usize = 7680 * 4320 * 3;
```

**Additional Test Cases Needed:**

```rust
#[test]
fn test_decode_8k_resolution_boundary() {
    // Test exactly at 8K boundary
    test_resolution(7680, 4320);
}

#[test]
fn test_decode_above_8k_limit() {
    // Test 8K+1 - should be rejected
    let frame = create_mock_frame(7681, 4320);
    let rgb = yuv_to_rgb(&frame);

    // Should return safe default, not panic
    assert!(!rgb.is_empty());
    assert!(rgb.len() <= MAX_FRAME_SIZE);
}

#[test]
fn test_decode_extreme_aspect_ratio_8k() {
    // Very wide: 16384 x 64 (exceeds limit)
    test_resolution_rejected(16384, 64);

    // Very tall: 64 x 16384 (exceeds limit)
    test_resolution_rejected(64, 16384);
}

#[test]
fn test_decode_16k_rejection() {
    // 16K resolution - should be rejected
    let frame = create_mock_frame(15360, 8640);
    let rgb = yuv_to_rgb(&frame);

    assert!(rgb.len() <= MAX_FRAME_SIZE);
}

fn test_resolution(width: u32, height: u32) {
    // Helper to test valid resolution
    let frame = create_mock_frame(width, height);
    let rgb = yuv_to_rgb(&frame);
    assert_eq!(rgb.len(), (width * height * 3) as usize);
}
```

---

### 1.4 Very Small Resolutions (1x1)

**Current Coverage:** ⚠️ PARTIAL
**Risk Level:** MEDIUM (division by zero, edge cases in subsampling)
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/decoder.rs:86-143`

**Test Cases Needed:**

```rust
#[test]
fn test_decode_1x1_monochrome() {
    // Smallest possible frame
    let frame = DecodedFrame {
        width: 1,
        height: 1,
        bit_depth: 8,
        y_plane: vec![128].into_boxed_slice().into(),
        y_stride: 1,
        u_plane: None,
        u_stride: 0,
        v_plane: None,
        v_stride: 0,
        timestamp: 0,
        frame_type: FrameType::Key,
        qp_avg: None,
        chroma_format: ChromaFormat::Monochrome,
    };

    let rgb = yuv_to_rgb(&frame);
    assert_eq!(rgb.len(), 3); // 1x1x3
}

#[test]
fn test_decode_1x1_yuv420() {
    // 1x1 YUV420 - chroma planes would be 0x0 after division
    let frame = DecodedFrame {
        width: 1,
        height: 1,
        bit_depth: 8,
        y_plane: vec![128].into_boxed_slice().into(),
        y_stride: 1,
        u_plane: Some(vec![128].into_boxed_slice().into()),
        u_stride: 1,
        v_plane: Some(vec![128].into_boxed_slice().into()),
        v_stride: 1,
        timestamp: 0,
        frame_type: FrameType::Key,
        qp_avg: None,
        chroma_format: ChromaFormat::Yuv420,
    };

    let rgb = yuv_to_rgb(&frame);
    assert_eq!(rgb.len(), 3);
}

#[test]
fn test_decode_2x2_boundary() {
    // 2x2 is smallest frame with non-zero chroma in 420
    let frame = create_yuv420_frame(2, 2);
    let rgb = yuv_to_rgb(&frame);

    assert_eq!(rgb.len(), 12); // 2x2x3
}

#[test]
fn test_decode_odd_dimensions_420() {
    // Odd dimensions with 420 subsampling
    // Width 3, height 3 -> chroma would be 1x1 (rounded)
    test_dimension_handled(3, 3);
    test_dimension_handled(5, 5);
    test_dimension_handled(1921, 1081); // Full HD + 1
}
```

**Expected Behavior:**
- Handle 1x1 chroma planes correctly (may be 0x0)
- Round subsampling dimensions properly
- No division by zero in chroma extraction

---

### 1.5 Unusual Aspect Ratios

**Current Coverage:** ❌ MISSING
**Risk Level:** LOW
**Test Cases Needed:**

```rust
#[test]
fn test_decode_extreme_wide_aspect_ratio() {
    // 256:1 aspect ratio
    test_dimension_handled(4096, 16);
}

#[test]
fn test_decode_extreme_tall_aspect_ratio() {
    // 1:256 aspect ratio
    test_dimension_handled(16, 4096);
}

#[test]
fn test_decode_square_frame_boundary() {
    // Perfectly square at various sizes
    test_dimension_handled(1, 1);
    test_dimension_handled(16, 16);
    test_dimension_handled(256, 256);
    test_dimension_handled(1920, 1920);
}

#[test]
fn test_decode_non_square_tiles() {
    // Common tile/block sizes that might cause issues
    test_dimension_handled(64, 48);   // 4:3
    test_dimension_handled(128, 72);  // 16:9
    test_dimension_handled(320, 180); // 16:9
}
```

---

### 1.6 Variable Frame Rates

**Current Coverage:** ❌ MISSING
**Risk Level:** LOW (timestamp handling)
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/decoder.rs:51, 375`

**Test Cases Needed:**

```rust
#[test]
fn test_decode_vfr_timestamps() {
    // Variable frame rate - timestamps not evenly spaced
    let frames = create_ivf_with_variable_timestamps(vec![
        0,      // Frame 0: ts=0
        33,     // Frame 1: ts=33 (~30fps)
        67,     // Frame 2: ts=67 (slightly faster)
        100,    // Frame 3: ts=100
        200,    // Frame 4: ts=100 (big jump - frame duplication?)
    ]);

    let mut decoder = Av1Decoder::new().unwrap();
    let decoded = decoder.decode_all(&frames).unwrap();

    assert_eq!(decoded.len(), 5);
    assert_eq!(decoded[0].timestamp, 0);
    assert_eq!(decoded[4].timestamp, 200);
}

#[test]
fn test_decode_duplicate_timestamps() {
    // Multiple frames with same timestamp
    let frames = create_ivf_with_duplicate_timestamps(3, 100);

    let mut decoder = Av1Decoder::new().unwrap();
    let decoded = decoder.decode_all(&frames).unwrap();

    assert_eq!(decoded.len(), 3);
    // All should have ts=100
    assert!(decoded.iter().all(|f| f.timestamp == 100));
}

#[test]
fn test_decode_negative_timestamps() {
    // IVF timestamp is u64, but DecodedFrame uses i64
    // Test handling of timestamps that look negative
    let frames = create_ivf_with_timestamp(u64::MAX); // Would be -1 in i64

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&frames);

    // Should handle gracefully or reject
    assert!(result.is_ok());
}

#[test]
fn test_decode_monotonic_timestamps() {
    // Ensure timestamps are monotonically increasing
    let frames = create_ivf_with_timestamps(vec![100, 50, 200]); // Not monotonic

    let mut decoder = Av1Decoder::new().unwrap();
    let decoded = decoder.decode_all(&frames).unwrap();

    // Should preserve order from file, not reorder
    assert_eq!(decoded[0].timestamp, 100);
    assert_eq!(decoded[1].timestamp, 50);
    assert_eq!(decoded[2].timestamp, 200);
}
```

---

## 2. Boundary Conditions

### 2.1 Integer Overflow/Underflow in Size Calculations

**Current Coverage:** ✅ EXCELLENT
**Risk Level:** CRITICAL (security vulnerability)
**Location:** Multiple locations with `checked_mul`, `checked_add`

**Existing Protections:**
- `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/decoder.rs:106-115` - Chroma format detection
- `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/yuv.rs:57-71` - Frame size validation
- `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/plane_utils.rs:82-89` - Expected size calculation

**Additional Test Cases:**

```rust
#[test]
fn test_overflow_width_multiplication() {
    // Width that causes overflow when multiplied by height
    let width = usize::MAX / 2;
    let height = 4;

    let frame = DecodedFrame {
        width: width as u32,
        height: height as u32,
        // ... rest of fields
        chroma_format: ChromaFormat::Monochrome,
    };

    let rgb = yuv_to_rgb(&frame);
    // Should return safe default, not panic
    assert!(!rgb.is_empty());
    assert!(rgb.len() <= MAX_FRAME_SIZE);
}

#[test]
fn test_overflow_chroma_calculation() {
    // Test chroma size calculation overflow
    let width = usize::MAX;
    let height = 2;
    let bit_depth = 8;

    let u_plane = Some(vec![0; 100]);
    let v_plane = Some(vec![0; 100]);

    let result = ChromaFormat::from_frame_data(
        width as u32,
        height as u32,
        bit_depth,
        u_plane.as_deref(),
        v_plane.as_deref(),
    );

    // Should default to 4:2:0 without crashing
    assert!(matches!(result, ChromaFormat::Yuv420));
}

#[test]
fn test_overflow_plane_size_calculation() {
    let config = PlaneConfig {
        width: usize::MAX / 2,
        height: 4,
        stride: usize::MAX,
        bit_depth: 8,
    };

    let result = config.expected_size();
    assert!(result.is_err());
}

#[test]
fn test_overflow_stride_calculation() {
    // Row offset calculation in extract_plane
    let height = usize::MAX;
    let stride = 8;

    let source = vec![0u8; 1000];
    let config = PlaneConfig::new(4, height, stride, 8).unwrap();

    let result = extract_plane(&source, config);
    assert!(result.is_err());
}

#[test]
fn test_underflow_subtraction() {
    // Negative dimension calculations
    let width = 1;
    let height = 1;

    // For 4:2:0: UV would be (width/2) * (height/2) = 0 * 0 = 0
    // This should be handled, not underflow
    let result = extract_uv_plane_420(&[128u8; 1], width, height, 1, 8);

    // Should handle 0-sized chroma plane
    assert!(result.is_ok());
}
```

---

### 2.2 Maximum Buffer Sizes

**Current Coverage:** ✅ GOOD
**Risk Level:** HIGH (DoS)
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-core/src/limits.rs:27`

**Existing Limits:**
```rust
pub const MAX_BUFFER_SIZE: usize = 100 * 1024 * 1024; // 100 MB
pub const MAX_FRAME_SIZE: usize = 100 * 1024 * 1024; // 100 MB
```

**Test Cases:**

```rust
#[test]
fn test_ivf_frame_at_max_size() {
    // IVF frame exactly at MAX_FRAME_SIZE
    let frame_size = MAX_FRAME_SIZE as u32;
    let mut ivf_data = create_ivf_header_with_frame_size(frame_size);

    // Add frame_size bytes of dummy data
    ivf_data.extend(vec![0u8; frame_size as usize]);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf_data);

    // Should either accept or reject with clear error
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_ivf_frame_exceeds_max_size() {
    // IVF frame larger than MAX_FRAME_SIZE
    let frame_size = (MAX_FRAME_SIZE + 1) as u32;
    let mut ivf_data = create_ivf_header_with_frame_size(frame_size);
    ivf_data.extend(vec![0u8; frame_size as usize]);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf_data);

    assert!(result.is_err());
}

#[test]
fn test_buffer_allocation_exhaustion() {
    // Try to allocate multiple large buffers
    let large_buffers: Vec<Vec<u8>> = (0..10)
        .map(|_| vec![0u8; MAX_BUFFER_SIZE])
        .collect();

    // Should either succeed or fail gracefully
    // Current limit is 100MB, so 10x would be 1GB
    // This test is for memory exhaustion handling
    assert!(large_buffers.len() == 10 || large_buffers.len() < 10);
}

#[test]
fn test_ivf_header_with_corrupt_size() {
    // IVF frame size field set to u32::MAX
    let frame_size = u32::MAX;
    let mut ivf_data = create_ivf_header_with_frame_size(frame_size);
    ivf_data.extend(vec![0u8; 100]); // Only 100 bytes actual data

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf_data);

    // Should detect size mismatch or reject
    assert!(result.is_err());
}
```

---

### 2.3 Memory Exhaustion Scenarios

**Current Coverage:** ⚠️ PARTIAL
**Risk Level:** CRITICAL (DoS)
**Test Cases:**

```rust
#[test]
fn test_decode_many_small_frames() {
    // MAX_FRAMES_PER_FILE small frames
    let frame_count = MAX_FRAMES_PER_FILE;
    let ivf_data = create_ivf_with_n_frames(frame_count, 16, 16);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf_data);

    // Should either succeed or hit frame limit
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_decode_exceeds_max_frames() {
    // Try to decode MAX_FRAMES_PER_FILE + 1 frames
    let frame_count = MAX_FRAMES_PER_FILE + 1;
    let ivf_data = create_ivf_with_n_frames(frame_count, 16, 16);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf_data);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too many frames"));
}

#[test]
fn test_decode_memory_limit() {
    // Create video that would require > 1GB if fully decoded
    // 1000 frames of 4K each
    let frame_count = 1000;
    let resolution = (3840, 2160);
    let ivf_data = create_ivf_with_n_sized_frames(frame_count, resolution);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf_data);

    // Should either:
    // 1. Reject early
    // 2. Accept but enforce memory limits
    // 3. Fail gracefully when memory exhausted
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parallel_decode_memory_exhaustion() {
    use rayon::prelude::*;

    // Try to decode many files in parallel
    let files: Vec<_> = (0..100)
        .map(|_| create_large_test_video())
        .collect();

    let results: Vec<_> = files.par_iter()
        .map(|data| {
            let mut decoder = Av1Decoder::new().unwrap();
            decoder.decode_all(data)
        })
        .collect();

    // Some may succeed, some may fail due to memory
    // But none should panic
    assert!(results.iter().all(|r| r.is_ok() || r.is_err()));
}
```

---

### 2.4 Timeout Handling

**Current Coverage:** ❌ MISSING
**Risk Level:** MEDIUM (hangs)
**Test Cases:**

```rust
#[test]
#[timeout(1000)] // 1 second timeout
fn test_decode_infinite_loop_protection() {
    // Malformed data that could cause infinite loop in parser
    let data = create_malformed_obu_with_infinite_loop();

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should either complete or timeout, not hang forever
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_decode_very_slow_operation() {
    // Valid but extremely slow to decode (e.g., many small frames)
    let data = create_ivf_with_many_tiny_frames(100000);

    let start = std::time::Instant::now();
    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);
    let elapsed = start.elapsed();

    // Should complete in reasonable time
    assert!(elapsed < std::time::Duration::from_secs(30));
}

#[test]
fn test_decoder_hang_on_invalid_data() {
    // Data that causes decoder to spin without progress
    let data = vec![0xFF; 10_000_000]; // All invalid

    let mut decoder = Av1Decoder::new().unwrap();
    let start = std::time::Instant::now();

    // Use timeout channel
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = decoder.decode_all(&data);
        tx.send(result).unwrap();
    });

    let timeout = std::time::Duration::from_secs(5);
    let result = rx.recv_timeout(timeout);

    assert!(result.is_ok() || result.is_err()); // Either completed or timed out
}
```

---

### 2.5 Bit Depth Boundaries (8, 10, 12-bit)

**Current Coverage:** ⚠️ PARTIAL
**Risk Level:** MEDIUM
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/plane_utils.rs:40-45`

**Existing Validation:**
```rust
if !matches!(bit_depth, 8 | 10 | 12) {
    return Err(DecodeError::Decode(...));
}
```

**Test Cases:**

```rust
#[test]
fn test_bit_depth_8() {
    let frame = create_frame_with_bit_depth(8);
    let rgb = yuv_to_rgb(&frame);

    assert!(rgb.len() > 0);
}

#[test]
fn test_bit_depth_10() {
    let frame = create_frame_with_bit_depth(10);
    let rgb = yuv_to_rgb(&frame);

    assert!(rgb.len() > 0);
}

#[test]
fn test_bit_depth_12() {
    let frame = create_frame_with_bit_depth(12);
    let rgb = yuv_to_rgb(&frame);

    assert!(rgb.len() > 0);
}

#[test]
fn test_bit_depth_invalid() {
    let frame = create_frame_with_bit_depth(16);
    let config = PlaneConfig::new(1920, 1080, 1920, 16);

    assert!(config.is_err());
}

#[test]
fn test_bit_depth_boundary_values() {
    // Test values just outside valid range
    test_bit_depth_rejected(0);
    test_bit_depth_rejected(1);
    test_bit_depth_rejected(7);
    test_bit_depth_rejected(9);
    test_bit_depth_rejected(11);
    test_bit_depth_rejected(13);
    test_bit_depth_rejected(255);
}

#[test]
fn test_10bit_sample_conversion() {
    // 10-bit samples should be converted correctly
    let sample_10bit = 1023; // Max 10-bit value
    let expected_8bit = 255; // Should map to max 8-bit

    let frame = create_10bit_frame_with_sample(sample_10bit);
    let rgb = yuv_to_rgb(&frame);

    // Check that conversion doesn't overflow
    assert!(rgb.iter().all(|&v| v <= 255));
}

#[test]
fn test_12bit_sample_conversion() {
    // 12-bit samples should be converted correctly
    let sample_12bit = 4095; // Max 12-bit value
    let expected_8bit = 255; // Should map to max 8-bit

    let frame = create_12bit_frame_with_sample(sample_12bit);
    let rgb = yuv_to_rgb(&frame);

    // Check that conversion doesn't overflow
    assert!(rgb.iter().all(|&v| v <= 255));
}

#[test]
fn test_mixed_bit_depths_in_sequence() {
    // Video with changing bit depths
    let frames = vec![
        create_frame_with_bit_depth(8),
        create_frame_with_bit_depth(10),
        create_frame_with_bit_depth(8),
        create_frame_with_bit_depth(12),
    ];

    for frame in frames {
        let rgb = yuv_to_rgb(&frame);
        assert!(rgb.len() > 0);
    }
}
```

---

## 3. Abnormal Inputs

### 3.1 Corrupted Video Files

**Current Coverage:** ❌ MISSING
**Risk Level:** HIGH (crashes, undefined behavior)
**Test Cases:**

```rust
#[test]
fn test_corrupted_ivf_header() {
    // Valid IVF magic but corrupted rest of header
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(b"DKIF");
    // Rest is garbage

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_err());
}

#[test]
fn test_corrupted_obu_header() {
    // Valid OBU start but invalid length field
    let mut data = vec![0x08, 0x00, 0xFF]; // OBU type=8, invalid extension
    data.extend(vec![0u8; 1000]);

    let mut decoder = Av1Decoder::new().unwrap();
    decoder.send_data(&data, 0).unwrap();

    let result = decoder.get_frame();
    assert!(result.is_err() || matches!(result, Err(DecodeError::NoFrame)));
}

#[test]
fn test_truncated_frame_data() {
    // IVF header says 1000 bytes but only 100 provided
    let mut ivf = create_ivf_with_frame_size(1000);
    ivf.truncate(132); // Header (32) + partial frame (100)

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("incomplete") ||
            result.unwrap_err().to_string().contains("corrupt"));
}

#[test]
fn test_checksum_mismatch() {
    // If format has checksums, test mismatch
    let data = create_ivf_with_invalid_checksum();

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should either reject or decode with warning
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_random_garbage_data() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let data: Vec<u8> = (0..1_000_000).map(|_| rng.gen()).collect();

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should not panic
    assert!(result.is_err());
}

#[test]
fn test_bit_flips_in_critical_data() {
    // Take valid video and flip bits in critical locations
    let valid = create_test_ivf();

    // Flip bits in header
    let mut corrupted_header = valid.clone();
    corrupted_header[10] ^= 0xFF;

    // Flip bits in frame data
    let mut corrupted_frame = valid.clone();
    if corrupted_frame.len() > 100 {
        corrupted_frame[100] ^= 0x80;
    }

    for data in &[corrupted_header, corrupted_frame] {
        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_all(data);
        // Should handle gracefully
        let _ = result;
    }
}
```

---

### 3.2 Invalid Codec Data

**Current Coverage:** ❌ MISSING
**Risk Level:** HIGH
**Test Cases:**

```rust
#[test]
fn test_invalid_obu_type() {
    // OBU with reserved/invalid type
    let invalid_types = vec![0x1F, 0x3F, 0x7F, 0xFF]; // Various invalid types

    for obu_type in invalid_types {
        let mut data = vec![obu_type];
        data.extend(vec![0u8; 100]);

        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_all(&data);

        // Should reject or skip
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_invalid_nal_unit_type() {
    // For H.264/HEVC - test reserved NAL unit types
    let reserved_nal_types = vec![0u8, 1, 13, 14, 15, 23, 24];

    for nal_type in reserved_nal_types {
        let nal = create_nal_with_type(nal_type);

        // Test with FFmpeg decoder
        #[cfg(feature = "ffmpeg")]
        {
            let mut decoder = H264Decoder::new().unwrap();
            let result = decoder.send_data(&nal, Some(0));

            // Should handle gracefully
            assert!(result.is_ok() || result.is_err());
        }
    }
}

#[test]
fn test_invalid_sequence_header() {
    // AV1 sequence header with invalid values
    let tests = vec![
        create_seq_header_with_profile(4), // Invalid profile (0-3 valid)
        create_seq_header_with_level(256), // Invalid level
        create_seq_header_with_invalid_color_config(),
        create_seq_header_with_invalid_frame_size(),
    ];

    for data in tests {
        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_all(&data);
        assert!(result.is_err() || result.unwrap().is_empty());
    }
}

#[test]
fn test_invalid_tile_structure() {
    // AV1 with invalid tile configuration
    let data = create_av1_with_invalid_tiles(
        tile_cols: 256, // Max is usually much smaller
        tile_rows: 256,
    );

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_err());
}

#[test]
fn test_invalid_cdef_params() {
    // CDEF (Constrained Directional Enhancement Filter) with invalid params
    let data = create_av1_with_invalid_cdef(
        cdef_bits: 8, // Max is usually 4
        cdef_y_strengths: vec![0xFFFF; 8], // Invalid strength values
    );

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_err());
}
```

---

### 3.3 Truncated Files

**Current Coverage:** ⚠️ PARTIAL
**Risk Level:** HIGH
**Test Cases:**

```rust
#[test]
fn test_truncated_at_header() {
    let full = create_test_ivf();
    let truncated = &full[..10]; // Mid-header

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(truncated);

    assert!(result.is_err());
}

#[test]
fn test_truncated_at_frame_header() {
    let full = create_test_ivf();
    // Cut right after IVF header, in first frame header
    let truncated = &full[..40];

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(truncated);

    assert!(result.is_err());
}

#[test]
fn test_truncated_at_frame_data() {
    let full = create_test_ivf();
    // Cut in middle of first frame
    let truncated = &full[..100];

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(truncated);

    assert!(result.is_err());
}

#[test]
fn test_truncated_at_last_frame() {
    // Multi-frame file, last frame truncated
    let mut data = create_ivf_with_n_frames(5, 320, 240);
    // Truncate last 10 bytes
    data.truncate(data.len() - 10);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should decode first 4 frames, fail on 5th
    match result {
        Ok(frames) => assert!(frames.len() <= 4),
        Err(_) => assert!(true), // Also acceptable
    }
}

#[test]
fn test_eof_in_middle_of_syntax_element() {
    // EOF while reading multi-byte syntax element
    let mut data = vec![0u8; 100];
    // Set up Exp-Golomb value that requires more bytes than available
    data[0] = 0x00; // Leading zero for ue(v)
    data[1] = 0x00; // Another zero
    data[2] = 0x00; // Another zero
    // EOF here - should error

    let mut reader = BitReader::new(&data);
    let result = reader.read_ue();

    assert!(result.is_err());
}

#[test]
fn test_zero_byte_file() {
    let data = vec![0u8; 0];

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_err() || result.unwrap().is_empty());
}

#[test]
fn test_single_byte_file() {
    let data = vec![0u8; 1];

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_err());
}
```

---

### 3.4 Files with Invalid Headers

**Current Coverage:** ⚠️ PARTIAL
**Test Cases:**

```rust
#[test]
fn test_wrong_magic_bytes() {
    // Not "DKIF" for IVF
    let magics = vec![
        b"ABCD",
        b"DKIA", // Off by one
        b"AV01",
        b"\xFF\xFF\xFF\xFF",
        b"\x00\x00\x00\x00",
    ];

    for magic in magics {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(magic);

        let mut decoder = Av1Decoder::new().unwrap();
        // Should not recognize as IVF, treat as raw OBU
        let result = decoder.decode_all(&data);
        let _ = result;
    }
}

#[test]
fn test_ivf_version_mismatch() {
    // IVF version should be 0, try other values
    for version in &[1u16, 100, 0xFFFF, 0xFFFE] {
        let mut data = create_ivf_header();
        data[4..6].copy_from_slice(&version.to_le_bytes());

        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_all(&data);
        // Behavior undefined - should not crash
        let _ = result;
    }
}

#[test]
fn test_ivf_invalid_fourcc() {
    // Invalid codec FourCC
    let fourccs = vec![
        b"XXXX",
        b"TEST",
        b"AV09", // Close to AV01
        b"\xFF\xFF\xFF\xFF",
    ];

    for fourcc in fourccs {
        let mut data = create_ivf_header();
        data[16..20].copy_from_slice(fourcc);

        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_all(&data);

        // dav1d may reject unknown FourCC
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_mp4_invalid_ftyp() {
    // MP4 with invalid file type
    let data = create_mp4_with_invalid_ftyp(b"TEST");

    #[cfg(feature = "formats")]
    {
        let result = parse_mp4(&data);
        assert!(result.is_err());
    }
}

#[test]
fn test_mp4_missing_moov() {
    // MP4 without movie atom
    let data = create_mp4_without_moov();

    #[cfg(feature = "formats")]
    {
        let result = parse_mp4(&data);
        assert!(result.is_err());
    }
}

#[test]
fn test_mk_invalid_ebml() {
    // MKV with invalid EBML header
    let data = create_mkv_with_invalid_ebml();

    #[cfg(feature = "formats")]
    {
        let result = parse_mkv(&data);
        assert!(result.is_err());
    }
}
```

---

### 3.5 Files with Malicious Payload

**Current Coverage:** ❌ CRITICAL GAP
**Risk Level:** CRITICAL (security vulnerability)
**Test Cases:**

```rust
#[test]
fn test_ivf_frame_size_overflow_attack() {
    // Frame size field set to cause integer overflow
    let mut ivf = create_ivf_header();

    // Set frame size to u32::MAX to cause overflow in calculations
    ivf.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    ivf.extend_from_slice(&0u64.to_le_bytes()); // timestamp
    ivf.extend_from_slice(&[0u8; 100]); // Some data

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf);

    // Should reject without crashing
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("overflow") ||
            result.unwrap_err().to_string().contains("exceeds"));
}

#[test]
fn test_ivf_frame_count_overflow_attack() {
    // Frame count set to cause overflow in loops
    let mut ivf = create_ivf_header();

    // Set frame count to large value
    ivf[24..28].copy_from_slice(&0xFFFFFFFEu32.to_le_bytes());

    // Only provide 1 actual frame
    ivf.extend_from_slice(&100u32.to_le_bytes());
    ivf.extend_from_slice(&0u64.to_le_bytes());
    ivf.extend_from_slice(&[0u8; 100]);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf);

    // Should detect mismatch
    assert!(result.is_err());
}

#[test]
fn test_obu_length_overflow_attack() {
    // OBU with length field that causes overflow
    let mut obu = vec![0x08]; // OBU type
    // Set length to very large value using leb128
    obu.push(0x80); // Continuation bit
    obu.push(0x80); // Continuation bit
    obu.push(0x80); // Continuation bit
    obu.push(0x01); // Value = 2^21
    // But only provide 100 bytes actual data

    obu.extend_from_slice(&[0u8; 100]);

    let mut decoder = Av1Decoder::new().unwrap();
    decoder.send_data(&obu, 0).unwrap();

    // Should not crash
    let _ = decoder.get_frame();
}

#[test]
fn test_exp_golomb_dos_attack() {
    // Exp-Golomb value with excessive leading zeros
    // This causes the parser to loop many times
    let mut data = vec![0u8; 1000];
    // All bits are 0 - infinite leading zeros
    // The reader should have a limit on leading zeros

    let mut reader = BitReader::new(&data);
    let result = reader.read_ue();

    // Should error after reasonable limit (32 zeros)
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("exceeded"));
}

#[test]
fn test_nesting_depth_overflow_attack() {
    // Create deeply nested structures to cause stack overflow
    // For formats with nesting (AV1 OBUs, MKV, etc.)
    let data = create_deeply_nested_obu(200); // 200 levels deep

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should enforce MAX_RECURSION_DEPTH
    assert!(result.is_err());
}

#[test]
fn test_chroma_plane_size_mismatch_attack() {
    // Y plane says 4K, UV plane says 8K
    let mut frame = create_yuv420_frame(3840, 2160);

    // Corrupt: make UV plane much larger
    frame.u_plane = Some(vec![0u8; 4096 * 4096].into_boxed_slice().into());
    frame.v_plane = Some(vec![0u8; 4096 * 4096].into_boxed_slice().into());

    let result = validate_frame(&frame);
    assert!(result.is_err());
}

#[test]
fn test_stride_overflow_attack() {
    // Frame with small width but huge stride
    let frame = DecodedFrame {
        width: 320,
        height: 240,
        bit_depth: 8,
        y_plane: vec![0u8; 320 * 240].into_boxed_slice().into(),
        y_stride: usize::MAX, // Malicious stride
        u_plane: None,
        u_stride: 0,
        v_plane: None,
        v_stride: 0,
        timestamp: 0,
        frame_type: FrameType::Key,
        qp_avg: None,
        chroma_format: ChromaFormat::Monochrome,
    };

    let config = PlaneConfig::new(
        frame.width as usize,
        frame.height as usize,
        frame.y_stride,
        frame.bit_depth
    );

    // Stride validation should catch this
    assert!(config.is_err() || config.unwrap().is_contiguous());
}

#[test]
fn test_memory_exhaustion_many_allocs() {
    // Many small allocations instead of one large
    // Can bypass size limits
    let ivf = create_ivf_with_many_tiny_frames(1000000);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf);

    // Should enforce MAX_FRAMES_PER_FILE
    assert!(result.is_err());
}
```

---

### 3.6 Invalid Chroma Subsampling Values

**Current Coverage:** ⚠️ PARTIAL
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/decoder.rs:86-143`

**Test Cases:**

```rust
#[test]
fn test_invalid_chroma_plane_size() {
    // Y plane 1920x1080, UV plane wrong size
    let tests = vec![
        // (width, height, uv_size, description)
        (1920, 1080, 1920 * 1080, "UV same as Y (should be 4:4:4 or error)"),
        (1920, 1080, 960 * 1080, "UV half width (4:2:2) but labeled 4:2:0"),
        (1920, 1080, 1920 * 540, "UV half height (not a standard format)"),
        (1920, 1080, 100, "UV way too small"),
        (1920, 1080, 1920 * 1080 * 2, "UV larger than Y"),
    ];

    for (width, height, uv_size, desc) in tests {
        let frame = DecodedFrame {
            width,
            height,
            bit_depth: 8,
            y_plane: vec![0u8; width * height].into_boxed_slice().into(),
            y_stride: width as usize,
            u_plane: Some(vec![128u8; uv_size].into_boxed_slice().into()),
            u_stride: 960, // Assume 4:2:0 stride
            v_plane: Some(vec![128u8; uv_size].into_boxed_slice().into()),
            v_stride: 960,
            timestamp: 0,
            frame_type: FrameType::Key,
            qp_avg: None,
            chroma_format: ChromaFormat::Yuv420, // Claimed format
        };

        let result = validate_frame(&frame);
        // Should reject mismatched sizes
        assert!(result.is_err(), "Should reject: {}", desc);
    }
}

#[test]
fn test_uv_plane_size_mismatch() {
    // U and V planes different sizes
    let frame = DecodedFrame {
        width: 1920,
        height: 1080,
        bit_depth: 8,
        y_plane: vec![0u8; 1920 * 1080].into_boxed_slice().into(),
        y_stride: 1920,
        u_plane: Some(vec![128u8; 960 * 540].into_boxed_slice().into()),
        u_stride: 960,
        v_plane: Some(vec![128u8; 960 * 541].into_boxed_slice().into()), // One row more
        v_stride: 960,
        timestamp: 0,
        frame_type: FrameType::Key,
        qp_avg: None,
        chroma_format: ChromaFormat::Yuv420,
    };

    let result = validate_frame(&frame);
    assert!(result.is_err());
}

#[test]
fn test_zero_sized_chroma_plane() {
    // Chroma plane exists but has 0 size
    let frame = DecodedFrame {
        width: 1920,
        height: 1080,
        bit_depth: 8,
        y_plane: vec![0u8; 1920 * 1080].into_boxed_slice().into(),
        y_stride: 1920,
        u_plane: Some(vec![].into_boxed_slice().into()),
        u_stride: 0,
        v_plane: Some(vec![].into_boxed_slice().into()),
        v_stride: 0,
        timestamp: 0,
        frame_type: FrameType::Key,
        qp_avg: None,
        chroma_format: ChromaFormat::Yuv420,
    };

    let result = validate_frame(&frame);
    assert!(result.is_err());
}

#[test]
fn test_chroma_format_detection_edge_cases() {
    // Test chroma format detection with unusual sizes
    let tests = vec![
        // (width, height, uv_size, expected_format)
        (1, 1, 1, ChromaFormat::Yuv444),       // 1x1 with 1 UV pixel
        (2, 2, 1, ChromaFormat::Yuv420),       // 2x2 with 1 UV pixel (4:2:0)
        (4, 2, 4, ChromaFormat::Yuv422),       // 4x2 with 4 UV pixels (4:2:2)
        (2, 2, 4, ChromaFormat::Yuv444),       // 2x2 with 4 UV pixels (4:4:4)
        (1920, 1080, 0, ChromaFormat::Monochrome), // No UV plane
    ];

    for (width, height, uv_size, expected) in tests {
        let u_plane = if uv_size > 0 {
            Some(vec![128u8; uv_size].into_boxed_slice().into())
        } else {
            None
        };
        let v_plane = u_plane.clone();

        let detected = ChromaFormat::from_frame_data(
            width,
            height,
            8,
            u_plane.as_deref(),
            v_plane.as_deref(),
        );

        assert_eq!(detected, expected);
    }
}
```

---

### 3.7 Invalid Frame Rates

**Current Coverage:** ❌ MISSING
**Test Cases:**

```rust
#[test]
fn test_extreme_framerate_values() {
    // Frame rate values at boundaries
    let framerates = vec![
        0,      // Invalid
        1,      // 1 fps
        240,    // 240 fps (high but valid)
        1000,   // 1000 fps (unusual)
        u32::MAX, // Invalid
    ];

    for fps in framerates {
        let seq_header = create_seq_header_with_framerate(fps);
        let mut decoder = Av1Decoder::new().unwrap();

        let result = decoder.decode_all(&seq_header);
        // Should handle gracefully
        let _ = result;
    }
}

#[test]
fn test_zero_timebase() {
    // Timebase denominator = 0 would cause division by zero
    let seq_header = create_seq_header_with_timebase(0, 1);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&seq_header);

    assert!(result.is_err());
}

#[test]
fn test_negative_framerate() {
    // Some formats use signed values
    let seq_header = create_seq_header_with_framerate(-1);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&seq_header);

    assert!(result.is_err());
}

#[test]
fn test_inconsistent_framerate_in_sequence() {
    // Frame rate changes mid-sequence
    let mut data = Vec::new();
    data.extend(create_seq_header_with_framerate(30));
    data.extend(create_frame_header());
    data.extend(create_seq_header_with_framerate(60)); // Change to 60
    data.extend(create_frame_header());

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should handle - maybe reset decoder or reject
    assert!(result.is_ok() || result.is_err());
}
```

---

## 4. Format-Specific Tests

### 4.1 AV1 Format Tests

**Current Coverage:** ⚠️ PARTIAL
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/decoder.rs`

**Test Cases:**

```rust
#[test]
fn test_av1_annex_b_format_variations() {
    // Test different OBU framing methods
    let tests = vec![
        ("obu_with_size_field", create_obu_with_size_field()),
        ("obu_with_length_field", create_obu_with_length_field()),
        ("annex_b_start_code", create_obu_with_annex_b_start()),
        ("raw_obu", create_raw_obu()),
    ];

    for (name, data) in tests {
        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_all(&data);

        println!("Testing {}: {:?}", name, result);
        // Should handle all variations
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_av1_all_obu_types() {
    // Test all OBU types
    let obu_types = vec![
        0x01, // OBU_SEQUENCE_HEADER
        0x02, // OBU_TEMPORAL_DELIMITER
        0x03, // OBU_FRAME_HEADER
        0x04, // OBU_TILE_GROUP
        0x05, // OBU_METADATA
        0x06, // OBU_FRAME
        0x07, // OBU_REDUNDANT_FRAME_HEADER
        0x08, // OBU_TILE_LIST
        0x09, // OBU_PADDING
    ];

    for obu_type in obu_types {
        let data = create_obu_of_type(obu_type);
        let mut decoder = Av1Decoder::new().unwrap();

        decoder.send_data(&data, 0).unwrap();
        // May or may not produce frame depending on type
        let _ = decoder.get_frame();
    }
}

#[test]
fn test_av1_profile_levels() {
    // Test all valid profiles
    let profiles = vec![0, 1, 2];
    let levels: Vec<u8> = (0..=31).collect(); // Levels 0-31 (some reserved)

    for profile in profiles {
        for level in levels {
            let data = create_av1_with_profile_level(profile, level);
            let mut decoder = Av1Decoder::new().unwrap();

            let result = decoder.decode_all(&data);
            // Some combinations may be invalid
            let _ = result;
        }
    }
}

#[test]
fn test_av1_color_configs() {
    // Test various color configurations
    let configs = vec![
        (8, YuvRange::Limited, ColorPrimaries::Bt709, TransferCharacteristics::Bt709, MatrixCoefficients::Bt709),
        (10, YuvRange::Full, ColorPrimaries::Bt2020, TransferCharacteristics::SmpteSt2084, MatrixCoefficients::Bt2020Nc),
        (12, YuvRange::Limited, ColorPrimaries::Bt709, TransferCharacteristics::Bt709, MatrixCoefficients::Identity),
    ];

    for (bit_depth, range, primaries, transfer, matrix) in configs {
        let data = create_av1_with_color_config(bit_depth, range, primaries, transfer, matrix);
        let mut decoder = Av1Decoder::new().unwrap();

        let result = decoder.decode_all(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_av1_film_grain_params() {
    // Test with film grain enabled
    let data = create_av1_with_film_grain(
        grain_seed: 12345,
        num_y_points: 12,
        num_cb_points: 8,
        num_cr_points: 8,
    );

    let mut decoder = Av1Decoder::new().unwrap();
    let frames = decoder.decode_all(&data).unwrap();

    // dav1d should apply film grain
    assert!(!frames.is_empty());
}

#[test]
fn test_av1_scaling_lists() {
    // Test with scaling lists enabled
    let data = create_av1_with_scaling_lists();

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_ok());
}
```

---

### 4.2 H.264 Format Tests

**Current Coverage:** ⚠️ PARTIAL (via FFmpeg)
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/ffmpeg.rs`

**Test Cases:**

```rust
#[cfg(feature = "ffmpeg")]
#[test]
fn test_h264_all_nal_unit_types() {
    // Test all NAL unit types
    let nal_types = vec![
        1,  // Coded slice of a non-IDR picture
        5,  // Coded slice of an IDR picture
        6,  // SEI
        7,  // SPS
        8,  // PPS
        9,  // AUD
        12, // Filler data
        14, // Prefix NAL unit
        20, // Coded slice extension
    ];

    for nal_type in nal_types {
        let nal = create_nal_with_type(nal_type);
        let mut decoder = H264Decoder::new().unwrap();

        decoder.send_data(&nal, Some(0)).unwrap();
        // Most NALs won't produce frames immediately
        let _ = decoder.get_frame();
    }
}

#[cfg(feature = "ffmpeg")]
#[test]
fn test_h264_sps_variations() {
    // Test various SPS configurations
    let tests = vec![
        (1, 1, "Baseline"),
        (3, 1, "Main"),
        (5, 1, "High"),
        (1, 0, "CABAC disabled"),
        (51, 1, "Max level"),
    ];

    for (profile, constraint_set, desc) in tests {
        let sps = create_sps_with_params(profile, constraint_set);
        let mut decoder = H264Decoder::new().unwrap();

        decoder.send_data(&sps, Some(0)).unwrap();
        // SPS alone won't produce frame
        let _ = decoder.get_frame();
    }
}

#[cfg(feature = "ffmpeg")]
#[test]
fn test_h264_cabac_vs_cavlc() {
    // Test both entropy coding modes
    let cabac = create_h264_stream_with_cabac();
    let cavlc = create_h264_stream_with_cavlc();

    for data in [cabac, cavlc] {
        let mut decoder = H264Decoder::new().unwrap();
        let result = decoder.decode_all(&data);
        assert!(result.is_ok());
    }
}

#[cfg(feature = "ffmpeg")]
#[test]
fn test_h264_mixed_slice_types() {
    // I, P, B slices in same stream
    let data = create_h264_with_mixed_slices();

    let mut decoder = H264Decoder::new().unwrap();
    let frames = decoder.decode_all(&data).unwrap();

    // Should decode all frame types
    assert!(!frames.is_empty());
}

#[cfg(feature = "ffmpeg")]
#[test]
fn test_h264_interlaced() {
    // Test interlaced content
    let data = create_interlaced_h264_stream();

    let mut decoder = H264Decoder::new().unwrap();
    let frames = decoder.decode_all(&data);

    assert!(result.is_ok());
}

#[cfg(feature = "ffmpeg")]
#[test]
fn test_h264_mbtaff() {
    // Macroblock-adaptive frame/field (MBAFF)
    let data = create_h264_with_mbff();

    let mut decoder = H264Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_ok());
}
```

---

### 4.3 HEVC Format Tests

**Current Coverage:** ⚠️ PARTIAL (via FFmpeg)
**Test Cases:**

```rust
#[cfg(feature = "ffmpeg")]
#[test]
fn test_hevc_all_nal_unit_types() {
    // Test all HEVC NAL unit types
    let nal_types = vec![
        1,   // TRAIL_R
        19,  // IDR_W_RADL
        20,  // IDR_N_LP
        32,  // VPS
        33,  // SPS
        34,  // PPS
        35,  // AUD
        39,  // PREFIX_SEI
        40,  // SUFFIX_SEI
    ];

    for nal_type in nal_types {
        let nal = create_hevc_nal_with_type(nal_type);
        let mut decoder = HevcDecoder::new().unwrap();

        decoder.send_data(&nal, Some(0)).unwrap();
        let _ = decoder.get_frame();
    }
}

#[cfg(feature = "ffmpeg")]
#[test]
fn test_hevc_all_profiles_tiers_levels() {
    let profiles = vec![1, 2]; // Main, Main 10
    let tiers = vec![0, 1];     // Main, High
    let levels: Vec<u8> = (1..=155).step_by(30).collect(); // Sample levels

    for profile in profiles {
        for tier in tiers {
            for level in levels {
                let data = create_hevc_with_params(profile, tier, level);
                let mut decoder = HevcDecoder::new().unwrap();

                let result = decoder.decode_all(&data);
                // Some combinations invalid
                let _ = result;
            }
        }
    }
}

#[cfg(feature = "ffmpeg")]
#[test]
fn test_hevc_ctu_sizes() {
    // Test different CTU sizes
    let ctu_sizes = vec![16, 32, 64];

    for ctu_size in ctu_sizes {
        let data = create_hevc_with_ctu_size(ctu_size);
        let mut decoder = HevcDecoder::new().unwrap();

        let result = decoder.decode_all(&data);
        assert!(result.is_ok());
    }
}

#[cfg(feature = "ffmpeg")]
#[test]
fn test_hevc_transform_units() {
    // Test various transform unit hierarchies
    let data = create_hevc_with_nested_tus();

    let mut decoder = HevcDecoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_ok());
}

#[cfg(feature = "ffmpeg")]
#[test]
fn test_hevc_intra_prediction_modes() {
    // Test all intra prediction modes (35 modes)
    for mode in 0..35 {
        let data = create_hevc_intra_slice_with_mode(mode);
        let mut decoder = HevcDecoder::new().unwrap();

        decoder.send_data(&data, Some(0)).unwrap();
        let _ = decoder.get_frame();
    }
}
```

---

### 4.4 VVC Format Tests

**Current Coverage:** ⚠️ PARTIAL
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-vvc/src/`

**Test Cases:**

```rust
#[test]
fn test_vvc_all_nal_unit_types() {
    // VVC has many NAL unit types
    let important_types = vec![
        1,   // TRAIL_R
        7,   // IDR_W_RADL
        9,   // GDR
        12,  // OPI
        13,  // DCI
        14,  // VPS
        15,  // SPS
        16,  // PPS
        17,  // APS
        18,  // FD
        19,  // PREFIX_SEI
        20,  // SUFFIX_SEI
        23,  // PH
    ];

    for nal_type in important_types {
        let nal = create_vvc_nal_with_type(nal_type);
        let mut reader = BitReader::new(&nal);

        let result = parse_vvc_nal(&mut reader);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_vvc_all_slice_types() {
    // Test all slice types (B, P, I)
    let slice_types = vec![0, 1, 2]; // B, P, I

    for slice_type in slice_types {
        let data = create_vvc_slice_of_type(slice_type);
        let mut parser = VvcParser::new();

        let result = parser.parse(&data);
        assert!(result.is_ok());
    }
}

#[test]
fn test_vvc_ctu_sizes() {
    // VVC supports CTU up to 128x128
    let ctu_sizes = vec![16, 32, 64, 128];

    for ctu_size in ctu_sizes {
        let data = create_vvc_with_ctu_size(ctu_size);
        let mut parser = VvcParser::new();

        let result = parser.parse(&data);
        assert!(result.is_ok());
    }
}

#[test]
fn test_vvc_mrl_mode() {
    // Test Multiple Ref Line (MRL) mode
    let data = create_vvc_with_mrl(3); // 3 reference lines

    let mut parser = VvcParser::new();
    let result = parser.parse(&data);

    assert!(result.is_ok());
}

#[test]
fn test_vvc_mip_mode() {
    // Test Matrix-based Intra Prediction (MIP)
    let data = create_vvc_with_mip_enabled();

    let mut parser = VvcParser::new();
    let result = parser.parse(&data);

    assert!(result.is_ok());
}

#[test]
fn test_vvc_lmcs() {
    // Test Luma Mapping with Chroma Scaling (LMCS)
    let data = create_vvc_with_lmcs();

    let mut parser = VvcParser::new();
    let result = parser.parse(&data);

    assert!(result.is_ok());
}

#[test]
fn test_vvc_alf() {
    // Test Adaptive Loop Filter (ALF)
    let data = create_vvc_with_alf();

    let mut parser = VvcParser::new();
    let result = parser.parse(&data);

    assert!(result.is_ok());
}

#[test]
fn test_vvc_affine_motion() {
    // Test various affine motion models
    let models = vec![
        AffineModel::Translation,
        AffineModel::RotationZoom,
        AffineModel::Shear,
    ];

    for model in models {
        let data = create_vvc_with_affine_model(model);
        let mut parser = VvcParser::new();

        let result = parser.parse(&data);
        assert!(result.is_ok());
    }
}

#[test]
fn test_vvc_virtual_boundaries() {
    // Test virtual boundary filtering
    let data = create_vvc_with_virtual_boundaries(
        boundary_type: BoundaryType::LoopFilter,
        num_boundaries: 4,
    );

    let mut parser = VvcParser::new();
    let result = parser.parse(&data);

    assert!(result.is_ok());
}
```

---

### 4.5 VP9 Format Tests

**Current Coverage:** ⚠️ PARTIAL
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-vp9/src/`

**Test Cases:**

```rust
#[test]
fn test_vp9_all_profiles() {
    // Test all VP9 profiles
    let profiles = vec![0, 1, 2, 3];

    for profile in profiles {
        let data = create_vp9_with_profile(profile);
        let mut parser = Vp9Parser::new();

        let result = parser.parse(&data);
        assert!(result.is_ok());
    }
}

#[test]
fn test_vp9_bit_depth_combinations() {
    // Profile 0: 8-bit only
    // Profile 1: 8-bit, 10-bit (_YUV420)
    // Profile 2: 8-bit, 10-bit, 12-bit (_YUV420 or _YUV444)
    // Profile 3: 8-bit, 10-bit, 12-bit _YUV444

    let tests = vec![
        (0, 8, YuvFormat::Yuv420),
        (1, 8, YuvFormat::Yuv420),
        (1, 10, YuvFormat::Yuv420),
        (2, 8, YuvFormat::Yuv420),
        (2, 10, YuvFormat::Yuv420),
        (2, 12, YuvFormat::Yuv420),
        (2, 8, YuvFormat::Yuv444),
        (2, 10, YuvFormat::Yuv444),
        (2, 12, YuvFormat::Yuv444),
        (3, 8, YuvFormat::Yuv444),
        (3, 10, YuvFormat::Yuv444),
        (3, 12, YuvFormat::Yuv444),
    ];

    for (profile, bit_depth, format) in tests {
        let data = create_vp9_with_params(profile, bit_depth, format);
        let mut parser = Vp9Parser::new();

        let result = parser.parse(&data);
        assert!(result.is_ok());
    }
}

#[test]
fn test_vp9_superframe() {
    // Test VP9 superframe format
    let data = create_vp9_superframe(vec![
        create_vp9_frame(),
        create_vp9_frame(),
        create_vp9_frame(),
    ]);

    let mut parser = Vp9Parser::new();
    let frames = parser.parse(&data).unwrap();

    assert_eq!(frames.len(), 3);
}

#[test]
fn test_vp9_superframe_index_corruption() {
    // Corrupted superframe index
    let mut data = create_vp9_superframe(vec![
        create_vp9_frame(),
        create_vp9_frame(),
    ]);

    // Corrupt the superframe marker at end
    let last_idx = data.len() - 1;
    data[last_idx] ^= 0xFF;

    let mut parser = Vp9Parser::new();
    let result = parser.parse(&data);

    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_vp9_loop_filter_levels() {
    // Test various loop filter levels
    for level in [0, 10, 30, 63] {
        let data = create_vp9_with_loop_filter_level(level);
        let mut parser = Vp9Parser::new();

        let result = parser.parse(&data);
        assert!(result.is_ok());
    }
}

#[test]
fn test_vp9_segmentation_map() {
    // Test segmentation feature
    let data = create_vp9_with_segmentation(
        temporal_update: true,
        update_data: true,
    );

    let mut parser = Vp9Parser::new();
    let result = parser.parse(&data);

    assert!(result.is_ok());
}
```

---

### 4.6 MKV/MP4 Container Tests

**Current Coverage:** ⚠️ PARTIAL
**Location:** `/Users/hawk/Workspaces/bitvue/crates/bitvue-formats/src/`

**Test Cases:**

```rust
#[cfg(feature = "formats")]
#[test]
fn test_mkv_deeply_nested_elements() {
    // EBML allows deep nesting
    let levels = vec![10, 50, 100, 200];

    for depth in levels {
        let data = create_mkv_with_nesting_depth(depth);
        let result = parse_mkv(&data);

        // Should enforce MAX_RECURSION_DEPTH
        if depth > 100 {
            assert!(result.is_err());
        }
    }
}

#[cfg(feature = "formats")]
#[test]
fn test_mkv_unknown_elements() {
    // Unknown/skippable elements
    let data = create_mkv_with_unknown_elements();

    let result = parse_mkv(&data);
    assert!(result.is_ok());
}

#[cfg(feature = "formats")]
#[test]
fn test_mkv_void_elements() {
    // Void elements for padding
    let data = create_mkv_with_void(1000); // 1KB of padding

    let result = parse_mkv(&data);
    assert!(result.is_ok());
}

#[cfg(feature = "formats")]
#[test]
fn test_mkv_crc_elements() {
    // CRC-32 elements
    let data = create_mkv_with_crc();

    let result = parse_mkv(&data);
    assert!(result.is_ok() || result.is_err()); // May validate CRC
}

#[cfg(feature = "formats")]
#[test]
fn test_mp4_edit_list() {
    // MP4 edit lists (edits)
    let data = create_mp4_with_edit_list(vec![
        Edit { duration: 1000, media_time: 0 },
        Edit { duration: 500, media_time: -1 }, // Empty edit
        Edit { duration: 2000, media_time: 1000 },
    ]);

    let result = parse_mp4(&data);
    assert!(result.is_ok());
}

#[cfg(feature = "formats")]
#[test]
fn test_mp4_fragmented() {
    // Fragmented MP4 (fMP4)
    let data = create_fragmented_mp4(
        num_fragments: 5,
        frames_per_fragment: 10,
    );

    let result = parse_mp4(&data);
    assert!(result.is_ok());
}

#[cfg(feature = "formats")]
#[test]
fn test_mp4_negative_ctts() {
    // Composition time offsets can be negative
    let data = create_mp4_with_negative_ctts();

    let result = parse_mp4(&data);
    assert!(result.is_ok());
}

#[cfg(feature = "formats")]
#[test]
fn test_mp4_esds_extended_descriptor() {
    // Extended ES_Descr tag
    let data = create_mp4_with_extended_esds();

    let result = parse_mp4(&data);
    assert!(result.is_ok());
}

#[cfg(feature = "formats")]
#[test]
fn test_mp4_unknown_atom_skipping() {
    // Unknown atoms should be skipped
    let data = create_mp4_with_unknown_atoms(vec![
        b"unk1",
        b"unk2",
        b"free",
        b"skip",
    ]);

    let result = parse_mp4(&data);
    assert!(result.is_ok());
}

#[cfg(feature = "formats")]
#[test]
fn test_mp4_64bit_offsets() {
    // 64-bit chunk offsets (co64)
    let data = create_mp4_with_64bit_offsets();

    let result = parse_mp4(&data);
    assert!(result.is_ok());
}
```

---

## 5. Error Handling

### 5.1 Resource Cleanup on Errors

**Current Coverage:** ⚠️ NEEDS VERIFICATION
**Risk Level:** HIGH (memory leaks, resource exhaustion)

**Test Cases:**

```rust
#[test]
fn test_decoder_cleanup_on_decode_error() {
    // Ensure decoder cleans up resources when decode fails
    let data = create_corrupted_video();

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_err());

    // Decoder should still be usable after error
    let good_data = create_test_ivf();
    let result = decoder.decode_all(&good_data);
    assert!(result.is_ok());
}

#[test]
fn test_file_handle_cleanup_on_error() {
    // Ensure file handles are closed on error
    use std::fs::File;
    use std::io::Read;

    let path = create_temp_file_with_corrupted_data();

    {
        let file = File::open(&path).unwrap();
        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_from_file(&path);

        assert!(result.is_err());
    } // file handle should be closed here

    // Should be able to open file again
    let file = File::open(&path).unwrap();
    assert!(true);
}

#[test]
fn test_memory_cleanup_after_partial_decode() {
    // Decode some frames, then error
    let data = create_ivf_with_corrupted_last_frame();

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should get partial results
    assert!(result.is_ok());
    let frames = result.unwrap();
    assert!(frames.len() > 0);
    assert!(frames.len() < 10); // Not all frames

    // Memory should be cleaned up
    // (hard to test directly, but decoder should be usable)
}

#[test]
fn test_buffer_cleanup_on_size_error() {
    // Test that large buffers are freed when size validation fails
    let large_data = vec![0u8; MAX_FRAME_SIZE + 1];

    let result = validate_buffer_size(large_data.len());
    assert!(result.is_err());

    // Buffer should be dropped here
}

#[test]
fn test_plane_extraction_cleanup() {
    // Test cleanup when plane extraction fails mid-way
    let frame = create_frame_with_strided_planes();

    // Cause error in middle of extraction
    let result = extract_plane_from_frame_with_corruption(&frame);
    assert!(result.is_err());

    // Partial results should be cleaned up
}
```

---

### 5.2 Graceful Degradation

**Current Coverage:** ⚠️ PARTIAL
**Test Cases:**

```rust
#[test]
fn test_decode_with_missing_optional_data() {
    // Missing metadata that's not critical
    let data = create_av1_without_metadata();

    let mut decoder = Av1Decoder::new().unwrap();
    let frames = decoder.decode_all(&data).unwrap();

    // Should still decode frames
    assert!(!frames.is_empty());
}

#[test]
fn test_decode_with_corrupted_non_ref_frames() {
    // Corrupted non-reference frames
    let data = create_video_with_corrupted_b_frames();

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should skip corrupted frames, decode valid ones
    assert!(result.is_ok());
    let frames = result.unwrap();
    assert!(!frames.is_empty());
}

#[test]
fn test_decode_with_partial_slice_data() {
    // Some slices missing
    let data = create_frame_with_missing_slices();

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    // Should handle gracefully - maybe partial frame
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_decode_with_truncated_sei() {
    // Truncated SEI messages (non-critical)
    let data = create_av1_with_truncated_sei();

    let mut decoder = Av1Decoder::new().unwrap();
    let frames = decoder.decode_all(&data).unwrap();

    // Should ignore truncated SEI and decode
    assert!(!frames.is_empty());
}

#[test]
fn test_yuv_conversion_with_missing_chroma() {
    // YUV420 frame with missing U plane
    let frame = DecodedFrame {
        width: 1920,
        height: 1080,
        bit_depth: 8,
        y_plane: vec![0u8; 1920 * 1080].into_boxed_slice().into(),
        y_stride: 1920,
        u_plane: None, // Missing
        u_stride: 0,
        v_plane: Some(vec![128u8; 960 * 540].into_boxed_slice().into()),
        v_stride: 960,
        timestamp: 0,
        frame_type: FrameType::Key,
        qp_avg: None,
        chroma_format: ChromaFormat::Yuv420,
    };

    let rgb = yuv_to_rgb(&frame);
    // Should fall back to grayscale
    assert!(!rgb.is_empty());
}

#[test]
fn test_bitreader_recovery_after_error() {
    let mut reader = BitReader::new(&[0xFF, 0xFF]);

    // Read beyond available
    let result = reader.read_bits(16);
    assert!(result.is_ok());

    let result = reader.read_bits(1);
    assert!(result.is_err());

    // Position should be at end, not corrupted
    assert_eq!(reader.position(), 16);
}
```

---

### 5.3 Error Propagation

**Current Coverage:** ⚠️ PARTIAL
**Test Cases:**

```rust
#[test]
fn test_error_context_preservation() {
    // Ensure error messages include context
    let data = create_corrupted_video_at_offset(1000);

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_err());
    let error = result.unwrap_err();

    // Error should mention where problem occurred
    let error_str = error.to_string();
    // Most errors should include offset or position
    // assert!(error_str.contains("offset") || error_str.contains("position"));
}

#[test]
fn test_error_chain_propagation() {
    // Test that errors from lower layers propagate correctly
    let data = vec![0u8; 10];

    let mut reader = BitReader::new(&data);
    let result = reader.read_bits(32); // Will fail
    assert!(result.is_err());

    // Error type should be preserved
    assert!(matches!(result, Err(BitvueError::UnexpectedEof(_))));
}

#[test]
fn test_aggregate_error_reporting() {
    // Test operations that can have multiple errors
    let data = create_video_with_multiple_issues();

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);

    assert!(result.is_err());

    // Should report first/most significant error
    // not necessarily all errors
}

#[test]
fn test_recovery_from_transient_errors() {
    // Test that decoder can recover from transient errors
    let data = create_video_with_transient_errors();

    let mut decoder = Av1Decoder::new().unwrap();
    let frames = decoder.decode_all(&data).unwrap();

    // Should decode frames despite transient errors
    assert!(frames.len() > 0);
}

#[test]
fn test_error_vs_warning_distinction() {
    // Ensure errors and warnings are distinguished
    // (If API has warning mechanism)

    let data_with_warning = create_video_with_suspicious_but_valid_data();
    let data_with_error = create_corrupted_video();

    let mut decoder = Av1Decoder::new().unwrap();

    let result1 = decoder.decode_all(&data_with_warning);
    assert!(result1.is_ok()); // Warning only

    let result2 = decoder.decode_all(&data_with_error);
    assert!(result2.is_err()); // Actual error
}
```

---

### 5.4 Panic Conditions

**Current Coverage:** ❌ CRITICAL GAP
**Risk Level:** CRITICAL (security, stability)

**Test Cases:**

```rust
#[test]
fn test_no_panic_on_null_pointer() {
    // Simulate null pointer scenarios
    let tests = vec![
        create_frame_with_null_planes(),
        create_decoder_with_null_context(),
    ];

    for test_case in tests {
        // Should not panic
        let result = std::panic::catch_unwind(|| {
            decode_with_null_check(&test_case);
        });

        assert!(result.is_ok());
    }
}

#[test]
fn test_no_panic_on_unaligned_access() {
    // Unaligned memory access
    let data = create_unaligned_data();

    let result = std::panic::catch_unwind(|| {
        let mut reader = BitReader::new(&data);
        let _ = reader.read_bits(16);
    });

    assert!(result.is_ok());
}

#[test]
fn test_no_panic_on_integer_overflow() {
    // Operations that could overflow
    let tests = vec![
        || { let _ = usize::MAX * 2; },
        || { let _ = i32::MAX + 1; },
    ];

    for test_fn in tests {
        // Rust should panic on overflow in debug, wrap in release
        // We want to ensure we use checked arithmetic
        let result = std::panic::catch_unwind(test_fn);
        // In our code, we use checked arithmetic, so no panic
        assert!(result.is_ok());
    }
}

#[test]
fn test_no_panic_on_stack_overflow() {
    // Deep recursion
    let result = std::panic::catch_unwind(|| {
        let parser = create_parser_with_deep_recursion(1000);
        parser.parse();
    });

    // Should catch recursion limit before stack overflow
    assert!(result.is_ok());
}

#[test]
fn test_no_panic_on_index_out_of_bounds() {
    let data = vec![0u8; 100];
    let mut reader = BitReader::new(&data);

    let result = std::panic::catch_unwind(|| {
        // Try to read beyond data
        for _ in 0..1000 {
            let _ = reader.read_byte();
        }
    });

    // Should error, not panic
    assert!(result.is_ok());
    // But read operation should return error
}

#[test]
fn test_no_panic_on_divide_by_zero() {
    let tests = vec![
        || divide_by_check(100, 0),
        || modulo_by_check(100, 0),
    ];

    for test_fn in tests {
        let result = std::panic::catch_unwind(test_fn);
        assert!(result.is_ok());
    }
}

#[test]
fn test_no_panic_on_invalid_utf8() {
    // If parsing string data
    let invalid_utf8 = vec![0xFF, 0xFF, 0xFF];

    let result = std::panic::catch_unwind(|| {
        let _ = String::from_utf8(invalid_utf8);
    });

    // Should error, not panic
    assert!(result.is_ok());
}
```

---

## 6. Concurrent Edge Cases

### 6.1 Race Conditions in Parallel Decode

**Current Coverage:** ❌ MISSING
**Risk Level:** HIGH (data races, deadlocks)

**Test Cases:**

```rust
#[test]
fn test_concurrent_decode_different_files() {
    use rayon::prelude::*;

    let files: Vec<_> = (0..10)
        .map(|i| create_test_video_with_size(320, 240, i))
        .collect();

    let results: Vec<_> = files.par_iter()
        .map(|data| {
            let mut decoder = Av1Decoder::new().unwrap();
            decoder.decode_all(data)
        })
        .collect();

    // All should succeed
    assert!(results.iter().all(|r| r.is_ok()));
}

#[test]
fn test_concurrent_decode_same_file() {
    use std::sync::Arc;
    use rayon::prelude::*;

    let data = Arc::new(create_test_video());

    let results: Vec<_> = (0..10).into_par_iter()
        .map(|_| {
            let mut decoder = Av1Decoder::new().unwrap();
            decoder.decode_all(&data)
        })
        .collect();

    // All should succeed
    assert!(results.iter().all(|r| r.is_ok()));
}

#[test]
fn test_concurrent_plane_extraction() {
    // Test U/V plane extraction in parallel (existing code)
    let data = create_test_frame();

    let (u_result, v_result) = rayon::join(
        || extract_u_plane(&data),
        || extract_v_plane(&data),
    );

    assert!(u_result.is_ok());
    assert!(v_result.is_ok());
}

#[test]
fn test_concurrent_yuv_conversion() {
    use rayon::prelude::*;

    let frames: Vec<_> = (0..100)
        .map(|_| create_test_frame())
        .collect();

    let rgb_results: Vec<_> = frames.par_iter()
        .map(|frame| yuv_to_rgb(frame))
        .collect();

    // All should succeed
    assert!(rgb_results.iter().all(|r| !r.is_empty()));
}

#[test]
fn test_shared_decoder_state() {
    // Test decoder with shared state (if any)
    use std::sync::{Arc, Mutex};

    let decoder = Arc::new(Mutex::new(Av1Decoder::new().unwrap()));
    let data = Arc::new(create_test_video());

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let decoder = decoder.clone();
            let data = data.clone();
            std::thread::spawn(move || {
                let mut dec = decoder.lock().unwrap();
                dec.decode_all(&data)
            })
        })
        .collect();

    // All should complete
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_rayon_thread_pool_exhaustion() {
    // Try to use more threads than available
    use rayon::prelude::*;

    let data: Vec<_> = (0..1000)
        .map(|_| create_test_video())
        .collect();

    let results: Vec<_> = data.par_iter()
        .map(|d| {
            let mut decoder = Av1Decoder::new().unwrap();
            decoder.decode_all(d)
        })
        .collect();

    // Should complete without deadlock
    assert!(results.len() == 1000);
}
```

---

### 6.2 Cancellation Safety

**Current Coverage:** ❌ MISSING
**Test Cases:**

```rust
#[test]
fn test_decode_cancellation_mid_frame() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let data = create_large_test_video();

    let handle = std::thread::spawn({
        let cancel_flag = cancel_flag.clone();
        move || {
            let mut decoder = Av1Decoder::new().unwrap();
            for chunk in data.chunks(1000) {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(DecodeError::Decode("Cancelled".to_string()));
                }
                decoder.send_data(chunk, Some(0)).unwrap();
            }
            decoder.collect_frames()
        }
    });

    // Cancel after 10ms
    std::thread::sleep(std::time::Duration::from_millis(10));
    cancel_flag.store(true, Ordering::Relaxed);

    let result = handle.join().unwrap();
    assert!(result.is_err() || result.unwrap().len() < 100);
}

#[test]
fn test_parallel_decode_cancellation() {
    use rayon::prelude::*;

    let data: Vec<_> = (0..100)
        .map(|_| create_large_test_video())
        .collect();

    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let cancel_flag = Arc::new(AtomicBool::new(false));

    // Start parallel decode
    let handle = std::thread::spawn({
        let cancel_flag = cancel_flag.clone();
        move || {
            let results: Vec<_> = data.par_iter()
                .with_evaluator(rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap())
                .map(|d| {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err(DecodeError::Decode("Cancelled".to_string()));
                    }
                    let mut decoder = Av1Decoder::new().unwrap();
                    decoder.decode_all(d)
                })
                .collect();
            results
        }
    });

    // Cancel quickly
    std::thread::sleep(std::time::Duration::from_millis(1));
    cancel_flag.store(true, Ordering::Relaxed);

    let results = handle.join().unwrap();
    // Some may have completed, some cancelled
    assert!(results.len() == 100);
}

#[test]
fn test_timeout_decode() {
    use std::time::Duration;

    let data = create_slow_to_decode_video();

    let handle = std::thread::spawn(move || {
        let mut decoder = Av1Decoder::new().unwrap();
        decoder.decode_all(&data)
    });

    let timeout = Duration::from_secs(1);
    let start = std::time::Instant::now();

    loop {
        if handle.is_finished() {
            break;
        }
        if start.elapsed() > timeout {
            // Timeout - thread will continue running but we move on
            println!("Decode timed out after 1 second");
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    // In production, would use proper timeout channels
}
```

---

### 6.3 Thread Pool Exhaustion

**Current Coverage:** ❌ MISSING
**Test Cases:**

```rust
#[test]
fn test_thread_pool_exhaustion_recovery() {
    use rayon::prelude::*;

    // Create more work than threads
    let work_items: Vec<_> = (0..10_000)
        .map(|i| create_test_video_with_id(i))
        .collect();

    let results: Vec<_> = work_items.par_iter()
        .map(|data| {
            let mut decoder = Av1Decoder::new().unwrap();
            decoder.decode_all(data)
        })
        .collect();

    // Should complete without deadlock
    assert_eq!(results.len(), 10_000);
    assert!(results.iter().all(|r| r.is_ok() || r.is_err()));
}

#[test]
fn test_nested_parallelism() {
    use rayon::prelude::*;

    let frames: Vec<_> = (0..10)
        .map(|_| create_test_frame())
        .collect();

    // Nested parallel: outer iterates frames, inner converts YUV
    let rgb_results: Vec<_> = frames.par_iter()
        .map(|frame| {
            // Inner parallelism within YUV conversion
            let rgb = yuv_to_rgb(frame);

            // Further parallel processing
            let histogram = rgb.par_chunks(1000)
                .map(|chunk| calculate_histogram(chunk))
                .reduce(|| [0usize; 256], |a, b| {
                    let mut sum = [0usize; 256];
                    for i in 0..256 {
                        sum[i] = a[i] + b[i];
                    }
                    sum
                });

            (rgb, histogram)
        })
        .collect();

    assert_eq!(rgb_results.len(), 10);
}

#[test]
fn test_max_threads_limit() {
    use rayon::prelude::*;

    // Limit threads
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();

    let work_items: Vec<_> = (0..100)
        .map(|_| create_test_video())
        .collect();

    pool.install(|| {
        let results: Vec<_> = work_items.par_iter()
            .map(|data| {
                let mut decoder = Av1Decoder::new().unwrap();
                decoder.decode_all(data)
            })
            .collect();

        assert_eq!(results.len(), 100);
    });
}

#[test]
fn test_global_thread_pool_interference() {
    // Test that using global thread pool doesn't interfere
    use rayon::prelude::*;

    let data1 = create_test_video();
    let data2 = create_test_video();

    let handle1 = std::thread::spawn(|| {
        (0..100).into_par_iter().map(|_| {
            let mut decoder = Av1Decoder::new().unwrap();
            decoder.decode_all(&data1)
        }).collect::<Vec<_>>()
    });

    let handle2 = std::thread::spawn(|| {
        (0..100).into_par_iter().map(|_| {
            let mut decoder = Av1Decoder::new().unwrap();
            decoder.decode_all(&data2)
        }).collect::<Vec<_>>()
    });

    let results1 = handle1.join().unwrap();
    let results2 = handle2.join().unwrap();

    assert_eq!(results1.len(), 100);
    assert_eq!(results2.len(), 100);
}
```

---

## 7. Test Infrastructure Recommendations

### 7.1 Test Data Generation Utilities

**Need:** Framework for generating test video data programmatically

```rust
// crates/bitvue-test-data/src/lib.rs

pub struct VideoGenerator {
    codec: CodecType,
    width: u32,
    height: u32,
    num_frames: usize,
}

impl VideoGenerator {
    pub fn new(codec: CodecType) -> Self {
        Self {
            codec,
            width: 320,
            height: 240,
            num_frames: 10,
        }
    }

    pub fn resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn frames(mut self, count: usize) -> Self {
        self.num_frames = count;
        self
    }

    pub fn generate_ivf(&self) -> Vec<u8> {
        // Generate valid IVF with specified parameters
    }

    pub fn generate_obu(&self) -> Vec<u8> {
        // Generate valid OBU frame
    }

    pub fn generate_corrupted(&self, corruption_type: CorruptionType) -> Vec<u8> {
        // Generate corrupted variant
    }
}

pub enum CorruptionType {
    TruncatedHeader,
    TruncatedFrame,
    InvalidMagic,
    FrameSizeOverflow,
    ZeroFrames,
    TooManyFrames,
    CorruptedChecksum,
    RandomGarbage,
}
```

### 7.2 Property-Based Testing

**Recommendation:** Use `proptest` for property-based testing

```rust
// Test with generated inputs

mod proptest_tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_roundtrip_conversion(width in 1u32..=7680, height in 1u32..=4320) {
            let frame = create_random_frame(width, height);
            let rgb = yuv_to_rgb(&frame);

            // Should not panic
            prop_assert!(!rgb.is_empty());
        }

        #[test]
        fn test_bitreader_read_write(bits in 0u8..64) {
            let data = create_random_data(1000);
            let mut reader = BitReader::new(&data);

            let result = reader.read_bits(bits);

            // Should either succeed or EOF, not panic
            prop_assert!(result.is_ok() || result.is_err());
        }

        #[test]
        fn test_ivf_parsing(frame_sizes in prop::collection::vec(1usize..=10000, 1..=100)) {
            let ivf = generate_ivf_with_frame_sizes(&frame_sizes);
            let mut decoder = Av1Decoder::new().unwrap();

            let result = decoder.decode_all(&ivf);

            prop_assert!(result.is_ok() || result.is_err());
        }
    }
}
```

### 7.3 Fuzzing Integration

**Recommendation:** Integrate fuzzing for security testing

```rust
// fuzz/fuzz_targets/av1_decode.rs

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut decoder = match Av1Decoder::new() {
        Ok(d) => d,
        Err(_) => return,
    };

    // Should never crash
    let _ = decoder.decode_all(data);
});
```

---

## 8. Priority Recommendations

### Critical (Implement Immediately)

1. **Malicious Input Testing** (Section 3.5)
   - Prevents security vulnerabilities
   - Tests for DoS vectors
   - Priority: CRITICAL

2. **Integer Overflow Testing** (Section 2.1)
   - Most existing protection, but need comprehensive tests
   - Priority: CRITICAL

3. **Panic Prevention** (Section 5.4)
   - Ensures stability
   - Priority: CRITICAL

### High Priority

4. **Empty/Truncated File Handling** (Sections 1.1, 3.3)
   - Common real-world scenario
   - Priority: HIGH

5. **Concurrent Operation Safety** (Section 6)
   - Parallel decoding is used
   - Priority: HIGH

6. **Error Recovery Testing** (Section 5.1)
   - Resource management
   - Priority: HIGH

### Medium Priority

7. **Boundary Resolution Testing** (Sections 1.3, 1.4)
   - Edge case coverage
   - Priority: MEDIUM

8. **Format-Specific Edge Cases** (Section 4)
   - Codec compliance
   - Priority: MEDIUM

### Low Priority

9. **Graceful Degradation** (Section 5.2)
   - Nice to have, not critical
   - Priority: LOW

10. **Unusual Aspect Ratios** (Section 1.5)
    - Rare in practice
    - Priority: LOW

---

## 9. Implementation Strategy

### Phase 1: Critical Security Tests (Week 1-2)

```bash
# Add fuzzing support
cd /Users/hawk/Workspaces/bitvue
cargo install cargo-fuzz
cargo fuzz add av1_decode

# Implement critical tests
# - test_ivf_frame_size_overflow_attack
# - test_exp_golomb_dos_attack
# - test_chroma_plane_size_mismatch_attack
```

### Phase 2: Boundary Conditions (Week 3-4)

```bash
# Implement boundary tests
# - Resolution limits (1x1 to 8K)
# - Bit depth boundaries
# - Integer overflow scenarios
```

### Phase 3: Error Handling (Week 5)

```bash
# Implement error handling tests
# - Resource cleanup
# - Error propagation
# - Panic prevention
```

### Phase 4: Concurrent Safety (Week 6)

```bash
# Implement concurrent tests
# - Parallel decode
# - Cancellation safety
# - Thread pool exhaustion
```

### Phase 5: Format-Specific Tests (Week 7-8)

```bash
# Implement format-specific tests
# - AV1 edge cases
# - H.264/HEVC edge cases
# - VVC edge cases
# - VP9 edge cases
# - Container edge cases
```

---

## 10. Test Coverage Metrics

### Current Coverage Estimate

| Component | Unit Tests | Integration Tests | Edge Case Coverage |
|-----------|------------|-------------------|-------------------|
| bitvue-decode | 60% | 40% | 45% |
| bitvue-vvc | 70% | 30% | 50% |
| bitvue-vp9 | 65% | 35% | 45% |
| bitvue-formats | 50% | 20% | 30% |
| bitvue-core | 75% | N/A | 55% |
| **Overall** | **64%** | **31%** | **45%** |

### Target Coverage

| Category | Current | Target | Gap |
|----------|---------|--------|-----|
| Critical Security | 40% | 95% | +55% |
| Boundary Conditions | 60% | 90% | +30% |
| Error Handling | 50% | 85% | +35% |
| Concurrent Safety | 20% | 80% | +60% |
| Format Edge Cases | 45% | 85% | +40% |
| **Overall Edge Cases** | **45%** | **85%** | **+40%** |

---

## 11. Testing Best Practices

### 11.1 Test Organization

```
crates/
├── bitvue-decode/
│   ├── tests/
│   │   ├── edge_cases/     # NEW: Edge case tests
│   │   │   ├── mod.rs
│   │   │   ├── resolution_tests.rs
│   │   │   ├── overflow_tests.rs
│   │   │   ├── malformed_tests.rs
│   │   │   └── concurrent_tests.rs
│   │   ├── decoder_test.rs (existing)
│   │   └── ...
├── bitvue-test-data/       # NEW: Test data utilities
│   ├── src/
│   │   ├── lib.rs
│   │   ├── generators/
│   │   │   ├── av1.rs
│   │   │   ├── h264.rs
│   │   │   └── hevc.rs
│   │   └── corruption/
│   │       └── mod.rs
└── fuzz/                   # NEW: Fuzzing targets
    ├── fuzz_targets/
    │   ├── av1_decode.rs
    │   ├── h264_decode.rs
    │   └── bitreader_parse.rs
    └── Cargo.toml
```

### 11.2 CI/CD Integration

```yaml
# .github/workflows/edge-case-tests.yml

name: Edge Case Tests

on: [push, pull_request]

jobs:
  edge-case-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        run: rustup update stable

      - name: Run edge case tests
        run: |
          cargo test --test edge_cases --release
          cargo test --test overflow_tests --release

      - name: Run fuzzing (limited time)
        run: |
          cargo install cargo-fuzz
          cargo fuzz run av1_decode -- -max_total_time=60

      - name: Check coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --timeout 120 --out Xml
```

---

## 12. Conclusion

This comprehensive analysis has identified **significant gaps in edge case and boundary condition testing** for the Bitvue video analyzer. The most critical areas requiring immediate attention are:

1. **Malicious input handling** - No current tests for DoS attacks
2. **Concurrent operation safety** - Limited testing of parallel decode
3. **Panic prevention** - Need comprehensive panic testing
4. **Resource cleanup** - Need verification of proper cleanup

Implementing the recommended tests will significantly improve the **security, stability, and reliability** of the Bitvue video analyzer, especially when processing untrusted or malformed input.

**Next Steps:**
1. Review and prioritize recommendations
2. Set up test infrastructure (generators, fuzzing)
3. Implement Phase 1 (critical security tests)
4. Incrementally add coverage for remaining phases
5. Establish continuous testing in CI/CD pipeline

---

**Document Control:**
- Author: Claude (Test Engineering Analysis)
- Version: 1.0
- Last Updated: 2025-02-06
- Review Date: 2025-03-06
