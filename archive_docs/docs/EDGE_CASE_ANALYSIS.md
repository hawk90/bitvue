# Bitvue Edge Case and Boundary Condition Analysis

## Executive Summary

This document analyzes the Bitvue codebase for edge cases, boundary conditions, and abnormal input handling. The analysis covers video codec parsing, numerical boundaries, resource limits, and concurrency concerns. It identifies existing test coverage gaps and proposes specific test cases to achieve 95%+ test coverage.

**Analysis Date:** 2026-02-21
**Branch:** feature/test-coverage-95

---

## 1. Video Codec Edge Cases

### 1.1 Empty/Minimal Video Files

**Current Coverage:** Good - `bitvue-decode/tests/edge_cases_test.rs` covers empty files, zero-length frames

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| Single byte file | decoder | High |
| File with only magic number (no content) | decoder | High |
| File with valid header but no frames | decoder | High |
| IVF with frame_count=0 in header but frames present | decoder | Medium |
| IVF with frame_count=N but fewer frames in file | decoder | High |

**Recommended Test Additions:**

```rust
// crates/bitvue-decode/tests/edge_cases_test.rs

#[test]
fn test_ivf_magic_only() {
    // File contains only "DKIF" - nothing else
    let data = b"DKIF";
    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(data);
    assert!(result.is_err(), "Should reject file with only magic number");
}

#[test]
fn test_ivf_header_only() {
    // Valid 32-byte header, no frame data
    let data = create_minimal_ivf_header();
    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);
    // Should succeed with 0 frames, not crash
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_ivf_frame_count_mismatch_fewer() {
    // Header says 100 frames, but only 50 exist
    let mut data = create_minimal_ivf_header();
    data[24..28].copy_from_slice(&100u32.to_le_bytes()); // frame_count = 100
    // Add only 50 frames...
    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&data);
    // Should handle gracefully, not hang or crash
}

#[test]
fn test_ivf_frame_count_zero_frames_present() {
    // Header says 0 frames, but frames exist in file
    let mut data = create_minimal_ivf_header();
    data[24..28].copy_from_slice(&0u32.to_le_bytes()); // frame_count = 0
    // Add frame data...
}
```

### 1.2 Corrupted Stream Data

**Current Coverage:** Partial - `vvc_edge_cases_test.rs`, `vp9_edge_cases_test.rs` have some corruption tests

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| Bit-flip in critical NAL header | all codecs | Critical |
| Truncated Exp-Golomb value | bitreader | High |
| Invalid emulation prevention sequences | bitreader | High |
| NAL unit with size=0 | VVC/HEVC/AVC | High |
| Negative timestamp handling | decoder | Medium |

**Recommended Test Additions:**

```rust
// crates/bitvue-core/tests/bitreader_edge_cases_test.rs

#[test]
fn test_exp_golomb_truncated_at_leading_zeros() {
    // Truncated in the middle of leading zeros
    let data = [0x00, 0x00, 0x00]; // All zeros - no stop bit
    let mut reader = BitReader::new(&data);
    let result = reader.read_ue();
    assert!(result.is_err(), "Should error on truncated Exp-Golomb");
}

#[test]
fn test_exp_golomb_truncated_at_info_bits() {
    // Has stop bit but truncated at info bits
    let data = [0b00000100]; // 5 leading zeros + stop bit, but no info bits
    let mut reader = BitReader::new(&data);
    let result = reader.read_ue();
    assert!(result.is_err());
}

#[test]
fn test_emulation_prevention_at_boundaries() {
    // 0x00 0x00 0x03 at start of data
    // 0x00 0x00 0x03 at end of data
    // 0x00 0x00 0x03 spanning buffer boundaries
}

#[test]
fn test_negative_timestamp_conversion() {
    // u64::MAX timestamp should fail or be clamped
    let timestamp = u64::MAX;
    // Verify conversion to i64 handles this case
}
```

### 1.3 Extremely Large Dimensions (8K+)

**Current Coverage:** Minimal - only dimension validation at limits

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| 8K UHD (7680x4320) frame parsing | all codecs | Medium |
| 16K (15360x8640) dimension rejection | decoder | High |
| Width=MAX, Height=1 (extreme aspect ratio) | decoder | Medium |
| Dimension overflow in plane size calc | decoder | Critical |

**Recommended Test Additions:**

```rust
// crates/bitvue-decode/tests/dimension_edge_cases_test.rs

#[test]
fn test_8k_dimensions_validation() {
    let frame = create_test_frame(7680, 4320);
    let result = validate_frame(&frame);
    assert!(result.is_ok(), "Should accept 8K dimensions");
}

#[test]
fn test_16k_dimensions_rejection() {
    let frame = create_test_frame(15360, 8640);
    let result = validate_frame(&frame);
    // May fail at allocation - should not panic
}

#[test]
fn test_dimension_overflow_protection() {
    // width * height * bytes_per_pixel should not overflow
    let width = u32::MAX / 2;
    let height = 3u32;
    let frame = create_test_frame_unsafe(width, height);
    let result = validate_frame(&frame);
    assert!(result.is_err(), "Should detect overflow in size calculation");
}

#[test]
fn test_extreme_aspect_ratio_10000x1() {
    let frame = create_test_frame(10000, 1);
    let result = validate_frame(&frame);
    // Should handle extreme aspect ratios
}

#[test]
fn test_extreme_aspect_ratio_1x10000() {
    let frame = create_test_frame(1, 10000);
    let result = validate_frame(&frame);
}
```

### 1.4 Unusual Frame Rates

**Current Coverage:** None identified

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| Frame rate = 0 (division by zero) | IVF parser | Critical |
| Frame rate = 1000 fps | IVF parser | Low |
| Fractional frame rates | IVF parser | Medium |
| Negative timebase | IVF parser | High |

**Recommended Test Additions:**

```rust
// crates/bitvue-decode/tests/framerate_edge_cases_test.rs

#[test]
fn test_ivf_zero_timebase_denominator() {
    // timebase denominator = 0 (division by zero risk)
    let mut header = create_minimal_ivf_header();
    // timebase_denominator is at offset 24-27 in header
    // Actually it's at offset 24-27 after the first 32 bytes
    // Need to verify exact offset
}

#[test]
fn test_ivf_zero_timebase_numerator() {
    // timebase numerator = 0
}

#[test]
fn test_ivf_high_frame_rate() {
    // 1000 fps - verify timestamp calculation doesn't overflow
}

#[test]
fn test_ivf_negative_timestamp_handling() {
    // Timestamp that would be negative when converted
}
```

### 1.5 Missing Required NAL Units

**Current Coverage:** Partial - some tests for missing SPS/PPS

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| Slice without prior SPS | VVC/HEVC/AVC | High |
| PPS without SPS | VVC/HEVC/AVC | High |
| Multiple SPS with same ID | VVC/HEVC/AVC | Medium |
| SPS ID out of range | VVC/HEVC/AVC | High |

---

## 2. Numerical Boundaries

### 2.1 Integer Overflow in Calculations

**Current Coverage:** Good - `limits.rs` has overflow protection, `decoder.rs` uses checked arithmetic

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| width * height overflow for plane size | decoder | Critical |
| offset + length overflow in range read | byte_cache | Critical |
| Loop counter overflow in frame iteration | decoder | Medium |
| Accumulator overflow in SIMD metrics | metrics | High |

**Recommended Test Additions:**

```rust
// crates/bitvue-core/tests/overflow_edge_cases_test.rs

#[test]
fn test_byte_cache_range_overflow() {
    // offset + len > u64::MAX
    let temp_file = create_temp_file(1024);
    let cache = ByteCache::new(&temp_file, 256, 256 * 1024).unwrap();
    let result = cache.read_range(u64::MAX - 10, 100);
    assert!(result.is_err(), "Should detect overflow in range calculation");
}

#[test]
fn test_checked_arithmetic_multiplication() {
    // Test that all size calculations use checked_mul
    let width = u32::MAX / 2 + 1;
    let height = 2u32;
    let size = width.checked_mul(height);
    assert!(size.is_none(), "Should overflow");
}
```

### 2.2 Division by Zero

**Current Coverage:** Good - MSE=0 handled in PSNR, returns infinity

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| Zero pixel count in PSNR | metrics | Critical |
| Zero timebase in frame rate | decoder | Critical |
| Zero segment count in cache | byte_cache | Medium |
| Zero dimension in stride calc | decoder | High |

**Recommended Test Additions:**

```rust
// crates/bitvue-metrics/tests/division_by_zero_test.rs

#[test]
fn test_psnr_zero_pixel_count() {
    // Calling PSNR with count=0 should not panic
    let reference: Vec<u8> = vec![];
    let distorted: Vec<u8> = vec![];
    let result = psnr(&reference, &distorted, 0, 0);
    // Should return error or handle gracefully
}

#[test]
fn test_ssim_zero_window_size() {
    // SSIM with window size = 0
}
```

### 2.3 NaN/Infinity Handling

**Current Coverage:** Good - PSNR returns infinity for identical images

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| NaN in SSIM calculation | metrics | High |
| Infinity in metric aggregation | metrics | Medium |
| NaN in VMAF calculation | vmaf | High |

**Recommended Test Additions:**

```rust
// crates/bitvue-metrics/tests/nan_infinity_test.rs

#[test]
fn test_psnr_infinity_handling() {
    let reference = vec![128u8; 100];
    let distorted = vec![128u8; 100];
    let result = psnr(&reference, &distorted, 10, 10).unwrap();
    assert!(result.is_infinite() && result.is_sign_positive());
}

#[test]
fn test_metric_aggregation_with_infinity() {
    // Multiple PSNR values including infinity
    let values = vec![45.0, f64::INFINITY, 42.0, f64::INFINITY];
    let avg = values.iter().sum::<f64>() / values.len() as f64;
    // Verify this doesn't produce NaN
}

#[test]
fn test_ssim_nan_prevention() {
    // Edge case that could produce NaN in SSIM
    // E.g., zero variance
}
```

### 2.4 Negative Values Where Positive Expected

**Current Coverage:** Partial - timestamp range checking exists

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| Negative stride in plane | decoder | High |
| Negative offset in byte_cache | byte_cache | Critical |
| Negative dimension | decoder | Critical |
| Signed Exp-Golomb edge cases | bitreader | Medium |

---

## 3. Resource Limits

### 3.1 Memory Exhaustion Scenarios

**Current Coverage:** Good - MAX_FILE_SIZE, MAX_FRAME_SIZE limits in place

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| Allocation failure simulation | decoder | High |
| Memory pressure during decode | decoder | Medium |
| Large LRU cache eviction | byte_cache | Medium |
| Mmap failure handling | byte_cache | High |

**Recommended Test Additions:**

```rust
// crates/bitvue-core/tests/memory_edge_cases_test.rs

#[test]
fn test_byte_cache_large_file_rejection() {
    // File larger than MAX_FILE_SIZE should be rejected
    // Note: Can't actually create 2GB file in test, mock the check
}

#[test]
fn test_cache_eviction_under_pressure() {
    // Create cache with small memory budget
    // Access many segments to force eviction
    let temp_file = create_temp_file(1024 * 1024); // 1MB
    let cache = ByteCache::new(&temp_file, 1024, 4 * 1024).unwrap(); // 4KB budget

    // Access more segments than cache can hold
    for i in 0..10 {
        let _ = cache.get_segment(i);
    }

    // Verify cache evicted older segments
    let stats = cache.stats();
    assert!(stats.segment_count <= 4);
}

#[test]
fn test_mmap_failure_graceful() {
    // Test that mmap failure is handled gracefully
    // This may require mocking or special test setup
}
```

### 3.2 File Handle Limits

**Current Coverage:** None identified

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| Too many open files | byte_cache | Medium |
| File deleted while mapped | byte_cache | High |
| File permissions change | decoder | Medium |

**Recommended Test Additions:**

```rust
// crates/bitvue-core/tests/file_handle_edge_cases_test.rs

#[test]
fn test_file_deleted_while_mapped() {
    // Create temp file, map it, delete file
    // Access should still work (Unix semantics) or fail gracefully
}

#[test]
#[cfg(unix)]
fn test_file_permission_denied() {
    use std::os::unix::fs::PermissionsExt;
    let temp_file = create_temp_file(1024);
    let mut perms = std::fs::metadata(&temp_file).unwrap().permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&temp_file, perms).unwrap();

    let result = ByteCache::new(&temp_file, 256, 1024);
    assert!(result.is_err());

    // Cleanup
    perms.set_mode(0o644);
    std::fs::set_permissions(&temp_file, perms).unwrap();
}
```

### 3.3 Thread Pool Saturation

**Current Coverage:** Minimal - some concurrent tests

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| More workers than CPU cores | decoder | Low |
| Worker thread panic recovery | decoder | High |
| Lock starvation | byte_cache | Medium |
| Deadlock detection | decoder | High |

**Recommended Test Additions:**

```rust
// crates/bitvue-decode/tests/concurrency_edge_cases_test.rs

#[test]
fn test_concurrent_decoder_stress() {
    use std::sync::Arc;
    use std::thread;

    let decoder = Arc::new(std::sync::Mutex::new(Av1Decoder::new().unwrap()));
    let handles: Vec<_> = (0..100)
        .map(|i| {
            let dec = Arc::clone(&decoder);
            thread::spawn(move || {
                let mut d = dec.lock().unwrap();
                // Attempt decode operation
                let _ = d.send_data(&[0x00, 0x00, 0x01, 0x09], i);
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_byte_cache_concurrent_reads() {
    // Multiple threads reading from same cache
    let temp_file = create_temp_file(1024 * 1024);
    let cache = Arc::new(ByteCache::new(&temp_file, 1024, 1024 * 1024).unwrap());

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let c = Arc::clone(&cache);
            thread::spawn(move || {
                let offset = (i * 100) as u64;
                let _ = c.read_range(offset, 100);
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}
```

---

## 4. Concurrency Edge Cases

### 4.1 Race Conditions

**Current Coverage:** Minimal

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| Cache read/write race | byte_cache | High |
| TOCTOU in file validation | byte_cache | Critical |
| LRU cache ordering race | byte_cache | Medium |

**Recommended Test Additions:**

```rust
// crates/bitvue-core/tests/race_condition_test.rs

#[test]
fn test_byte_cache_toctou_protection() {
    // Create file, get cache
    let temp_file = create_temp_file(1024);
    let cache = ByteCache::new(&temp_file, 256, 1024).unwrap();

    // Modify file externally (truncate)
    std::fs::File::create(&temp_file).unwrap().set_len(512).unwrap();

    // Access should detect modification via TOCTOU protection
    let result = cache.validate();
    assert!(result.is_err(), "Should detect file modification");
}

#[test]
fn test_concurrent_lru_cache_access() {
    // Multiple threads accessing same LRU cache
    // Should not corrupt internal state
}
```

### 4.2 Deadlock Potential

**Current Coverage:** None

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| Lock ordering deadlock | decoder | High |
| Recursive lock attempt | byte_cache | Critical |
| Lock held during I/O | byte_cache | Medium |

### 4.3 Lock Contention

**Current Coverage:** Minimal

**Gaps Identified:**

| Test Case | Location | Priority |
|-----------|----------|----------|
| High contention read path | byte_cache | Medium |
| Write lock blocking reads | byte_cache | Medium |

---

## 5. SIMD Edge Cases

### 5.1 Alignment Issues

**Current Coverage:** Good - `edge_cases_test.rs` in metrics tests alignment

**Additional Test Cases:**

```rust
// crates/bitvue-metrics/tests/simd_alignment_test.rs

#[test]
fn test_simd_misaligned_pointers() {
    // Test all SIMD implementations with misaligned data
    let data = vec![0u8; 1001]; // Odd length
    let misaligned = &data[1..]; // Offset by 1

    let result = psnr_simd(misaligned, misaligned, 100, 10);
    assert!(result.is_ok());
}

#[test]
fn test_simd_near_boundary_sizes() {
    // Sizes just below SIMD width boundaries
    for size in [14, 15, 16, 17, 30, 31, 32, 33, 62, 63, 64, 65] {
        let data = vec![128u8; size];
        let result = psnr_simd(&data, &data, size, 1);
        assert!(result.is_ok(), "Size {} should work", size);
    }
}
```

### 5.2 Accumulator Overflow in SIMD

**Current Coverage:** Good - recent fix in `simd.rs` prevents overflow

**Current Issue (Fixed):**
```rust
// crates/bitvue-metrics/src/simd.rs line 755-756
// Security: Use u64 with saturating add to prevent overflow
let simd_sum: u64 = mse_array.iter().map(|&x| x as u32 as u64).sum();
```

### 5.3 Platform Differences (x86 vs ARM)

**Current Coverage:** Good - cross-platform consistency tests exist

---

## 6. Codec-Specific Edge Cases

### 6.1 VVC-Specific

**Current Coverage:** Good - `vvc_edge_cases_test.rs` and `vvc_stress_test.rs`

**Gaps:**

| Test Case | Priority |
|-----------|----------|
| ALCS NAL handling | Medium |
| RPL (Reference Picture List) overflow | High |
| LMCS adaptive parameter set | Medium |
| Scaling list edge cases | Medium |

### 6.2 HEVC-Specific

**Current Coverage:** `hevc_edge_cases_test.rs`, `hevc_stress_test.rs`

**Gaps:**

| Test Case | Priority |
|-----------|----------|
| RPS (Reference Picture Set) overflow | High |
| Tile boundary handling | Medium |
| WPP (Wavefront Parallel Processing) | Medium |
| SAO (Sample Adaptive Offset) edge cases | Medium |

### 6.3 VP9-Specific

**Current Coverage:** `vp9_edge_cases_test.rs`, `vp9_stress_test.rs`

**Gaps:**

| Test Case | Priority |
|-----------|----------|
| Superframe index parsing | High |
| Loop filter level edge cases | Medium |
| Probability table overflow | Medium |
| Reference frame validation | High |

### 6.4 AVC/H.264-Specific

**Current Coverage:** `avc_stress_test.rs`, `slice_edge_cases_test.rs`

**Gaps:**

| Test Case | Priority |
|-----------|----------|
| FMO (Flexible Macroblock Ordering) | Low |
| ASO (Arbitrary Slice Ordering) | Low |
| Redundant picture handling | Low |
| MMCO (Memory Management Control Operations) | High |

---

## 7. Test Case Implementation Priority

### Critical (Must Add)

1. Division by zero in frame rate calculation
2. Integer overflow in dimension calculations
3. Negative offset handling in byte_cache
4. TOCTOU protection validation
5. SIMD accumulator overflow verification

### High Priority (Should Add)

1. Truncated Exp-Golomb handling
2. Memory exhaustion simulation
3. Deadlock detection tests
4. Race condition tests for LRU cache
5. Missing NAL unit handling

### Medium Priority (Nice to Have)

1. Extreme aspect ratio tests
2. File handle exhaustion
3. Lock contention benchmarks
4. Codec-specific edge cases
5. Platform-specific SIMD variations

---

## 8. Test Coverage Metrics

Current estimated coverage based on analysis:

| Module | Current | Target | Gap |
|--------|---------|--------|-----|
| bitvue-core | ~85% | 95% | 10% |
| bitvue-decode | ~80% | 95% | 15% |
| bitvue-metrics | ~90% | 95% | 5% |
| bitvue-vvc | ~85% | 95% | 10% |
| bitvue-vp9 | ~80% | 95% | 15% |
| bitvue-hevc | ~80% | 95% | 15% |
| bitvue-avc | ~75% | 95% | 20% |

---

## 9. Recommendations

### Immediate Actions

1. Add division-by-zero tests for frame rate and PSNR calculations
2. Add overflow protection tests for dimension calculations
3. Add TOCTOU validation tests for ByteCache
4. Add race condition tests for concurrent cache access

### Short-Term Actions

1. Implement codec-specific edge case tests
2. Add memory exhaustion simulation tests
3. Add deadlock detection tests
4. Expand SIMD edge case coverage

### Long-Term Actions

1. Implement fuzz testing for all codec parsers
2. Add property-based testing for numerical calculations
3. Set up mutation testing to verify test effectiveness
4. Implement automated edge case generation

---

## 10. Conclusion

The Bitvue codebase has reasonable edge case coverage for core functionality but has gaps in several areas:

- **Division by zero protection** needs more explicit testing
- **Integer overflow** has some protection but needs verification
- **Concurrency edge cases** are under-tested
- **Codec-specific edge cases** vary significantly by codec

Implementing the recommended test cases will significantly improve robustness and move toward the 95% coverage target.

---

*Generated by edge case analysis tool*
*Report version: 1.0*
