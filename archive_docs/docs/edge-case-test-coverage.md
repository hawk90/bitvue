# Edge Case and Boundary Condition Test Coverage

This document provides comprehensive coverage of edge case and boundary condition tests for the Bitvue video analyzer.

## Test Suite Organization

### Module: `bitvue-decode`
**File**: `crates/bitvue-decode/tests/edge_cases_test.rs`

### Module: `bitvue-metrics`
**File**: `crates/bitvue-metrics/tests/edge_cases_test.rs`

### Module: `bitvue-core`
**File**: `crates/bitvue-core/src/limits_edge_cases.rs`

---

## Test Categories

### 1. Empty/Null Inputs

#### bitvue-decode
- **test_empty_file**: Tests decoder with completely empty file
  - Expected: Returns error indicating no data to decode
  - Validates: `decode_all(&[])`

- **test_empty_ivf_header**: Tests incomplete IVF header
  - Expected: Returns error about insufficient header data
  - Validates: IVF parsing with < 32 bytes

- **test_zero_length_frame_data**: Tests zero-length frame in IVF
  - Expected: Skips zero-length frames or returns error
  - Validates: Frame header with size = 0

- **test_null_pointer_handling**: Tests None/null-like situations
  - Expected: Handles gracefully without crash
  - Validates: Decoder with None timestamp

#### bitvue-metrics
- **test_empty_buffers**: Tests PSNR with empty buffers
  - Expected: Handles gracefully or returns error
  - Validates: `psnr_simd(&[], &[], 0, 0)`

- **test_zero_length_dimension**: Tests zero width or height
  - Expected: Rejects or handles appropriately
  - Validates: `psnr_simd(data, data, 0, 1080)`

- **test_single_pixel**: Tests smallest valid image (1x1)
  - Expected: Calculates correctly
  - Validates: `psnr_simd(&[128], &[130], 1, 1)`

---

### 2. Boundary Values

#### bitvue-decode
- **test_zero_dimensions**: Tests zero width or height
  - Expected: Rejects zero dimensions
  - Validates: `validate_frame` with 0xN or Nx0

- **test_minimal_dimensions**: Tests smallest valid dimensions (1x1)
  - Expected: Handles 1x1 frames correctly
  - Validates: Frame with width=1, height=1

- **test_max_file_size_boundary**: Tests at exactly MAX_FILE_SIZE
  - Expected: Accepts file at exactly MAX_FILE_SIZE
  - Validates: File size validation logic

- **test_max_frames_per_file_boundary**: Tests at exactly MAX_FRAMES_PER_FILE
  - Expected: Accepts at limit, rejects above
  - Validates: Frame count validation

- **test_max_frame_size_boundary**: Tests at exactly MAX_FRAME_SIZE
  - Expected: Accepts at limit, rejects above
  - Validates: Frame size validation

- **test_negative_values_wrapping**: Tests unsigned integer wrapping
  - Expected: Handles wrapped values gracefully
  - Validates: Timestamp = u64::MAX

#### bitvue-metrics
- **test_minimum_psnr_values**: Tests maximum distortion
  - Expected: Handles minimum PSNR (~0 dB)
  - Validates: All zeros vs all 255s

- **test_maximum_psnr_values**: Tests identical images
  - Expected: Returns infinity
  - Validates: MSE = 0 case

- **test_boundary_pixel_values**: Tests pixel values at 0 and 255
  - Expected: Handles extreme values correctly
  - Validates: All zeros, all 255s, mixed extremes

- **test_odd_dimensions**: Tests non-power-of-2 dimensions
  - Expected: Handles odd dimensions correctly
  - Validates: 1919x1079 image

- **test_prime_dimensions**: Tests prime number dimensions
  - Expected: Handles prime dimensions correctly
  - Validates: 997x991 image

#### bitvue-core
- **test_thread_count_below_minimum**: Tests thread count at/below minimum
  - Expected: Rejects below MIN_WORKER_THREADS
  - Validates: `validate_thread_count(0)`

- **test_thread_count_at_maximum**: Tests at maximum
  - Expected: Accepts at MAX, rejects above
  - Validates: `validate_thread_count(32)` and `validate_thread_count(33)`

- **test_buffer_size_boundaries**: Tests buffer size at boundaries
  - Expected: Accepts at MAX_BUFFER_SIZE
  - Validates: `validate_buffer_size(100MB)` and `validate_buffer_size(100MB+1)`

- **test_cache_size_boundaries**: Tests cache size at boundaries
  - Expected: Accepts at MAX_CACHE_ENTRIES
  - Validates: `validate_cache_size(1000)` and `validate_cache_size(1001)`

---

### 3. Malformed Inputs

#### bitvue-decode
- **test_invalid_magic_number**: Tests invalid file magic number
  - Expected: Detects as unknown format
  - Validates: `detect_format(b"INVALID_MAGIC")`

- **test_corrupt_ivf_header**: Tests corrupted IVF header
  - Expected: Rejects or handles gracefully
  - Validates: Invalid version, header size

- **test_truncated_frame_data**: Tests truncated frame data
  - Expected: Detects incomplete frame
  - Validates: Frame header size > actual data

- **test_invalid_chroma_format**: Tests invalid chroma subsampling
  - Expected: Detects and rejects invalid format
  - Validates: Mismatched U/V plane sizes

- **test_invalid_bit_depth**: Tests invalid bit depth values
  - Expected: Warns about unusual bit depths
  - Validates: Bit depth = 15

- **test_mismatched_plane_sizes**: Tests mismatched U/V planes
  - Expected: Rejects mismatched chroma planes
  - Validates: U size != V size

---

### 4. Size Limits

#### bitvue-decode
- **test_very_large_frame_dimensions**: Tests maximum supported dimensions
  - Expected: Handles large dimensions within limits
  - Validates: 8192x8192 frame

- **test_memory_allocation_limits**: Tests memory limit enforcement
  - Expected: Doesn't allocate excessive memory
  - Validates: File with frame size = MAX_FRAME_SIZE

- **test_buffer_size_limits**: Tests I/O buffer limits
  - Expected: Respects MAX_BUFFER_SIZE
  - Validates: `validate_buffer_size`

#### bitvue-core
- **test_file_size_boundaries**: Tests file size at boundaries
  - Expected: MAX_FILE_SIZE = 2GB, fits in u64
  - Validates: `MAX_FILE_SIZE < u64::MAX`

- **test_file_size_practical_videos**: Tests practical video sizes
  - Expected: Handles typical video durations
  - Validates: 4K@60fps for >1 hour, 1080p@30fps for >2 hours

- **test_frame_count_boundaries**: Tests frame count at boundaries
  - Expected: 100,000 frames = ~27 min at 60fps
  - Validates: `MAX_FRAMES_PER_FILE / fps`

- **test_frame_size_boundaries**: Tests frame size at boundaries
  - Expected: 100 MB accommodates 8K frames
  - Validates: 8K YUV420 = ~50 MB

---

### 5. Path Traversal

#### bitvue-decode
- **test_path_traversal_parent_directory**: Tests ".." in paths
  - Expected: Rejects paths with ".."
  - Validates: `decode_from_file("../../../etc/passwd")`

- **test_path_traversal_absolute**: Tests absolute paths outside working dir
  - Expected: Rejects absolute paths
  - Validates: `decode_from_file("/etc/passwd")`

- **test_symlink_restriction**: Tests symlinks to outside directory
  - Expected: Rejects symlinks outside working directory
  - Validates: Symlink to /etc/passwd

---

### 6. Large Inputs

#### bitvue-decode
- **test_deep_nesting_structure**: Tests deeply nested structures
  - Expected: Enforces MAX_RECURSION_DEPTH
  - Validates: Structure depth = 100, 101

#### bitvue-core
- **test_grid_block_limits**: Tests maximum grid blocks
  - Expected: Enforces MAX_GRID_BLOCKS
  - Validates: Grid blocks = 262,144, 262,145

- **test_grid_dimension_limits**: Tests grid dimension validation
  - Expected: Enforces MAX_GRID_DIMENSION
  - Validates: Grid dimension = 512, 513

---

### 7. Concurrent Access

#### bitvue-decode
- **test_concurrent_decoding**: Tests multiple decoder instances
  - Expected: Each decoder operates independently
  - Validates: 3 threads using 3 decoders simultaneously

- **test_shared_frame_data**: Tests Arc-wrapped frame data cloning
  - Expected: Multiple references work correctly
  - Validates: `Arc::ptr_eq` on cloned frames

#### bitvue-metrics
- **test_concurrent_psnr_calculations**: Tests concurrent PSNR calculations
  - Expected: Each calculation is independent
  - Validates: 8 threads calculating PSNR simultaneously

- **test_simd_thread_safety**: Tests SIMD code thread safety
  - Expected: No data races or undefined behavior
  - Validates: 4 threads accessing shared data

---

### 8. Error Conditions

#### bitvue-decode
- **test_file_not_found**: Tests non-existent file
  - Expected: Returns appropriate error
  - Validates: `decode_from_file("/nonexistent.ivf")`

- **test_permission_denied**: Tests unreadable file
  - Expected: Returns permission error
  - Validates: File with 000 permissions

- **test_directory_as_file**: Tests directory path
  - Expected: Returns error (not a file)
  - Validates: `decode_from_file("/tmp")`

---

### 9. Encoding Issues

#### bitvue-decode
- **test_invalid_utf8_in_paths**: Tests invalid UTF-8 in paths
  - Expected: Handles gracefully
  - Validates: Path with bytes [0xFF, 0xFE, 0xFD]

- **test_unicode_path_handling**: Tests Unicode characters in paths
  - Expected: Handles Unicode correctly
  - Validates: Path with Chinese characters

- **test_bom_handling**: Tests BOM (Byte Order Mark) in data
  - Expected: Handles or rejects BOM
  - Validates: Data with UTF-8 BOM prefix

---

### 10. Platform Differences

#### bitvue-decode
- **test_windows_path_handling**: Tests Windows-style paths
  - Expected: Handles on Windows, rejects on Unix
  - Validates: `C:\Users\test\video.ivf`

- **test_mixed_path_separators**: Tests mixed path separators
  - Expected: Handles or normalizes
  - Validates: `subdir\file.ivf` on Unix

- **test_line_endings_in_metadata**: Tests different line endings
  - Expected: Handles both CRLF and LF
  - Validates: `b"DKIF\r\n"` vs `b"DKIF\n"`

#### bitvue-metrics
- **test_simd_feature_detection**: Tests SIMD feature detection
  - Expected: Detects available features
  - Validates: `is_x86_feature_detected!("avx2")`

- **test_cross_platform_consistency**: Tests SIMD implementations agree
  - Expected: AVX2, AVX, SSE2, NEON, scalar all agree
  - Validates: SIMD vs scalar results match

- **test_endianness_handling**: Tests endianness handling
  - Expected: Works on both little and big endian
  - Validates: Specific byte pattern [0x12, 0x34, ...]

---

### 11. Additional SIMD-Specific Tests (bitvue-metrics)

#### Alignment Issues
- **test_unaligned_pointer_access**: Tests unaligned memory access
  - Expected: Uses unaligned loads
  - Validates: `psnr_simd(&data[1..], &data[1..], ...)`

- **test_stack_vs_heap_allocation**: Tests stack and heap data
  - Expected: Works with both
  - Validates: Array vs Vec

#### Precision Issues
- **test_floating_point_edge_cases**: Tests float edge cases
  - Expected: Handles infinity, NaN correctly
  - Validates: Identical images → ∞

- **test_mse_zero_division**: Tests MSE = 0 case
  - Expected: Returns +∞, not NaN
  - Validates: MSE = 0 → PSNR = +∞

- **test_simd_scalar_consistency**: Tests SIMD vs scalar agreement
  - Expected: Results match within tolerance
  - Validates: SIMD - Scalar < 0.5 dB

#### Special Cases
- **test_checkerboard_pattern**: Tests alternating extreme values
  - Expected: Handles correctly
  - Validates: Pattern [0, 255, 0, 255, ...]

- **test_gradient_pattern**: Tests full value range
  - Expected: Handles full range
  - Validates: Gradient (0..255).cycle()

- **test_single_bit_difference**: Tests minimal difference
  - Expected: Detects single-bit differences
  - Validates: Single pixel changed by 1

- **test_all_same_value**: Tests flat fields
  - Expected: Returns ∞
  - Validates: All pixels = constant

#### Performance Edge Cases
- **test_very_small_input**: Tests SIMD overhead dominates
  - Expected: Still works correctly
  - Validates: 10 pixels

- **test_non_simd_multiple_size**: Tests remainder handling
  - Expected: Remainder handling works
  - Validates: Sizes 31, 32, 33, 63, 64, 65, ...

- **test_power_of_two_sizes**: Tests optimal alignment
  - Expected: Works efficiently
  - Validates: Sizes 16, 32, 64, 128, ...

- **test_widescreen_dimensions**: Tests extreme aspect ratios
  - Expected: Handles any valid aspect ratio
  - Validates: 16:9, 2.40:1, 9:16, 1:1, ...

- **test_repeated_calculations**: Tests consistency
  - Expected: Same input → same output
  - Validates: 10 calculations, all identical

---

### 12. Overflow Conditions (bitvue-metrics)

- **test_accumulator_overflow**: Tests accumulator overflow prevention
  - Expected: Uses 32-bit or wider accumulators
  - Validates: 4K frame with max differences

- **test_wrapping_arithmetic**: Tests signed subtraction
  - Expected: Uses signed subtraction for accuracy
  - Validates: 0 vs 255 comparison

- **test_large_frame_overflow_protection**: Tests u64 accumulator usage
  - Expected: Uses u64 for safety
  - Validates: 8K × 8K frame calculation

---

### 13. Additional Limit Validation Tests (bitvue-core)

#### Arithmetic Overflow
- **test_multiplication_overflow**: Tests safe multiplication
  - Expected: Uses checked arithmetic
  - Validates: width × height for various sizes

- **test_addition_overflow**: Tests safe addition
  - Expected: Uses checked arithmetic
  - Validates: offset + 1

- **test_type_conversion_limits**: Tests type conversions
  - Expected: Handles u32→usize, u64→usize
  - Validates: Conversions on 32-bit and 64-bit

#### Constant Consistency
- **test_limit_relationships**: Tests limit relationships
  - Expected: Limits internally consistent
  - Validates: MAX_FRAME_SIZE < MAX_FILE_SIZE

- **test_limit_reasonableness**: Tests limit practicality
  - Expected: Limits are practical
  - Validates: Thread count, buffer size, file size ranges

- **test_limit_documentation_consistency**: Tests values match comments
  - Expected: Values match documentation
  - Validates: All limit constants

#### Error Messages
- **test_validation_error_messages**: Tests error message quality
  - Expected: Error messages are informative
  - Validates: Messages mention the parameter and limit

#### Edge Case Combinations
- **test_combined_validations**: Tests multiple validations together
  - Expected: All validations work independently
  - Validates: Thread count + buffer size + cache size

- **test_limit_cross_product**: Tests no impossible constraints
  - Expected: Possible to satisfy all limits
  - Validates: Valid configuration exists

---

## Running the Tests

### All Edge Case Tests
```bash
# Run all edge case tests
cargo test --test edge_cases_test

# Run with output
cargo test --test edge_cases_test -- --nocapture

# Run specific test
cargo test test_empty_file
```

### Specific Module Tests
```bash
# Decoder edge cases
cd crates/bitvue-decode
cargo test --test edge_cases_test

# Metrics edge cases
cd crates/bitvue-metrics
cargo test --test edge_cases_test

# Limits edge cases
cd crates/bitvue-core
cargo test limits_edge_cases
```

### With Sanitizers
```bash
# Run with address sanitizer (nightly Rust)
cargo +nightly test -Z build-std --test edge_cases_test -- -Z sanitizer=address

# Run with thread sanitizer (nightly Rust)
cargo +nightly test -Z build-std --test edge_cases_test -- -Z sanitizer=thread
```

---

## Test Coverage Goals

### Coverage Targets
- **Unit Tests**: 80%+ coverage
- **Edge Cases**: 100% of identified edge cases covered
- **Boundary Conditions**: All limit values tested
- **Error Paths**: All error return paths tested

### What Gets Tested
1. All public APIs with valid and invalid inputs
2. All limit constants and validation functions
3. All error conditions and error messages
4. All type conversions and arithmetic operations
5. All platform-specific code paths
6. All SIMD implementations (AVX2, AVX, SSE2, NEON)

### What Doesn't Need Testing
- Private helper functions (tested via public API)
- Third-party library internals
- Generated code

---

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Edge Case Tests

on: [push, pull_request]

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

      - name: Run edge case tests
        run: |
          cargo test --test edge_cases_test --all-targets

      - name: Run with sanitizers (nightly)
        if: matrix.rust == 'nightly'
        run: |
          cargo +nightly test -Z build-std --test edge_cases_test -- -Z sanitizer=address
```

---

## Maintenance

### Adding New Tests
When adding new functionality, ensure:
1. Add tests for all edge cases identified in design
2. Add tests for all boundary conditions
3. Add tests for all error paths
4. Add tests for all platform-specific code

### Reviewing Test Failures
When tests fail:
1. Check if it's a legitimate bug or test issue
2. If bug: fix the code, not the test
3. If test issue: update test to reflect new behavior
4. Add regression test for bugs found

### Test Documentation
Keep this file updated:
- Add new test categories as needed
- Document why each edge case is important
- Note any platform-specific behaviors
- Track test coverage metrics

---

## References

### Security Considerations
- All file path operations validate against directory traversal
- All size validations prevent memory exhaustion DoS
- All arithmetic uses checked operations to prevent overflow
- All SIMD operations use explicit bounds checking

### Performance Considerations
- Edge case tests may be slower than typical tests
- Consider running edge cases in separate CI job
- Some tests (large allocations) may be marked as ignored

### Platform Support
- Tests run on Linux, Windows, macOS
- SIMD tests detect available features at runtime
- Path handling tests adapt to platform conventions

---

## Appendix: Test Naming Convention

### Format
`test_{category}_{specific_case}_{expected_behavior}`

### Examples
- `test_empty_file_returns_error`
- `test_max_file_size_boundary_accepts_at_limit`
- `test_invalid_magic_number_detects_as_unknown`
- `test_concurrent_decoding_no_deadlock`

### Categories
- `empty` / `null`: Empty or null inputs
- `boundary`: Values at limits
- `malformed`: Invalid/corrupt data
- `size`: Size limit tests
- `path`: File path tests
- `large`: Large input tests
- `concurrent`: Multi-threading tests
- `error`: Error condition tests
- `encoding`: Character encoding tests
- `platform`: Platform-specific tests
