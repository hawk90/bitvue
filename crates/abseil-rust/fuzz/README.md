# Fuzz Tests for Abseil-Rust

This directory contains fuzz tests for the abseil-rust hash functions and related data structures.

## Prerequisites

Install `cargo-fuzz`:

```bash
cargo install cargo-fuzz
```

## Running Fuzz Tests

### Run all fuzz targets

```bash
cd fuzz
cargo fuzz run
```

### Run specific fuzz targets

```bash
# FNV hash
cargo fuzz run fnv_hash

# MurmurHash3
cargo fuzz run murmur3_64

# xxHash64
cargo fuzz run xxhash_64

# xxHash3
cargo fuzz run xxhash3_64

# HighwayHash
cargo fuzz run highway_hash

# WyHash
cargo fuzz run wyhash

# DJB2
cargo fuzz run djb2_hash

# SipHash
cargo fuzz run siphash_24

# Determinism tests (all hash functions)
cargo fuzz run hash_determinism

# Empty/single byte edge cases
cargo fuzz run hash_empty_single

# Boundary condition tests
cargo fuzz run hash_boundaries

# Bloom filter
cargo fuzz run bloom_filter
```

### Run with specific corpus or timeout

```bash
# Run for 60 seconds
cargo fuzz run fnv_hash -- -timeout=60

# Run with existing corpus
cargo fuzz run fnv_hash corpus/
```

## Fuzz Targets

| Target | Description | Edge Cases Covered |
|--------|-------------|-------------------|
| `fnv_hash` | FNV-32, FNV-64, FNV-128 | Determinism, overflow |
| `murmur3_64` | MurmurHash3 64-bit | 16-byte chunks, remaining bytes |
| `xxhash_64` | xxHash 64-bit | 32-byte chunks, seed variations |
| `xxhash3_64` | xxHash3 64-bit | 4-stage remaining bytes (8/16/24/32) |
| `highway_hash` | HighwayHash | 32-byte blocks, 4 u64 values |
| `wyhash` | WyHash | 16-byte chunks, tail processing |
| `djb2_hash` | DJB2 | Empty input, wrapping arithmetic |
| `siphash_24` | SipHash-2-4 | Key handling, 8-byte chunks |
| `hash_determinism` | All hash functions | Consistency check |
| `hash_empty_single` | Edge cases | Empty, single byte, boundaries |
| `hash_boundaries` | Boundary conditions | +/- 2 bytes at all chunk boundaries |
| `bloom_filter` | BloomFilter | Index overflow, division by zero |

## What the Fuzz Tests Check

### Safety Properties
- **No panics** on any input (including empty, single byte, very large inputs)
- **No undefined behavior** (out-of-bounds access, use-after-free)
- **No integer overflow** that causes incorrect behavior

### Correctness Properties
- **Determinism**: Same input + same seed = same output
- **Idempotency**: Multiple calls with same input produce same result
- **No false negatives** (for BloomFilter)

### Edge Cases Covered
- Empty input (`&[]`)
- Single byte
- Exact block boundaries (8, 16, 32 bytes)
- One byte past block boundaries
- Maximum remaining bytes before next block (7, 15, 23, 31)
- Large inputs (> 1MB)
- Seeds at edge values (0, MAX, etc.)

## Adding New Fuzz Targets

1. Create a new file in `fuzz_targets/`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Your fuzz test code here
    // Use `data` as input to your function
});
```

2. Add to `Cargo.toml`:

```toml
[[bin]]
name = "your_target_name"
path = "fuzz_targets/your_target_name.rs"
```

3. Run:

```bash
cargo fuzz run your_target_name
```

## Continuous Fuzzing

For CI/CD, you can run fuzz tests for a limited time:

```bash
# Run for 5 minutes each
cargo fuzz run fnv_hash -- -max_total_time=300
cargo fuzz run murmur3_64 -- -max_total_time=300
```

## Corpus Management

Save interesting test cases:

```bash
# Minimize corpus
cargo fuzz cmin fnv_hash

# Display coverage
cargo fuzz coverage fnv_hash
```

## Known Issues Found by Fuzzing

The fuzz tests helped identify and verify fixes for:

1. **Integer overflow in BloomFilter index calculation** - Fixed with `wrapping_mul`
2. **Bounds checking in hash algorithms** - Added safe array access
3. **Empty input handling** - All hash functions now handle `&[]` correctly
4. **Boundary condition bugs** - Tested at +/- 2 bytes from all chunk boundaries

## References

- [libFuzzer Documentation](https://llvm.org/docs/LibFuzzer.html)
- [cargo-fuzz README](https://github.com/rust-fuzz/cargo-fuzz)
