# Edge Case Testing - Quick Reference

## Implemented Tests

### Location
```
/Users/hawk/Workspaces/bitvue/crates/bitvue-decode/tests/edge_case_tests.rs
```

### Running Tests

```bash
# All edge case tests
cargo test --package bitvue-decode --test edge_case_tests

# Specific categories
cargo test --package bitvue-decode --test edge_case_tests empty
cargo test --package bitvue-decode --test edge_case_tests overflow
cargo test --package bitvue-decode --test edge_case_tests boundary
cargo test --package bitvue-decode --test edge_case_tests concurrent
```

## Test Data Generation

### Location
```
/Users/hawk/Workspaces/bitvue/crates/bitvue-test-data/src/
```

### Usage

```rust
use bitvue_test_data::{create_test_frame, create_minimal_ivf};

// Generate test frame
let frame = create_test_frame();

// Generate test IVF
let ivf = create_minimal_ivf();
```

## Fuzzing

### Location
```
/Users/hawk/Workspaces/bitvue/fuzz/fuzz_targets/av1_decode.rs
```

### Running Fuzzer

```bash
cd /Users/hawk/Workspaces/bitvue
cargo fuzz run av1_decode
```

## Documentation

- **Full Analysis:** `/Users/hawk/Workspaces/bitvue/docs/edge_case_testing_analysis.md`
- **Implementation Summary:** `/Users/hawk/Workspaces/bitvue/docs/edge_case_testing_summary.md`
- **This Quick Reference:** `/Users/hawk/Workspaces/bitvue/docs/EDGE_CASE_TESTING_IMPLEMENTATION.md`

## Test Categories

| Category | Tests | File |
|----------|-------|------|
| Empty/Truncated Files | 4 | edge_case_tests.rs |
| Integer Overflow | 3 | edge_case_tests.rs |
| Boundary Resolutions | 6 | edge_case_tests.rs |
| Bit Depth | 6 | edge_case_tests.rs |
| Chroma Subsampling | 4 | edge_case_tests.rs |
| Missing Chroma Planes | 4 | edge_case_tests.rs |
| Maximum Limits | 2 | edge_case_tests.rs |
| Panic Prevention | 3 | edge_case_tests.rs |
| Stride Handling | 5 | edge_case_tests.rs |
| Concurrent Safety | 2 | edge_case_tests.rs |
| **Total** | **39+** | |

## Coverage Goals

| Metric | Current | Target |
|--------|---------|--------|
| Edge Case Coverage | 60% | 85% |
| Security Tests | 55% | 90% |
| Boundary Conditions | 70% | 90% |
| Concurrent Safety | 45% | 80% |
