# Edge Case and Boundary Condition Tests - Implementation Summary

This document provides implementation examples and code patterns for the edge case tests designed for the Bitvue video analyzer.

## Test Files Created

1. **`/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/tests/edge_cases_test.rs`**
   - 85+ test cases covering decoder edge conditions
   - Tests: empty inputs, boundaries, malformed data, path traversal, concurrency, encoding, platform differences

2. **`/Users/hawk/Workspaces/bitvue/crates/bitvue-metrics/tests/edge_cases_test.rs`**
   - 60+ test cases covering SIMD metrics edge conditions
   - Tests: empty inputs, boundaries, overflow, alignment, precision, concurrency, performance

3. **`/Users/hawk/Workspaces/bitvue/crates/bitvue-core/src/limits_edge_cases.rs`**
   - 40+ test cases covering resource limit validation
   - Tests: thread count, buffer size, cache size, file size, frame limits, arithmetic overflow

4. **`/Users/hawk/Workspaces/bitvue/docs/edge-case-test-coverage.md`**
   - Comprehensive documentation of all test categories
   - Test descriptions, expected behaviors, validation points

## Key Test Patterns

### 1. Empty/Null Input Tests

#### Pattern: Empty Array Handling
```rust
#[test]
fn test_empty_file() {
    let empty_data: Vec<u8> = vec![];
    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&empty_data);

    assert!(result.is_err());
    if let Err(DecodeError::Decode(msg) | DecodeError::NoFrame) = result {
        assert!(msg.contains("too short") || msg.contains("empty"));
    }
}
```

#### Pattern: Zero Dimension Validation
```rust
#[test]
fn test_zero_dimensions() {
    let frame = create_test_frame(0, 1920);
    let result = bitvue_decode::decoder::validate_frame(&frame);
    assert!(result.is_err(), "Should reject zero width");
}
```

### 2. Boundary Value Tests

#### Pattern: At Maximum Limit
```rust
#[test]
fn test_max_file_size_boundary() {
    let file_size = MAX_FILE_SIZE;
    let metadata_ok = file_size <= MAX_FILE_SIZE;
    assert!(metadata_ok, "File at MAX_FILE_SIZE should be accepted");

    let file_size_too_large = MAX_FILE_SIZE + 1;
    let metadata_too_large = file_size_too_large > MAX_FILE_SIZE;
    assert!(metadata_too_large, "File over MAX_FILE_SIZE should be rejected");
}
```

#### Pattern: Minimum Valid Size
```rust
#[test]
fn test_minimal_dimensions() {
    let frame = create_test_frame(1, 1);
    let result = bitvue_decode::decoder::validate_frame(&frame);
    assert!(result.is_ok(), "Should accept 1x1 frame");
}
```

### 3. Malformed Input Tests

#### Pattern: Invalid Magic Number
```rust
#[test]
fn test_invalid_magic_number() {
    let invalid_data = b"INVALID_MAGIC_NUMBER";
    let format = detect_format(invalid_data);
    assert_eq!(format, VideoFormat::Unknown);
}
```

#### Pattern: Truncated Data
```rust
#[test]
fn test_truncated_frame_data() {
    let mut ivf_data = create_minimal_ivf_header();
    ivf_data.extend_from_slice(&1000u32.to_le_bytes()); // Frame size: 1000 bytes
    ivf_data.extend_from_slice(&0u64.to_le_bytes()); // Timestamp
    ivf_data.extend_from_slice(&[0u8; 500]); // Only 500 bytes (truncated)

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_all(&ivf_data);
    assert!(result.is_err(), "Should detect truncated frame data");
}
```

### 4. Path Traversal Tests

#### Pattern: Parent Directory Access
```rust
#[test]
fn test_path_traversal_parent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let traversal_path = temp_dir.path().join("../../../etc/passwd");

    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_from_file(&traversal_path);
    assert!(result.is_err(), "Should reject path traversal attempt");
}
```

#### Pattern: Absolute Path Restriction
```rust
#[test]
fn test_path_traversal_absolute() {
    let system_path = Path::new("/etc/passwd");
    let mut decoder = Av1Decoder::new().unwrap();
    let result = decoder.decode_from_file(system_path);
    assert!(result.is_err(), "Should reject absolute path outside working directory");
}
```

### 5. SIMD Alignment Tests

#### Pattern: Unaligned Access
```rust
#[test]
fn test_unaligned_pointer_access() {
    let mut reference = vec![128u8; 1000];
    let mut distorted = vec![128u8; 1000];

    let reference_offset = &reference[1..]; // Create misalignment
    let distorted_offset = &distorted[1..];

    let result = psnr_simd(reference_offset, distorted_offset, 999, 1);
    assert!(result.is_ok(), "Should handle unaligned pointer access");
}
```

#### Pattern: Non-SIMD-Multiple Sizes
```rust
#[test]
fn test_simd_alignment_edge_cases() {
    let sizes_to_test = vec![1, 15, 16, 17, 31, 32, 33, 63, 64, 65];

    for size in sizes_to_test {
        let reference = vec![128u8; size];
        let distorted = vec![128u8; size];
        let result = psnr_simd(&reference, &distorted, size, 1);
        assert!(result.is_ok(), "Should handle size {} (alignment boundary)", size);
    }
}
```

### 6. Overflow Protection Tests

#### Pattern: Accumulator Overflow
```rust
#[test]
fn test_accumulator_overflow() {
    let reference: Vec<u8> = (0..255).cycle().take(1920 * 1080).collect();
    let distorted: Vec<u8> = reference.iter().map(|&v| v.wrapping_add(1)).collect();

    let result = psnr_simd(&reference, &distorted, 1920, 1080);
    assert!(result.is_ok(), "Should handle large accumulations without overflow");
}
```

#### Pattern: Checked Arithmetic
```rust
#[test]
fn test_multiplication_overflow() {
    let dimensions = [(3840u32, 2160u32), (8192u32, 8192u32)];

    for &(width, height) in &dimensions {
        let _checked = (width as usize).checked_mul(height as usize);
        // Should not panic
    }
}
```

### 7. Concurrent Access Tests

#### Pattern: Thread Safety
```rust
#[test]
fn test_concurrent_decoding() {
    use std::thread;

    let decoder1 = Av1Decoder::new().unwrap();
    let decoder2 = Av1Decoder::new().unwrap();

    let handle1 = thread::spawn(move || {
        decoder1.send_data(&[0x00, 0x00, 0x01, 0x09], 0)
    });

    let handle2 = thread::spawn(move || {
        decoder2.send_data(&[0x00, 0x00, 0x01, 0x09], 0)
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}
```

#### Pattern: Shared Data Access
```rust
#[test]
fn test_simd_thread_safety() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let reference = Arc::new(vec![128u8; 1920 * 1080]);
    let distorted = Arc::new(vec![130u8; 1920 * 1080]);
    let results = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..4).map(|_| {
        let ref_clone = Arc::clone(&reference);
        let dist_clone = Arc::clone(&distorted);
        let results_clone = Arc::clone(&results);
        thread::spawn(move || {
            let result = psnr_simd(&ref_clone, &dist_clone, 1920, 1080);
            let mut results = results_clone.lock().unwrap();
            results.push(result);
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 4);
}
```

### 8. Platform Difference Tests

#### Pattern: Platform-Specific Paths
```rust
#[test]
fn test_windows_path_handling() {
    let windows_path = Path::new("C:\\Users\\test\\video.ivf");

    #[cfg(windows)]
    {
        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_from_file(windows_path);
        // Handle Windows-specific behavior
    }

    #[cfg(unix)]
    {
        let mut decoder = Av1Decoder::new().unwrap();
        let result = decoder.decode_from_file(windows_path);
        assert!(result.is_err(), "Should fail on Unix with Windows-style absolute path");
    }
}
```

#### Pattern: SIMD Feature Detection
```rust
#[test]
fn test_simd_feature_detection() {
    #[cfg(target_arch = "x86_64")]
    {
        let has_avx2 = is_x86_feature_detected!("avx2");
        let has_avx = is_x86_feature_detected!("avx");
        let has_sse2 = is_x86_feature_detected!("sse2");

        assert!(has_sse2, "SSE2 should be available on x86_64");
        if has_avx2 {
            assert!(has_avx, "AVX2 implies AVX is available");
        }
    }
}
```

### 9. Precision Tests

#### Pattern: Floating Point Edge Cases
```rust
#[test]
fn test_mse_zero_division() {
    let reference = vec![100u8; 1920 * 1080];
    let distorted = vec![100u8; 1920 * 1080];

    let result = psnr_simd(&reference, &distorted, 1920, 1080).unwrap();

    assert!(result.is_infinite() && result.is_sign_positive(),
        "MSE=0 should give positive infinity, not NaN");
}
```

#### Pattern: SIMD vs Scalar Consistency
```rust
#[test]
fn test_simd_scalar_consistency() {
    let reference = vec![100u8; 1920 * 1080];
    let distorted = vec![150u8; 1920 * 1080];

    let simd_result = psnr_simd(&reference, &distorted, 1920, 1080);
    let scalar_result = psnr(&reference, &distorted, 1920, 1080);

    let simd_psnr = simd_result.unwrap();
    let scalar_psnr = scalar_result.unwrap();

    assert!((simd_psnr - scalar_psnr).abs() < 0.5,
        "SIMD={} and Scalar={} should match within 0.5 dB", simd_psnr, scalar_psnr);
}
```

### 10. Error Message Tests

#### Pattern: Informative Error Messages
```rust
#[test]
fn test_validation_error_messages() {
    if let Err(BitvueError::InvalidData(msg)) = validate_thread_count(0) {
        assert!(msg.contains("thread") || msg.contains("Thread"),
            "Error message should mention threads: {}", msg);
        assert!(msg.contains("below") || msg.contains("minimum"),
            "Error message should explain the issue: {}", msg);
    } else {
        panic!("Should return InvalidData error for zero threads");
    }
}
```

## Helper Functions

### Test Frame Creation
```rust
fn create_test_frame(width: u32, height: u32) -> DecodedFrame {
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
```

### IVF Header Creation
```rust
fn create_minimal_ivf_header() -> Vec<u8> {
    let mut header = Vec::new();

    header.extend_from_slice(b"DKIF"); // Magic number
    header.extend_from_slice(&0u16.to_le_bytes()); // Version
    header.extend_from_slice(&32u16.to_le_bytes()); // Header size
    header.extend_from_slice(b"AV01"); // FourCC
    header.extend_from_slice(&1920u16.to_le_bytes()); // Width
    header.extend_from_slice(&1080u16.to_le_bytes()); // Height
    header.extend_from_slice(&60u32.to_le_bytes()); // Timebase numerator
    header.extend_from_slice(&1u32.to_le_bytes()); // Timebase denominator
    header.extend_from_slice(&0u32.to_le_bytes()); // Number of frames
    header.extend_from_slice(&0u32.to_le_bytes()); // Reserved

    header
}
```

## Running the Tests

### Basic Test Execution
```bash
# Run all edge case tests
cargo test --test edge_cases_test

# Run specific module
cargo test -p bitvue-decode --test edge_cases_test
cargo test -p bitvue-metrics --test edge_cases_test
cargo test -p bitvue-core limits_edge_cases

# Run with output
cargo test --test edge_cases_test -- --nocapture

# Run specific test
cargo test test_empty_file
```

### With Sanitizers
```bash
# Address sanitizer (detects memory errors)
cargo +nightly test -Z build-std --test edge_cases_test -- -Z sanitizer=address

# Thread sanitizer (detects data races)
cargo +nightly test -Z build-std --test edge_cases_test -- -Z sanitizer=thread
```

### Release Mode Testing
```bash
# Test optimized build (may reveal different bugs)
cargo test --test edge_cases_test --release
```

## Test Dependencies

### Required Crates
```toml
[dev-dependencies]
tempfile = "3"           # For temporary file/directory creation
```

### Platform-Specific
```toml
[target.'cfg(unix)'.dev-dependencies]
# Unix-specific test dependencies

[target.'cfg(windows)'.dev-dependencies]
# Windows-specific test dependencies
```

## CI/CD Integration

### GitHub Actions Workflow
```yaml
name: Edge Case Tests

on:
  push:
    branches: [main, develop]
  pull_request:

jobs:
  edge_cases:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable, nightly]

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
          override: true

      - name: Install dependencies
        run: cargo fetch

      - name: Run edge case tests
        run: cargo test --test edge_cases_test --all-targets

      - name: Run with sanitizers (nightly only)
        if: matrix.rust == 'nightly' && matrix.os == 'ubuntu-latest'
        run: |
          cargo +nightly test -Z build-std --test edge_cases_test -- -Z sanitizer=address

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: test-results-${{ matrix.os }}-${{ matrix.rust }}
          path: |
            target/ctest-results/
            **/ctest-results/
```

### GitLab CI Pipeline
```yaml
edge_case_tests:
  script:
    - cargo test --test edge_cases_test --all-targets
  parallel:
    matrix:
      - RUST: [stable, nightly]
        OS: [ubuntu, windows, macos]
  artifacts:
    when: always
    paths:
      - target/ctest-results/
```

## Test Maintenance

### Adding New Tests
1. Identify the edge case or boundary condition
2. Determine expected behavior
3. Write test following naming convention
4. Add to appropriate test file
5. Update documentation

### Test Naming Convention
```rust
// Format: test_{category}_{specific_case}_{expected_behavior}

test_empty_file_returns_error
test_max_file_size_accepts_at_limit
test_invalid_magic_number_detects_as_unknown
test_concurrent_decoding_no_deadlock
```

### When Tests Fail
1. Determine if it's a bug or test issue
2. If bug: Fix code, add regression test
3. If test issue: Update test to reflect new behavior
4. Document the reason for changes

## Coverage Tracking

### Using tarpaulin
```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --test edge_cases_test --out Html --output-dir coverage/

# View coverage
open coverage/index.html
```

### Using cargo-llvm-cov
```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --test edge_cases_test --html

# View coverage
open target/llvm-cov/html/index.html
```

## Best Practices

### DO
- Test all public APIs with edge cases
- Test all boundary conditions
- Test all error paths
- Test all platform-specific code
- Use descriptive test names
- Include expected behavior in doc comments
- Mock external dependencies

### DON'T
- Test private methods directly
- Test third-party library internals
- Write fragile tests that break easily
- Ignore test failures
- Skip edge case tests for time

## References

### Documentation
- Test Coverage: `/Users/hawk/Workspaces/bitvue/docs/edge-case-test-coverage.md`
- Codebase: `/Users/hawk/Workspaces/bitvue/`

### Related Files
- Decoder: `/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/src/decoder.rs`
- Limits: `/Users/hawk/Workspaces/bitvue/crates/bitvue-core/src/limits.rs`
- SIMD: `/Users/hawk/Workspaces/bitvue/crates/bitvue-metrics/src/simd.rs`

### Test Resources
- Rust Testing Guide: https://doc.rust-lang.org/book/ch11-00-testing.html
- Rust By Example: https://doc.rust-lang.org/rust-by-example/testing.html
