# Edge Case Tests for Abseil-Rust

This document describes the comprehensive edge case tests created for the abseil-rust library.
These tests cover scenarios that fuzz tests might miss, focusing on integer overflow, abnormal
inputs, boundary conditions, and thread safety.

## Test Files

### 1. `integer_overflow_tests.rs`

Tests for integer overflow scenarios that could lead to security vulnerabilities:

#### MemoryRegion Overflow Tests
- `test_memory_region_from_size_usize_max` - Verifies overflow detection when base + size overflows
- `test_memory_region_from_size_zero_at_max` - Tests zero-size region at usize::MAX
- `test_memory_region_from_size_near_max` - Tests region creation near usize::MAX boundary
- `test_memory_region_new_zero_length` - Tests zero-length regions
- `test_memory_region_new_max_range` - Tests region covering entire address space
- `test_memory_region_contains_max_address` - Tests contains() at usize::MAX boundary
- `test_memory_region_sub_region_max_offset` - Tests sub_region offset overflow
- `test_memory_region_sub_region_size_overflow` - Tests sub_region size overflow
- `test_memory_region_sub_region_exact_bounds` - Tests sub_region at exact parent bounds
- `test_memory_region_intersection_with_max` - Tests intersection at usize::MAX
- `test_memory_region_overlaps_at_max` - Tests overlaps() at high addresses
- `test_memory_region_size_overflow_in_calculation` - Verifies size() doesn't overflow
- `test_memory_region_alignment_at_max` - Tests align_up() at high addresses
- `test_memory_region_alignment_overflow` - Tests align_up() overflow handling
- `test_memory_region_contains_region_at_max` - Tests contains_region() at usize::MAX
- `test_memory_region_contains_region_same` - Tests that region contains itself

#### BloomFilter Overflow Tests
- `test_bloom_filter_capacity_overflow` - Tests capacity calculation overflow
- `test_bloom_filter_max_hashes` - Tests with large num_hashes value
- `test_bloom_filter_single_bit_capacity` - Tests with minimal capacity
- `test_bloom_filter_large_capacity_small_hashes` - Tests large capacity with few hashes
- `test_bloom_filter_small_capacity_many_hashes` - Tests small capacity with many hashes
- `test_bloom_filter_overflow_in_insert` - Tests insert with values that could cause overflow
- `test_bloom_filter_all_zeros_input` - Tests with all-zero hash values
- `test_bloom_filter_all_ones_input` - Tests with all-ones hash values

### 2. `abnormal_input_tests.rs`

Tests for abnormal and unusual inputs:

#### Hash Function Tests
- `test_hash_with_empty_slice` - Tests all hash functions with empty input
- `test_hash_with_single_byte` - Tests hash functions with single byte
- `test_hash_with_all_zeros` - Tests with 1KB and 1MB of zeros
- `test_hash_with_all_ones` - Tests with 1KB and 1MB of 0xFF bytes
- `test_hash_with_alternating_pattern` - Tests with alternating 0xAA/0x55 pattern
- `test_hash_with_incrementing_pattern` - Tests with incrementing byte pattern
- `test_hash_with_large_input_1mb` - Tests hash functions with 1MB input
- `test_hash_with_large_input_10mb` - Tests hash functions with 10MB input
- `test_hash_with_very_large_input` - Tests with 100MB (optional, skipped in CI)

#### BloomFilter Abnormal Input Tests
- `test_bloom_filter_empty` - Tests BloomFilter with no inserts
- `test_bloom_filter_all_same_value` - Tests inserting same value many times
- `test_bloom_filter_zero_values` - Tests with various zero representations
- `test_bloom_filter_max_values` - Tests with MAX values for various types
- `test_bloom_filter_negative_values` - Tests with negative integers
- `test_bloom_filter_special_floats` - Tests with INF, NEG_INF, NAN
- `test_bloom_filter_unicode_strings` - Tests with Unicode strings (Korean, emoji, etc.)
- `test_bloom_filter_clear_empty` - Tests clearing empty BloomFilter
- `test_bloom_filter_clear_full` - Tests clearing populated BloomFilter
- `test_bloom_filter_false_positive_rate` - Measures false positive rate

#### MemoryRegion Abnormal Input Tests
- `test_memory_region_empty` - Tests zero-size region
- `test_memory_region_single_byte` - Tests single-byte region
- `test_memory_region_at_zero` - Tests region starting at address 0
- `test_memory_region_overlap_touching` - Tests that touching regions don't overlap

#### Hash Collision Detection
- `test_hash_collision_detection` - Tests hash collision behavior
- `test_hash_of_special_types` - Tests hash_of with Option, Result, Array, Tuple

### 3. `boundary_condition_tests.rs`

Tests for boundary conditions and edge values:

#### OnceFlag Boundary Tests
- `test_once_flag_default` - Tests OnceFlag with Default trait
- `test_once_flag_new` - Tests OnceFlag::new()
- `test_once_flag_called` - Tests OnceFlag::called() pre-called state
- `test_once_flag_single_call` - Tests single initialization call
- `test_once_flag_multiple_calls` - Tests multiple calls to same flag
- `test_once_flag_with_panic` - Tests behavior when closure panics

#### MemoryRegion Boundary Tests
- `test_memory_region_zero_size` - Tests MemoryRegion with size 0
- `test_memory_region_size_one` - Tests MemoryRegion with size 1
- `test_memory_region_min_address` - Tests region at address 0
- `test_memory_region_from_size_zero` - Tests from_size with size 0
- `test_memory_region_from_size_one` - Tests from_size with size 1
- `test_memory_region_sub_region_zero_offset` - Tests sub_region with offset 0
- `test_memory_region_sub_region_zero_size` - Tests sub_region with size 0
- `test_memory_region_sub_region_full_size` - Tests sub_region covering entire region
- `test_memory_region_intersection_empty` - Tests intersection of non-overlapping regions
- `test_memory_region_intersection_touching` - Tests intersection of touching regions
- `test_memory_region_intersection_single_byte` - Tests intersection resulting in 1 byte
- `test_memory_region_contains_region_empty` - Tests contains_region with empty region
- `test_memory_region_overlaps_identical` - Tests that identical regions overlap
- `test_memory_region_overlaps_empty` - Tests overlap with empty regions
- `test_memory_region_contains_boundary` - Tests contains() at boundary addresses

#### BloomFilter Boundary Tests
- `test_bloom_filter_min_capacity` - Tests with minimum valid capacity (1)
- `test_bloom_filter_min_hashes` - Tests with minimum valid num_hashes (1)
- `test_bloom_filter_single_insert` - Tests with single insert
- `test_bloom_filter_no_inserts` - Tests with no inserts
- `test_bloom_filter_clear_immediate` - Tests clear immediately after creation
- `test_bloom_filter_clear_after_insert` - Tests clear after inserts
- `test_bloom_filter_insert_same_value` - Tests inserting same value multiple times
- `test_bloom_filter_boundary_values` - Tests with MIN/MAX values for all integer types
- `test_bloom_filter_empty_strings` - Tests with empty strings
- `test_bloom_filter_single_char_strings` - Tests with single character strings
- `test_bloom_filter_try_new_zero_capacity` - Tests try_new with capacity 0
- `test_bloom_filter_try_new_zero_hashes` - Tests try_new with num_hashes 0
- `test_bloom_filter_try_new_both_zero` - Tests try_new with both parameters 0
- `test_bloom_filter_try_new_min_valid` - Tests try_new with minimum valid values

### 4. `thread_safety_tests.rs`

Tests for concurrent access patterns and synchronization:

#### OnceFlag Concurrency Tests
- `test_once_flag_concurrent_initialization` - Tests 100 threads racing to initialize
- `test_once_flag_concurrent_after_done` - Tests concurrent calls after initialization
- `test_once_flag_stress_test` - Tests 1000 threads × 100 calls = 100,000 total calls
- `test_once_flag_slow_initialization` - Tests with slow initialization (100ms delay)
- `test_once_flag_no_deadlock` - Tests for deadlocks with 100 iterations × 20 threads
- `test_once_flag_interleaved_calls` - Tests interleaved calls with different flags
- `test_once_flag_panic_recovery` - Tests behavior when closure panics
- `test_once_flag_static_scope` - Tests OnceFlag in static scope

#### BloomFilter Concurrency Tests
- `test_bloom_filter_concurrent_inserts` - Tests 10 threads inserting 1000 items each
- `test_bloom_filter_concurrent_insert_and_contains` - Tests concurrent insert/query
- `test_bloom_filter_concurrent_clear` - Tests periodic clears during concurrent inserts
- `test_bloom_filter_stress_concurrent` - Tests 20 threads × 1000 operations
- `test_bloom_filter_memory_barrier` - Tests memory ordering correctness

#### MemoryRegion Thread Safety Tests
- `test_memory_region_thread_safe_clone` - Tests cloning between 10 threads

### 5. `stress_tests.rs`

Stress tests for performance and memory leak detection:

#### Creation/Destruction Tests
- `test_once_flag_repeated_creation` - Tests creating 10,000 OnceFlags
- `test_once_flag_repeated_calls` - Tests 10,000 calls to same flag
- `test_once_flag_many_flags` - Tests 1000 different flags
- `test_once_flag_memory_stress` - Tests for memory leaks with 1000 iterations

#### MemoryRegion Stress Tests
- `test_memory_region_many_regions` - Tests creating 10,000 regions
- `test_memory_region_many_sub_regions` - Tests 10,000 sub_region operations
- `test_memory_region_intersection_stress` - Tests 1000 intersection operations
- `test_memory_region_overlaps_stress` - Tests 2000 overlap checks
- `test_memory_region_edge_combinations` - Tests various edge combinations

#### BloomFilter Stress Tests
- `test_bloom_filter_many_filters` - Tests creating 100 BloomFilters
- `test_bloom_filter_many_inserts` - Tests 10,000 inserts
- `test_bloom_filter_repeated_clear` - Tests 100 rounds of insert/verify/clear
- `test_bloom_filter_collision_stress` - Measures false positive rate
- `test_bloom_filter_unicode_stress` - Tests 1000 Unicode strings
- `test_bloom_filter_pattern_stress` - Tests various bit patterns
- `test_bloom_filter_capacity_stress` - Tests various capacities
- `test_bloom_filter_num_hashes_stress` - Tests various num_hashes values

#### Hash Function Stress Tests
- `test_hash_repeated_operations` - Tests 10,000 hash operations for consistency
- `test_hash_many_different_inputs` - Tests hashing 1000 different inputs
- `test_hash_sized_inputs` - Tests with various input sizes (1 to 8192 bytes)
- `test_hash_consistency_stress` - Tests 1000 calls for consistency

## Running the Tests

To run all edge case tests:

```bash
cargo test --test integer_overflow_tests
cargo test --test abnormal_input_tests
cargo test --test boundary_condition_tests
cargo test --test thread_safety_tests
cargo test --test stress_tests
```

To run all tests at once:

```bash
cargo test --test '*'
```

## Coverage Areas

These tests cover the following areas that fuzz tests might miss:

1. **Integer Overflow**: Prevention of overflow in arithmetic operations
2. **Boundary Values**: Testing with MIN/MAX values for various types
3. **Empty Collections**: Behavior with zero-size/empty data structures
4. **Single Elements**: Behavior with single-element collections
5. **Concurrent Access**: Thread safety under concurrent operations
6. **Memory Safety**: No memory leaks under stress
7. **Special Floats**: Handling of INF, NEG_INF, NAN
8. **Unicode**: Proper handling of Unicode strings
9. **Large Inputs**: Correct behavior with 1MB+ inputs
10. **Pattern Tests**: Special bit patterns (all zeros, all ones, alternating)

## Security Considerations

These tests specifically target security-related concerns:
- Integer overflow vulnerabilities (CWE-190)
- Memory corruption from overflow
- Race conditions in concurrent code
- DoS from malformed inputs
- Memory exhaustion from repeated operations

## Maintenance

When adding new functionality to abseil-rust, ensure corresponding edge case tests are added:
1. Add overflow tests for any arithmetic operations
2. Add boundary tests for any ranged operations
3. Add concurrent tests for any shared state
4. Add stress tests for any data structures
5. Add abnormal input tests for any parsing/processing

## References

- The tests complement the existing fuzz tests in `/fuzz/fuzz_targets/`
- Related security fixes in git history:
  - bb7d84b: Fix integer overflow in memory_region.rs sub_region
  - eef8a41: Fix integer overflow in contains()
  - a03a24a: Fix integer overflow in bloom filter index calculation
  - dbff87a: Fix busy-wait DoS in call_once_impl
