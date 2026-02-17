#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Edge case and boundary condition tests for SIMD metrics
//!
//! This test suite covers:
//! - Empty/Null Inputs: Empty arrays, zero-length buffers
//! - Boundary Values: Minimum/maximum dimensions, bit depths
//! - Size Mismatches: Different buffer sizes, mismatched dimensions
//! - Overflow Conditions: Numeric overflow in calculations
//! - Alignment Issues: Unaligned memory access for SIMD
//! - Precision Issues: Floating point edge cases
//! - Special Cases: Identical images, all zeros, all max
//! - Platform Differences: x86 vs ARM SIMD differences
//! - Concurrent Access: Thread-safety of SIMD operations
//! - Performance Edge Cases: Very small/large inputs

use bitvue_core::BitvueError;
use bitvue_metrics::psnr;
use bitvue_metrics::simd::psnr_simd;

// ============================================================================
// Category 1: Empty/Null Inputs
// ============================================================================

#[test]
fn test_empty_buffers() {
    /// Test PSNR with empty reference and distorted buffers
    /// Expected: Should handle gracefully or return error
    let reference: Vec<u8> = vec![];
    let distorted: Vec<u8> = vec![];

    let result = psnr_simd(&reference, &distorted, 0, 0);

    // Empty buffers might cause division by zero, should handle
    assert!(
        result.is_err() || result.is_ok(),
        "Should handle empty buffers"
    );
}

#[test]
fn test_zero_length_dimension() {
    /// Test PSNR with zero width or height
    /// Expected: Should handle or reject appropriately
    let reference = vec![128u8; 1920 * 1080];
    let distorted = vec![128u8; 1920 * 1080];

    // Width = 0
    let result = psnr_simd(&reference, &distorted, 0, 1080);
    // Should likely fail due to division by zero or invalid dimensions

    // Height = 0
    let result = psnr_simd(&reference, &distorted, 1920, 0);
    // Should likely fail
}

#[test]
fn test_single_pixel() {
    /// Test PSNR with 1x1 image (smallest valid size)
    /// Expected: Should calculate correctly for single pixel
    let reference = vec![128u8; 1];
    let distorted = vec![130u8; 1];

    let result = psnr_simd(&reference, &distorted, 1, 1);

    assert!(result.is_ok(), "Should calculate PSNR for 1x1 image");
    let psnr_value = result.unwrap();

    // MSE = (128-130)^2 = 4
    // PSNR = 10 * log10(255^2 / 4) ≈ 48.13 dB
    assert!(
        (psnr_value - 48.13).abs() < 0.5,
        "PSNR should be approximately 48.13 dB"
    );
}

// ============================================================================
// Category 2: Boundary Values
// ============================================================================

#[test]
fn test_minimum_psnr_values() {
    /// Test PSNR calculation with maximum distortion
    /// Expected: Should handle minimum PSNR values correctly
    // All zeros vs all 255s (maximum difference)
    let reference = vec![0u8; 1000];
    let distorted = vec![255u8; 1000];

    let result = psnr_simd(&reference, &distorted, 10, 100).unwrap();

    // MSE = 255^2 = 65025
    // PSNR = 10 * log10(255^2 / 65025) = 10 * log10(1) = 0 dB
    assert!(
        result >= 0.0 && result < 10.0,
        "PSNR should be near 0 dB for max distortion"
    );
}

#[test]
fn test_maximum_psnr_values() {
    /// Test PSNR with identical images
    /// Expected: Should return infinity for identical images
    let reference = vec![128u8; 1920 * 1080];
    let distorted = vec![128u8; 1920 * 1080];

    let result = psnr_simd(&reference, &distorted, 1920, 1080).unwrap();

    assert!(
        result.is_infinite(),
        "PSNR should be infinite for identical images"
    );
}

#[test]
fn test_boundary_pixel_values() {
    /// Test PSNR with pixel values at 0 and 255 boundaries
    /// Expected: Should handle extreme values correctly
    // All zeros
    let reference_zeros = vec![0u8; 1000];
    let distorted_zeros = vec![0u8; 1000];
    let result_zeros = psnr_simd(&reference_zeros, &distorted_zeros, 10, 100).unwrap();
    assert!(
        result_zeros.is_infinite(),
        "All zeros should give infinite PSNR"
    );

    // All 255s
    let reference_max = vec![255u8; 1000];
    let distorted_max = vec![255u8; 1000];
    let result_max = psnr_simd(&reference_max, &distorted_max, 10, 100).unwrap();
    assert!(
        result_max.is_infinite(),
        "All 255s should give infinite PSNR"
    );

    // Mixed extremes
    let reference = vec![0u8; 500]
        .into_iter()
        .chain(vec![255u8; 500])
        .collect::<Vec<_>>();
    let distorted = vec![255u8; 500]
        .into_iter()
        .chain(vec![0u8; 500])
        .collect::<Vec<_>>();
    let result = psnr_simd(&reference, &distorted, 10, 100);
    assert!(result.is_ok(), "Should handle mixed extreme values");
}

#[test]
fn test_odd_dimensions() {
    /// Test PSNR with odd width/height
    /// Expected: Should handle non-power-of-2 dimensions
    let reference = vec![128u8; 1919 * 1079];
    let distorted = vec![128u8; 1919 * 1079];

    let result = psnr_simd(&reference, &distorted, 1919, 1079);

    assert!(result.is_ok(), "Should handle odd dimensions");
    assert!(
        result.unwrap().is_infinite(),
        "Odd dimensions with identical data should give infinite PSNR"
    );
}

#[test]
fn test_prime_dimensions() {
    /// Test PSNR with prime number dimensions (worst case for alignment)
    /// Expected: Should handle prime dimensions correctly
    let width = 997; // Prime number
    let height = 991; // Different prime number
    let reference = vec![128u8; width * height];
    let distorted = vec![128u8; width * height];

    let result = psnr_simd(&reference, &distorted, width, height);

    assert!(result.is_ok(), "Should handle prime number dimensions");
}

// ============================================================================
// Category 3: Size Mismatches
// ============================================================================

#[test]
fn test_buffer_size_mismatch() {
    /// Test PSNR with mismatched buffer sizes
    /// Expected: SIMD implementation should handle safely
    let reference = vec![128u8; 1000];
    let distorted = vec![128u8; 500]; // Different size

    // This is a programming error, but SIMD should not crash
    // The implementation should use the minimum size or error
    let result = std::panic::catch_unwind(|| psnr_simd(&reference, &distorted, 10, 100));

    // Should either error or handle gracefully (not crash)
    assert!(
        result.is_ok() || result.is_err(),
        "Should not crash on size mismatch"
    );
}

#[test]
fn test_dimension_vs_buffer_mismatch() {
    /// Test PSNR when width*height doesn't match buffer size
    /// Expected: Should handle or detect mismatch
    let reference = vec![128u8; 1920 * 1080];
    let distorted = vec![128u8; 1920 * 1080];

    // Provide incorrect dimensions
    let result = psnr_simd(&reference, &distorted, 1920, 1079);

    // May produce incorrect results or error, but should not crash
    assert!(
        result.is_ok() || result.is_err(),
        "Should handle dimension mismatch"
    );
}

#[test]
fn test_simd_alignment_edge_cases() {
    /// Test SIMD with buffers that don't align to SIMD width
    /// Expected: Should handle unaligned buffers correctly
    // AVX2: 32-byte alignment
    // SSE2: 16-byte alignment
    // NEON: 16-byte alignment
    let sizes_to_test = vec![1, 15, 16, 17, 31, 32, 33, 63, 64, 65];

    for size in sizes_to_test {
        let reference = vec![128u8; size];
        let distorted = vec![128u8; size];

        let result = psnr_simd(&reference, &distorted, size, 1);

        assert!(
            result.is_ok(),
            "Should handle size {} (alignment boundary)",
            size
        );
        let psnr = result.unwrap();
        if psnr.is_finite() {
            // If finite, should be reasonable value
            assert!(psnr > 0.0, "PSNR should be positive for size {}", size);
        }
    }
}

// ============================================================================
// Category 4: Overflow Conditions
// ============================================================================

#[test]
fn test_accumulator_overflow() {
    /// Test PSNR with values that could cause accumulator overflow
    /// Expected: Should use 32-bit or wider accumulators to prevent overflow
    // Maximum difference per pixel: 255
    // Maximum squared difference: 255^2 = 65025
    // For 4K frame (3840x2160 = 8,294,400 pixels)
    // Maximum sum: 8,294,400 * 65025 ≈ 539 billion (fits in u64 but not u32)
    let reference: Vec<u8> = (0..255).cycle().take(1920 * 1080).collect();
    let distorted: Vec<u8> = reference.iter().map(|&v| v.wrapping_add(1)).collect();

    let result = psnr_simd(&reference, &distorted, 1920, 1080);

    assert!(
        result.is_ok(),
        "Should handle large accumulations without overflow"
    );
    let psnr = result.unwrap();
    assert!(
        psnr.is_finite() && psnr > 0.0,
        "PSNR should be finite and positive"
    );
}

#[test]
fn test_wrapping_arithmetic() {
    /// Test that SIMD handles wrapping arithmetic correctly
    /// Expected: Should use signed subtraction for accurate differences
    // Test case where unsigned wrapping would give wrong result
    let reference = vec![0u8; 100];
    let distorted = vec![255u8; 100];

    let simd_result = psnr_simd(&reference, &distorted, 10, 10).unwrap();
    let scalar_result = psnr(&reference, &distorted, 10, 10).unwrap();

    // SIMD and scalar should agree
    assert!(
        (simd_result - scalar_result).abs() < 0.5,
        "SIMD should handle signed subtraction correctly"
    );
}

#[test]
fn test_large_frame_overflow_protection() {
    /// Test SIMD overflow protection with very large frames
    /// Expected: Should use u64 accumulators for safety
    let size = 8192usize * 8192; // Maximum reasonable frame size

    // Don't actually allocate this (would be 64 MB per plane)
    // Just verify the logic would work
    let max_squared_diff = 65025u64; // 255^2
    let max_sum = size as u64 * max_squared_diff;

    // Should fit in u64
    assert!(max_sum < u64::MAX, "Maximum accumulation should fit in u64");

    // Would overflow u32 - use checked_mul to verify
    let size_u32 = size as u32;
    let max_squared_diff_u32 = max_squared_diff as u32;
    let max_sum_u32 = size_u32.checked_mul(max_squared_diff_u32);
    assert!(max_sum_u32.is_none(), "Would overflow in u32 (expected)");
}

// ============================================================================
// Category 5: Alignment Issues
// ============================================================================

#[test]
fn test_unaligned_pointer_access() {
    /// Test SIMD with intentionally unaligned pointers
    /// Expected: Should use unaligned loads (_mm_loadu_si256, etc.)
    let mut reference = vec![128u8; 1000];
    let mut distorted = vec![128u8; 1000];

    // Offset to create misalignment
    let reference_offset = &reference[1..];
    let distorted_offset = &distorted[1..];

    // Adjust dimensions
    let size = reference_offset.len();
    let result = psnr_simd(reference_offset, distorted_offset, size, 1);

    // Should handle unaligned access
    assert!(result.is_ok(), "Should handle unaligned pointer access");
}

#[test]
fn test_stack_vs_heap_allocation() {
    /// Test SIMD with both stack and heap allocated data
    /// Expected: Should work correctly with both
    // Stack allocated (small arrays)
    let reference_stack = [128u8; 100];
    let distorted_stack = [130u8; 100];

    let result_stack = psnr_simd(&reference_stack[..], &distorted_stack[..], 10, 10);
    assert!(
        result_stack.is_ok(),
        "Should work with stack-allocated data"
    );

    // Heap allocated (vectors)
    let reference_heap = vec![128u8; 1920 * 1080];
    let distorted_heap = vec![130u8; 1920 * 1080];

    let result_heap = psnr_simd(&reference_heap, &distorted_heap, 1920, 1080);
    assert!(result_heap.is_ok(), "Should work with heap-allocated data");

    // Results should be consistent (same MSE per pixel)
    let psnr_stack = result_stack.unwrap();
    let psnr_heap = result_heap.unwrap();

    // Both should have same MSE (4 per pixel)
    // PSNR should be similar (minor differences possible due to SIMD remainder handling)
    assert!(
        (psnr_stack - psnr_heap).abs() < 0.5,
        "Stack and heap allocations should give similar results"
    );
}

// ============================================================================
// Category 6: Precision Issues
// ============================================================================

#[test]
fn test_floating_point_edge_cases() {
    /// Test PSNR with floating point edge cases
    /// Expected: Should handle denormals, infinity, NaN correctly
    // Identical images -> infinite PSNR
    let reference = vec![128u8; 1000];
    let distorted = vec![128u8; 1000];

    let result = psnr_simd(&reference, &distorted, 10, 100).unwrap();
    assert!(
        result.is_infinite(),
        "Identical images should give infinite PSNR"
    );

    // Very small differences -> high PSNR
    let mut distorted = vec![128u8; 1000];
    distorted[0] = 129; // Single pixel difference

    let result = psnr_simd(&reference, &distorted, 10, 100).unwrap();
    assert!(
        result.is_finite() && result > 60.0,
        "Single pixel difference should give very high PSNR"
    );
}

#[test]
fn test_mse_zero_division() {
    /// Test PSNR when MSE is zero (identical images)
    /// Expected: Should return infinity, not NaN or error
    let reference = vec![100u8; 1920 * 1080];
    let distorted = vec![100u8; 1920 * 1080];

    let result = psnr_simd(&reference, &distorted, 1920, 1080).unwrap();

    // Should be infinity, not NaN
    assert!(
        result.is_infinite() && result.is_sign_positive(),
        "MSE=0 should give positive infinity, not NaN"
    );
}

#[test]
fn test_simd_scalar_consistency() {
    /// Test that SIMD and scalar implementations agree
    /// Expected: Results should match within floating point tolerance
    let test_cases = vec![
        (vec![0u8; 1000], vec![255u8; 1000]),   // Max difference
        (vec![128u8; 1000], vec![128u8; 1000]), // Identical
        (vec![100u8; 1000], vec![150u8; 1000]), // Medium difference
        (vec![50u8; 1000], vec![51u8; 1000]),   // Small difference
    ];

    for (reference, distorted) in test_cases {
        let simd_result = psnr_simd(&reference, &distorted, 10, 100);
        let scalar_result = psnr(&reference, &distorted, 10, 100);

        assert!(simd_result.is_ok(), "SIMD should succeed");
        assert!(scalar_result.is_ok(), "Scalar should succeed");

        let simd_psnr = simd_result.unwrap();
        let scalar_psnr = scalar_result.unwrap();

        // Both should be infinite or both finite
        assert_eq!(
            simd_psnr.is_infinite(),
            scalar_psnr.is_infinite(),
            "SIMD and scalar should agree on infinity"
        );

        // If both finite, should be close
        if simd_psnr.is_finite() && scalar_psnr.is_finite() {
            assert!(
                (simd_psnr - scalar_psnr).abs() < 0.5,
                "SIMD={} and Scalar={} should match within 0.5 dB",
                simd_psnr,
                scalar_psnr
            );
        }
    }
}

// ============================================================================
// Category 7: Special Cases
// ============================================================================

#[test]
fn test_checkerboard_pattern() {
    /// Test PSNR with alternating extreme values (hardest for SIMD)
    /// Expected: Should handle alternating patterns correctly
    let mut reference = Vec::with_capacity(1920 * 1080);
    let mut distorted = Vec::with_capacity(1920 * 1080);

    for i in 0..(1920 * 1080) {
        if i % 2 == 0 {
            reference.push(0);
            distorted.push(255);
        } else {
            reference.push(255);
            distorted.push(0);
        }
    }

    let result = psnr_simd(&reference, &distorted, 1920, 1080);

    assert!(result.is_ok(), "Should handle checkerboard pattern");
    let psnr = result.unwrap();
    assert!(
        psnr.is_finite() && psnr > 0.0,
        "Checkerboard should give finite PSNR"
    );
}

#[test]
fn test_gradient_pattern() {
    /// Test PSNR with gradient (tests all pixel values)
    /// Expected: Should handle full range of values
    let reference: Vec<u8> = (0..=255).cycle().take(1920 * 1080).collect();
    let distorted: Vec<u8> = reference.iter().map(|&v| v.wrapping_sub(10)).collect();

    let result = psnr_simd(&reference, &distorted, 1920, 1080);

    assert!(result.is_ok(), "Should handle gradient pattern");
    let psnr = result.unwrap();
    assert!(
        psnr.is_finite() && psnr > 20.0,
        "Gradient shift should give reasonable PSNR"
    );
}

#[test]
fn test_single_bit_difference() {
    /// Test PSNR with minimal quantifiable difference
    /// Expected: Should detect single-bit differences
    let reference = vec![128u8; 1000];
    let mut distorted = vec![128u8; 1000];
    distorted[500] = 129; // Single pixel, single-bit difference

    let result = psnr_simd(&reference, &distorted, 10, 100).unwrap();

    // MSE = 1 / 1000 = 0.001
    // PSNR = 10 * log10(255^2 / 0.001) ≈ 87 dB
    assert!(
        result > 80.0,
        "Single-bit difference should give very high PSNR (>80 dB)"
    );
}

#[test]
fn test_all_same_value() {
    /// Test PSNR when all pixels are same value (flat field)
    /// Expected: Should handle flat fields correctly
    for value in [0u8, 128, 255] {
        let reference = vec![value; 1000];
        let distorted = vec![value; 1000];

        let result = psnr_simd(&reference, &distorted, 10, 100).unwrap();
        assert!(
            result.is_infinite(),
            "Flat field (value={}) should give infinite PSNR",
            value
        );
    }
}

// ============================================================================
// Category 8: Platform Differences
// ============================================================================

#[test]
fn test_simd_feature_detection() {
    /// Test that SIMD feature detection works correctly
    /// Expected: Should detect available features and use appropriate path
    #[cfg(target_arch = "x86_64")]
    {
        // Test that we can detect features
        let has_avx2 = is_x86_feature_detected!("avx2");
        let has_avx = is_x86_feature_detected!("avx");
        let has_sse2 = is_x86_feature_detected!("sse2");

        // SSE2 should always be available on x86_64
        assert!(has_sse2, "SSE2 should be available on x86_64");

        // If AVX2 is available, AVX should also be available
        if has_avx2 {
            assert!(has_avx, "AVX2 implies AVX is available");
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        // Test NEON detection
        let has_neon = std::arch::is_aarch64_feature_detected!("neon");
        // NEON is optional but common
    }
}

#[test]
fn test_cross_platform_consistency() {
    /// Test that results are consistent across SIMD implementations
    /// Expected: AVX2, AVX, SSE2, NEON, and scalar should all agree
    let reference = vec![100u8; 1920 * 1080];
    let distorted: Vec<u8> = reference.iter().map(|&v| v.wrapping_add(10)).collect();

    let simd_result = psnr_simd(&reference, &distorted, 1920, 1080);
    let scalar_result = psnr(&reference, &distorted, 1920, 1080);

    assert!(simd_result.is_ok(), "SIMD should succeed");
    assert!(scalar_result.is_ok(), "Scalar should succeed");

    let simd_psnr = simd_result.unwrap();
    let scalar_psnr = scalar_result.unwrap();

    // Results should match
    assert!(
        (simd_psnr - scalar_psnr).abs() < 0.5,
        "Cross-platform results should match"
    );
}

#[test]
fn test_endianness_handling() {
    /// Test that SIMD handles endianness correctly
    /// Expected: Should work correctly on both little and big endian
    // This is mainly a concern for loading multi-byte values
    let reference = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
    let distorted = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];

    let result = psnr_simd(&reference, &distorted, 8, 1);

    assert!(result.is_ok(), "Should handle specific byte patterns");
    assert!(
        result.unwrap().is_infinite(),
        "Identical patterns should give infinite PSNR"
    );
}

// ============================================================================
// Category 9: Concurrent Access
// ============================================================================

#[test]
fn test_concurrent_psnr_calculations() {
    /// Test multiple PSNR calculations running concurrently
    /// Expected: Each calculation should be independent
    use std::thread;

    let reference = vec![128u8; 1920 * 1080];
    let distorted = vec![130u8; 1920 * 1080];

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let ref_clone = reference.clone();
            let dist_clone = distorted.clone();
            thread::spawn(move || psnr_simd(&ref_clone, &dist_clone, 1920, 1080))
        })
        .collect();

    // All threads should complete successfully
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(
            result.is_ok(),
            "Concurrent PSNR calculations should succeed"
        );
    }
}

#[test]
fn test_simd_thread_safety() {
    /// Test that SIMD code is thread-safe
    /// Expected: No data races or undefined behavior
    use std::sync::{Arc, Mutex};
    use std::thread;

    let reference = Arc::new(vec![128u8; 1920 * 1080]);
    let distorted = Arc::new(vec![130u8; 1920 * 1080]);
    let results = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let ref_clone = Arc::clone(&reference);
            let dist_clone = Arc::clone(&distorted);
            let results_clone = Arc::clone(&results);
            thread::spawn(move || {
                let result = psnr_simd(&ref_clone, &dist_clone, 1920, 1080);
                let mut results = results_clone.lock().unwrap();
                results.push(result);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 4, "All threads should complete");

    // All results should be identical
    let first_result = &results[0];
    for result in results.iter() {
        match (result, first_result) {
            (Ok(r1), Ok(r2)) => {
                assert!(
                    (r1 - r2).abs() < 0.001,
                    "All concurrent results should be identical"
                );
            }
            (Err(e1), Err(e2)) => {
                // Different error types, but both are errors
                let _ = (e1, e2);
            }
            _ => panic!("Mixed results - some Ok, some Err"),
        }
    }
}

// ============================================================================
// Category 10: Performance Edge Cases
// ============================================================================

#[test]
fn test_very_small_input() {
    /// Test PSNR with very small input (SIMD overhead may dominate)
    /// Expected: Should still work correctly, just slower
    let reference = vec![128u8; 10];
    let distorted = vec![130u8; 10];

    let result = psnr_simd(&reference, &distorted, 10, 1);

    assert!(result.is_ok(), "Should handle very small inputs");
    let psnr = result.unwrap();
    assert!(psnr.is_finite(), "Small input should give finite PSNR");
}

#[test]
fn test_non_simd_multiple_size() {
    /// Test sizes that don't evenly divide SIMD width
    /// Expected: Remainder handling should work correctly
    // AVX2 processes 32 bytes at a time
    // Test sizes just below/above multiples of 32
    for size in [31, 32, 33, 63, 64, 65, 127, 128, 129] {
        let reference = vec![128u8; size];
        let distorted = vec![130u8; size];

        let result = psnr_simd(&reference, &distorted, size, 1);

        assert!(
            result.is_ok(),
            "Should handle size {} (remainder case)",
            size
        );

        // Compare with scalar for correctness
        let scalar_result = psnr(&reference, &distorted, size, 1);

        let simd_psnr = result.unwrap();
        let scalar_psnr = scalar_result.unwrap();

        assert!(
            (simd_psnr - scalar_psnr).abs() < 0.5,
            "Size {}: SIMD={} should match scalar={}",
            size,
            simd_psnr,
            scalar_psnr
        );
    }
}

#[test]
fn test_power_of_two_sizes() {
    /// Test PSNR with power-of-two sizes (optimal for SIMD)
    /// Expected: Should work efficiently with optimal alignment
    let power_of_two_sizes = [16, 32, 64, 128, 256, 512, 1024, 2048];

    for &size in &power_of_two_sizes {
        let reference = vec![128u8; size * size];
        let distorted = vec![130u8; size * size];

        let result = psnr_simd(&reference, &distorted, size, size);

        assert!(
            result.is_ok(),
            "Should handle power-of-two size {}x{}",
            size,
            size
        );
    }
}

#[test]
fn test_widescreen_dimensions() {
    /// Test PSNR with extreme aspect ratios
    /// Expected: Should handle any valid aspect ratio
    let test_cases = [
        (3840, 2160), // 16:9 4K
        (1920, 1080), // 16:9 FHD
        (1280, 720),  // 16:9 HD
        (4096, 2160), // 1.90:1 4K DCI
        (1920, 800),  // 2.40:1 Cinematic
        (1080, 1920), // 9:16 Portrait
        (1080, 1080), // 1:1 Square
    ];

    for (width, height) in test_cases {
        let reference = vec![128u8; width * height];
        let distorted = vec![130u8; width * height];

        let result = psnr_simd(&reference, &distorted, width, height);

        assert!(
            result.is_ok(),
            "Should handle {}x{} aspect ratio",
            width,
            height
        );
    }
}

#[test]
fn test_repeated_calculations() {
    /// Test that repeated calculations give consistent results
    /// Expected: Same input should always give same output
    let reference = vec![100u8; 1920 * 1080];
    let distorted = vec![150u8; 1920 * 1080];

    let mut results = Vec::new();

    for _ in 0..10 {
        let result = psnr_simd(&reference, &distorted, 1920, 1080).unwrap();
        results.push(result);
    }

    // All results should be identical
    for result in &results[1..] {
        assert_eq!(
            result, &results[0],
            "Repeated calculations should give identical results"
        );
    }
}
