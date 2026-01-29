# Performance Issues - Remaining TODO

## Summary
- **Status**: 5 CRITICAL/HIGH issues fixed ✅
- **Remaining**: 4 MEDIUM issues
- **Priority**: Optional optimizations, not blocking

---

## MEDIUM Priority (4 issues)

### 1. String Allocations in Hot Paths
**Impact**: Unnecessary allocations in performance-critical code
**Locations**:
- `crates/bitvue-decode/src/decoder.rs` - Error formatting in decode loop
- `crates/bitvue-av1/src/parser.rs` - String formatting in parse_obu
- Logging macros with string interpolation

**Current Code**:
```rust
// Hot path - allocates even if log disabled
tracing::debug!("Decoded frame: {}x{} {}bit", width, height, bit_depth);

// Error formatting allocates
return Err(DecodeError::Decode(format!(
    "Frame size {} invalid", size
)));
```

**Fix**:
```rust
// Use lazy formatting
tracing::debug!(
    width = width,
    height = height,
    bit_depth = bit_depth,
    "Decoded frame"
);

// Pre-allocate error strings for common cases
const ERR_INVALID_SIZE: &str = "Frame size invalid";
return Err(DecodeError::Decode(ERR_INVALID_SIZE.into()));
```

**Estimated effort**: 3-4 hours
**Impact**: 5-10% reduction in allocations
**Benchmark**: Decode 1000 frames, measure allocation count

---

### 2. Vec Resizing in Loops
**Impact**: Multiple reallocations when capacity known upfront
**Locations**:
- `crates/bitvue-decode/src/decoder.rs:348` - decoded_frames Vec
- `crates/bitvue-av1/src/parser.rs` - OBU collection
- Plane data accumulation

**Current Code**:
```rust
let mut decoded_frames = Vec::new();
while offset < data.len() {
    // ... decode frame ...
    decoded_frames.push(frame); // May reallocate
}
```

**Fix**:
```rust
// Estimate capacity from IVF header or file size
let estimated_frames = data.len() / avg_frame_size;
let mut decoded_frames = Vec::with_capacity(estimated_frames);

// Or use collect() when possible
let decoded_frames: Vec<_> = frames_iter.collect();
```

**Estimated effort**: 2-3 hours
**Impact**: Eliminate 2-5 reallocations per decode
**Benchmark**: Memory allocations during IVF decode

---

### 3. Redundant Clones in Non-Critical Paths
**Impact**: Convenience clones that could be avoided
**Locations**:
- Config structs cloned when passed to threads
- Error types cloned unnecessarily
- String clones in non-hot paths

**Analysis Required**:
```rust
// Audit these patterns:
fn process(config: Config) {}           // Takes by value
fn process(config: &Config) {}          // Takes by ref
let c = config.clone(); thread::spawn() // Could Arc

// Error clones
let err = error.clone();
return Err(err); // Could return original
```

**Fix Strategy**:
1. Profile to identify clones in hot paths
2. Use `Arc` for shared read-only data
3. Use references where possible
4. Document intentional clones

**Estimated effort**: 4-5 hours
**Impact**: 2-5% reduction in allocations
**Benchmark**: Flame graph to identify clone hotspots

---

### 4. Debug Logging Overhead
**Impact**: Logging macros evaluated even when disabled
**Locations**:
- Extensive tracing in decode loop
- Debug format strings computed unconditionally
- Expensive debug assertions

**Current Code**:
```rust
// Even with logging disabled, these evaluate:
tracing::debug!(
    "Decoded {}x{} frame with {} bytes",
    width,
    height,
    calculate_size() // ❌ Called even if debug disabled
);
```

**Fix**:
```rust
// Use lazy evaluation
tracing::debug!(
    width = width,
    height = height,
    size = tracing::field::debug(|| calculate_size()),
    "Decoded frame"
);

// Or feature-gate expensive logging
#[cfg(feature = "debug-logging")]
tracing::trace!("Expensive debug info: {:?}", data);
```

**Estimated effort**: 2-3 hours
**Impact**: 3-5% speedup with logging disabled
**Benchmark**: Decode performance with/without logging

---

## Optimization Ideas (Not Blocking)

### 5. SIMD Optimizations
**Locations**:
- YUV to RGB conversion
- Plane data copying
- Already has NEON support, could add AVX2

**Estimated effort**: 8-10 hours
**Impact**: 20-40% faster color conversion

---

### 6. Parallel Frame Decoding
**Opportunity**: Decode multiple IVF frames in parallel
**Complexity**: High (needs thread-safe decoders)
**Estimated effort**: 10-15 hours
**Impact**: Near-linear speedup with frame count

---

### 7. Memory Pool for Frames
**Opportunity**: Reuse frame buffers instead of allocating
**Implementation**: Ring buffer of pre-allocated frames
**Estimated effort**: 6-8 hours
**Impact**: Eliminate frame allocation overhead

---

### 8. Zero-Copy Frame Export
**Opportunity**: Export frames without copying planes
**Challenge**: Lifetime management, API changes
**Estimated effort**: 10-12 hours
**Impact**: 30-50% faster frame export

---

## Benchmarking Suite

Create comprehensive benchmarks:

```rust
// benches/decode_performance.rs
#[bench]
fn decode_1080p_av1_30fps(b: &mut Bencher) {
    let data = load_test_video("1080p_30fps.ivf");
    b.iter(|| {
        let mut decoder = Av1Decoder::new().unwrap();
        decoder.decode_all(&data)
    });
}

#[bench]
fn decode_4k_av1_60fps(b: &mut Bencher) { ... }

#[bench]
fn overlay_extraction_qp(b: &mut Bencher) { ... }

#[bench]
fn overlay_extraction_mv(b: &mut Bencher) { ... }
```

**Estimated effort**: 4-5 hours
**Impact**: Continuous performance monitoring

---

## Priority Order

1. **#4 - Debug logging** (easy fix, measurable improvement)
2. **#2 - Vec resizing** (straightforward, good gains)
3. **#1 - String allocations** (moderate effort, good impact)
4. **#3 - Redundant clones** (requires profiling, case-by-case)
5. **Benchmarking suite** (enables future optimization)
6. **Advanced optimizations** (SIMD, parallel, zero-copy)

---

## Completed (5 issues) ✅

1. ✅ Tile data clones (1-10 MB per extraction) - Arc optimization
2. ✅ Plane extraction reallocations (1000+ per frame) - Pre-allocation
3. ✅ Frame data to_vec (50-500 KB per frame) - Explicit ownership
4. ✅ O(n²) QP lookup (510k comparisons) - Spatial index
5. ✅ IVF frame cloning (determined unavoidable due to dav1d API)

---

## Performance Targets

### Current Performance (estimated)
- 1080p AV1 decode: ~60 fps
- 4K AV1 decode: ~15 fps
- Overlay extraction: ~5ms per frame

### Target Performance (with remaining optimizations)
- 1080p AV1 decode: ~70 fps (+15%)
- 4K AV1 decode: ~18 fps (+20%)
- Overlay extraction: ~3ms per frame (+40%)

---

## Profiling Tools

```bash
# CPU profiling
cargo flamegraph --bin bitvue-cli -- decode video.ivf

# Memory profiling
cargo instruments --bin bitvue-cli --template Allocations

# Criterion benchmarks
cargo bench

# Performance tests
cargo test --release --test perf_test
```
