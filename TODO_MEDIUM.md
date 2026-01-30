# MEDIUM Severity Issues - Moderate Priority

## Summary
- **Total Issues**: 20+
- **Security**: 4
- **Performance**: 5
- **Bugs**: 6
- **Refactoring**: 5+
- **Estimated Total Effort**: 60-80 hours

**Priority**: Address after CRITICAL and HIGH issues, or as time permits.

---

## SECURITY Issues

### 1. Potential Overflow in Chroma Format Detection
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-decode/src/decoder.rs:105-108`
**CVSS**: 4.3 (MEDIUM)
**Effort**: 1-2 hours

#### Description
Chroma format detection calculates plane sizes without overflow protection before comparison.

#### Current Code
```rust
if uv_size == (width / 2) * height * bytes_per_sample {
    Self::Yuv422
} else if uv_size == (width / 2) * (height / 2) * bytes_per_sample {
    Self::Yuv420
}
```

#### Issue
If `width` or `height` is very large, `(width / 2) * height` could overflow before comparison.

#### Fix Required
```rust
// Use checked_mul for overflow protection
let size_422 = (width / 2)
    .checked_mul(height)
    .and_then(|v| v.checked_mul(bytes_per_sample as u32));

let size_420 = (width / 2)
    .checked_mul(height / 2)
    .and_then(|v| v.checked_mul(bytes_per_sample as u32));

if let Some(expected_422) = size_422 {
    if uv_size == expected_422 as usize {
        return Ok(Self::Yuv422);
    }
}

if let Some(expected_420) = size_420 {
    if uv_size == expected_420 as usize {
        return Ok(Self::Yuv420);
    }
}
```

---

### 2. MP4 Box Size Validation Gap
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-formats/src/mp4.rs:94-100, 438-439`
**CVSS**: 5.3 (MEDIUM)
**Effort**: 1-2 hours

#### Description
Box size is validated to be >= header_size, but no validation that `box_end` doesn't overflow or exceed file size.

#### Current Code
```rust
let box_end = box_start + header.data_size();
// Seek to box end without validation
cursor.seek(SeekFrom::Start(box_end))?;
```

#### Fix Required
```rust
let box_end = box_start
    .checked_add(header.data_size())
    .ok_or_else(|| BitvueError::InvalidData("Box end offset overflow".to_string()))?;

if box_end > data.len() as u64 {
    return Err(BitvueError::InvalidData("Box extends beyond file".to_string()));
}

cursor.seek(SeekFrom::Start(box_end))?;
```

---

### 3. Missing Timeout Enforcement in run_with_timeout
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-decode/src/vvdec.rs:266-274`
**CVSS**: 5.3 (MEDIUM)
**Effort**: 2-3 hours

#### Description
Function has "timeout" in name but performs blocking `.join()` with no timeout. It's unused but could be mistakenly used.

#### Current Code
```rust
fn run_with_timeout<F, T>(f: F) -> FfiResult<T> {
    thread::spawn(move || FfiResult::Success(f()))
        .join()
        .unwrap_or(FfiResult::Panic)
}
```

#### Fix Required
```rust
// Option 1: Remove unused function
// (Recommended if truly unused)

// Option 2: Implement actual timeout
fn run_with_timeout<F, T>(f: F, timeout: Duration) -> FfiResult<T>
where
    F: Send + 'static,
    T: Send + 'static,
{
    let handle = thread::spawn(move || FfiResult::Success(f()));
    let start = Instant::now();

    loop {
        if handle.is_finished() {
            return handle.join().unwrap_or(FfiResult::Panic);
        }
        if start.elapsed() >= timeout {
            // Note: Thread continues running in background
            // This is a known limitation
            return FfiResult::Timeout;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
```

#### Recommendation
Remove the unused function to prevent accidental use.

---

### 4. LEB128 Shift Value Overflow Edge Case
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/leb128.rs:35-55`
**CVSS**: 3.1 (LOW/MEDIUM)
**Effort**: 1 hour

#### Description
Overflow check `shift >= 64` happens AFTER shift addition. If `MAX_LEB128_BYTES` were increased, this could be problematic.

#### Fix Required
```rust
// Add assertion to catch future changes
const MAX_LEB128_SHIFT: u32 = MAX_LEB128_BYTES as u32 * 7;
debug_assert!(MAX_LEB128_SHIFT <= 64, "LEB128 max shift exceeds 64 bits");

// Or use compile-time assertion
const _: () = assert!(MAX_LEB128_BYTES * 7 <= 64, "LEB128 max shift exceeds 64");
```

---

## PERFORMANCE Issues

### 5. Cache Return Clones Entire Vec
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/overlay_extraction/cache.rs:73`
**Impact**: 3 MB per cache hit
**Effort**: 1-2 hours

#### Description
Cache hit still clones entire Vec<CodingUnit>, defeating caching purpose.

#### Current Code
```rust
if let Some(cached) = cache.get(&cache_key) {
    return Ok(cached.clone());  // Clones entire Vec
}
```

#### Fix Required
```rust
// Return Arc-wrapped vector
type CachedCodingUnits = Arc<Vec<CodingUnit>>;

// In cache:
cache.insert(cache_key, Arc::new(units));

// In retrieval:
if let Some(cached) = cache.get(&cache_key) {
    return Ok(cached.clone());  // Arc::clone is O(1)
}
```

#### Expected Improvement
- Eliminate 3 MB copy per cache hit
- O(1) cache hit instead of O(n)

---

### 6. Unnecessary .collect() Allocations
**Severity**: MEDIUM
**Status**: TODO
**Files**: Multiple
**Effort**: 2-3 hours

#### Locations

**6a. cache.rs:93**
```rust
let keys_to_remove: Vec<_> = cache.keys().take(remove_count).copied().collect();
```

**Fix:**
```rust
// Use VecDeque with pre-allocated buffer
let mut keys_to_remove = VecDeque::with_capacity(remove_count);
for key in cache.keys().take(remove_count) {
    keys_to_remove.push_back(*key);
}
```

**6b. dependency.rs**
```rust
let mut obu_indices: Vec<usize> = required_obu_indices.into_iter().collect();
```

**Fix:**
```rust
// Use iterators directly or extend existing Vec
let mut obu_indices = Vec::with_capacity(required_obu_indices.len());
obu_indices.extend(required_obu_indices);
```

---

### 7. OBU Payload Allocation
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/obu.rs:254`
**Impact**: 1-10 MB per frame
**Effort**: 3-4 hours

#### Description
Every OBU creates a new allocation for payload.

#### Current Code
```rust
let payload = slice[payload_start..total_size].to_vec();
```

#### Fix Required
```rust
// Use Arc<[u8]> for shared ownership
pub struct Obu {
    pub payload: Arc<[u8]>,
    // ...
}

// During parsing:
let payload: Arc<[u8]> = Arc::from(&slice[payload_start..total_size]);
```

#### Expected Improvement
- Reduce allocations by 50-90%
- Enable zero-copy sharing where possible

---

### 8. Nested Loops in Partition Extraction
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/overlay_extraction/partition.rs`
**Impact**: 260K iterations for 1080p
**Effort**: 4-6 hours

#### Description
Nested loops for grid building: O(sb_rows × sb_cols × grid_h × grid_w).

#### Current Code
```rust
for sb_y in 0..parsed.dimensions.sb_rows {
    for sb_x in 0..parsed.dimensions.sb_cols {
        // ...
        for grid_y in 0..grid_h {
            for grid_x in 0..grid_w {
```

#### Analysis
```
For 1080p with 64px blocks:
- sb_rows: 17
- sb_cols: 30
- grid_h: 17
- grid_w: 30
- Total: 17 × 30 × 17 × 30 = 260,100 iterations
```

#### Potential Optimizations
```rust
// Option 1: Flatten loops
for sb_idx in 0..(sb_rows * sb_cols) {
    let sb_y = sb_idx / sb_cols;
    let sb_x = sb_idx % sb_cols;
    // ...
}

// Option 2: Use itertools
use itertools::iproduct;

for (sb_y, sb_x, grid_y, grid_x) in iproduct!(
    0..sb_rows,
    0..sb_cols,
    0..grid_h,
    0..grid_w
) {
    // ...
}

// Option 3: Parallel processing with rayon
use rayon::prelude::*;

(0..sb_rows).into_par_iter().for_each(|sb_y| {
    (0..sb_cols).into_par_iter().for_each(|sb_x| {
        // Process superblock in parallel
    });
});
```

#### Recommendation
Profile first to confirm this is actually a bottleneck before optimizing.

---

### 9. Cache Key Hashing O(n)
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/overlay_extraction/cache.rs:47-55`
**Impact**: 1ms per lookup for 10 MB tiles
**Effort**: 2-3 hours

#### Description
Hash computed on entire tile_data for every cache lookup.

#### Current Code
```rust
pub fn compute_cache_key(tile_data: &[u8], base_qp: i16) -> u64 {
    let mut hasher = DefaultHasher::new();
    tile_data.hash(&mut hasher);  // O(n) where n = 1-10 MB
    base_qp.hash(&mut hasher);
    hasher.finish()
}
```

#### Fix Required
```rust
// Use XXH3 or similar fast hash (5-10x faster)
use twox_hash::XxHash64;

pub fn compute_cache_key(tile_data: &[u8], base_qp: i16) -> u64 {
    use std::hash::Hasher;
    let mut hasher = XxHash64::with_seed(0);
    tile_data.hash(&mut hasher);  // Much faster for large data
    base_qp.hash(&mut hasher);
    hasher.finish()
}
```

#### Expected Improvement
- 5-10x faster hashing for large tiles
- Reduced cache miss overhead

---

## BUG Issues

### 10. Missing Error Propagation in Overlay Extraction
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/overlay_extraction/parser.rs:101`
**Effort**: 1-2 hours

#### Description
Silently defaults to empty OBUs on parse failure, masking critical errors.

#### Current Code
```rust
let obus_vec = parse_all_obus(&obu_data).unwrap_or_default();
```

#### Fix Required
```rust
// Option 1: Propagate error
let obus_vec = parse_all_obus(&obu_data)?;

// Option 2: Log and fallback with context
let obus_vec = match parse_all_obus(&obu_data) {
    Ok(obus) => obus,
    Err(e) => {
        tracing::warn!(
            "Failed to parse OBUs for frame {}: {}, using fallback",
            frame_index, e
        );
        return Ok(create_fallback_pixel_info(frame_index));
    }
};
```

---

### 11. Inconsistent Timestamp Calculations
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/overlay_extraction/parser.rs:283-286`
**Effort**: 2-3 hours

#### Description
"Estimated" values with no basis in actual stream data.

#### Current Code
```rust
let est_frame_size = 25000;  // Magic number!
let bit_offset = Some((frame_index as u64) * 200000 + (pixel_y as u64) * 1920 + pixel_x as u64);
let byte_offset = Some((frame_index as u64) * est_frame_size);
```

#### Fix Required
```rust
// Option 1: Calculate from actual stream metadata
pub struct TimestampMetadata {
    frame_size_bytes: u64,
    bit_rate: u64,
    frame_rate: u64,
}

// Option 2: Remove estimates entirely
// Either provide real data or None
let (bit_offset, byte_offset) = if let Some(metadata) = &frame_metadata {
    (
        Some(calculate_real_bit_offset(metadata, frame_index, pixel_y, pixel_x)),
        Some(calculate_real_byte_offset(metadata, frame_index))
    )
} else {
    (None, None)  // Don't provide fake data
};
```

---

### 12. Unsafe FFI Call Without Lifetime Validation
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-decode/src/vvdec.rs:535-537`
**Effort**: 2-3 hours

#### Description
While `plane.ptr` is validated non-null, there's no validation that memory is valid for slice lifetime.

#### Current Code
```rust
let plane_slice = unsafe {
    std::slice::from_raw_parts(plane.ptr, total_buffer_size)
};
```

#### Fix Required
```rust
// Add comprehensive safety documentation
/// # Safety
///
/// This creates a slice from vvdec decoder plane data. The following invariants must hold:
///
/// 1. `plane.ptr` must be non-null (validated above)
/// 2. `plane.ptr` must be valid for reading `total_buffer_size` bytes
/// 3. The decoder mutex must be held for the entire lifetime of the slice
/// 4. No concurrent decode operations must occur while slice is alive
///
/// # Lifetime Considerations
///
/// The slice borrows from decoder-internal memory. The decoder must not be:
/// - Reset while slice is in use
/// - Used for concurrent decoding
/// - Destroyed before slice goes out of scope
///
/// # Usage Pattern
///
/// ```rust
/// let guard = decoder.lock()?;
/// let slice = unsafe { extract_plane_slice(&guard.plane) };
/// // Use slice here while guard is held
/// drop(guard);  // Slice becomes invalid after this
/// ```
let plane_slice = unsafe {
    std::slice::from_raw_parts(plane.ptr, total_buffer_size)
};

// Ensure slice doesn't escape decoder lock scope
// Consider using a RAII guard that ties slice lifetime to mutex guard
```

---

### 13. Debug Assertions in Release Builds
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/symbol/cdf.rs:230,264`
**Effort**: 1-2 hours

#### Description
Assertions will panic in debug but are disabled in release, allowing invalid CDF tables.

#### Current Code
```rust
assert_eq!(*mv_joint_cdf.last().unwrap(), CDF_SCALE);
assert_eq!(*mv_class_cdf.last().unwrap(), CDF_SCALE);
```

#### Fix Required
```rust
// Replace with runtime checks that work in all builds
if *mv_joint_cdf.last().unwrap_or(&0) != CDF_SCALE {
    return Err(BitvueError::InvalidData(format!(
        "MV joint CDF last value must be {}: got {}",
        CDF_SCALE,
        mv_joint_cdf.last().unwrap_or(&0)
    )));
}

if *mv_class_cdf.last().unwrap_or(&0) != CDF_SCALE {
    return Err(BitvueError::InvalidData(format!(
        "MV class CDF last value must be {}: got {}",
        CDF_SCALE,
        mv_class_cdf.last().unwrap_or(&0)
    )));
}
```

---

### 14. Potential Infinite Loop in Partition Parsing
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/tile/partition.rs:356-379`
**Effort**: 2-3 hours

#### Description
If `sub_block_size` returns same size as parent, could create infinite recursion.

#### Current Code
```rust
if partition != PartitionType::None {
    let sub_sizes = block_size.sub_block_size(partition);

    for (i, sub_size) in sub_sizes.iter().enumerate() {
        let child = parse_partition_recursive(/* ... */)?;
        node.children.push(child);
    }
}
```

#### Issue
Fallback case (line 193) doesn't prevent same-size return.

#### Fix Required
```rust
// Add recursion depth limit
const MAX_PARTITION_DEPTH: u8 = 10;

fn parse_partition_recursive(
    data: &ParsedTile,
    node: &SyntaxNode,
    depth: u8,
) -> Result<PartitionNode, BitvueError> {
    // Validate depth
    if depth > MAX_PARTITION_DEPTH {
        return Err(BitvueError::Decode(
            format!("Partition recursion depth exceeds maximum: {}", depth)
        ));
    }

    if partition != PartitionType::None {
        let sub_sizes = block_size.sub_block_size(partition);

        // Validate children are strictly smaller
        for sub_size in &sub_sizes {
            if sub_size.width() >= block_size.width() || sub_size.height() >= block_size.height() {
                return Err(BitvueError::Decode(
                    format!("Child partition not smaller than parent: {:?} vs {:?}",
                        sub_size, block_size)
                ));
            }
        }

        for (i, sub_size) in sub_sizes.iter().enumerate() {
            let child = parse_partition_recursive(
                data,
                /* ... */
                depth + 1,  // Increment depth
            )?;
            node.children.push(child);
        }
    }

    Ok(node)
}
```

---

### 15. Frame Header Magic Number (40 bytes)
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/frame_header.rs:150`
**Effort**: 4-6 hours

#### Description
Adding magic number `40` to account for "unparsed fields" is fragile.

#### Current Code
```rust
let header_size_bytes = base_header_bytes + 40;  // Magic number!
```

#### Issue
Actual size depends on many conditional fields. Brittle if format changes.

#### Fix Required
```rust
// Parse all intermediate fields properly
pub fn parse_frame_header(reader: &mut BitReader) -> Result<(FrameHeader, usize), BitvueError> {
    let start_pos = reader.bit_pos();

    // Parse all fields in order
    let show_existing_frame = reader.read_bool()?;
    let frame_type = reader.read_bits(2)?;
    let show_frame = reader.read_bool()?;

    // ... parse ALL intermediate fields ...

    let end_pos = reader.bit_pos();
    let header_size_bits = end_pos - start_pos;
    let header_size_bytes = (header_size_bits + 7) / 8;

    Ok((FrameHeader { /* ... */ }, header_size_bytes))
}
```

---

## REFACTORING Issues

### 16. Swallowed Errors with Silent Fallback
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/bitvue-av1/src/overlay_extraction/partition.rs:77-80`
**Effort**: 2-3 hours

#### Description
Errors silently downgraded to fallback behavior, making it hard to distinguish between:
- Intentional fallback (feature not implemented)
- Parse failure (corrupt data)
- Implementation bug

#### Current Code
```rust
Err(e) => {
    tracing::warn!("Failed to parse coding units for QP: {}, using base_qp", e);
    // Fall through to scaffold - error is logged but not propagated
}
```

#### Fix Required
```rust
pub enum ParseStrategy {
    Strict,      // Fail on any error
    BestEffort,  // Log and fallback
    Silent,      // Silently fallback
}

pub fn extract_qp_grid_with_strategy(
    parsed: &ParsedFrame,
    strategy: ParseStrategy,
) -> Result<QPGrid, BitvueError> {
    match parse_all_coding_units(parsed) {
        Ok(coding_units) => { /* ... */ }
        Err(e) => match strategy {
            ParseStrategy::Strict => Err(e),
            ParseStrategy::BestEffort => {
                tracing::warn!("Failed to parse CUs: {}, using fallback", e);
                Ok(create_fallback_qp_grid(parsed))
            }
            ParseStrategy::Silent => Ok(create_fallback_qp_grid(parsed)),
        }
    }
}

// Default to strict in production
let qp_grid = extract_qp_grid_with_strategy(parsed, ParseStrategy::Strict)?;
```

---

### 17. Primitive Obsession
**Severity**: MEDIUM
**Status**: TODO
**Files**: Multiple
**Effort**: 8-10 hours

#### Description
Domain concepts use primitive types without validation.

#### Examples
```rust
pub qp: i16,           // What's the valid range?
pub timestamp: i64,    // What unit?
pub width: u32,        // What constraints?
pub mv_x: i32,         // Quarter-pel? Integer-pel?
```

#### Refactoring - Newtype Pattern

```rust
// Quality Parameter
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Qp(i16);

impl Qp {
    pub const MIN: i16 = 0;
    pub const MAX: i16 = 255;
    pub const DEFAULT: i16 = 32;

    pub fn new(value: i16) -> Result<Self, BitvueError> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(BitvueError::InvalidData(
                format!("QP out of range: {} (valid: {}-{})", value, Self::MIN, Self::MAX)
            }))
        }
    }

    pub fn value(self) -> i16 { self.0 }
}

// Motion Vector (quarter-pel precision)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct QuarterPel(i32);

impl QuarterPel {
    pub fn from_qpel(qpel: i32) -> Self { Self(qpel) }
    pub fn from_pel(pel: i32) -> Self { Self(pel * 4) }
    pub fn qpel(self) -> i32 { self.0 }
    pub fn pel(self) -> i32 { self.0 / 4 }
}

// Timestamp
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TimestampPts(i64);

impl TimestampPts {
    pub fn new(pts: i64) -> Result<Self, BitvueError> {
        if pts >= 0 {
            Ok(Self(pts))
        } else {
            Err(BitvueError::InvalidData(
                format!("Invalid PTS: {} (must be >= 0)", pts)
            ))
        }
    }

    pub fn value(self) -> i64 { self.0 }
}
```

---

### 18. SRP Violation in BitvueApp
**Severity**: MEDIUM
**Status**: TODO
**File**: `crates/app/src/bitvue_app.rs`
**Effort**: 6-8 hours

#### Description
`BitvueApp` struct has too many responsibilities.

#### Current Responsibilities
- UI layout management
- Notification handling
- Recent files management
- Settings management
- Worker coordination
- Panel registry
- Workspace registry

#### Refactoring
```rust
// Split into focused components

pub struct BitvueApp {
    core: Arc<Core>,
    ui: UiStateManager,
    workers: WorkerCoordinator,
    settings: AppSettingsManager,
}

pub struct UiStateManager {
    dock_state: DockState<PanelTab>,
    panels: PanelRegistry,
    workspaces: WorkspaceRegistry,
    notifications: NotificationManager,
}

impl UiStateManager {
    pub fn add_notification(&mut self, notification: Notification) { /* ... */ }
    pub fn get_panel(&mut self, id: PanelId) -> Option<&mut dyn Panel> { /* ... */ }
}

pub struct WorkerCoordinator {
    decoder: DecodeCoordinator,
    parser: ParseCoordinator,
    bytecache: ByteCacheWorker,
    export: ExportWorker,
    config: ConfigWorker,
}

impl WorkerCoordinator {
    pub fn poll_results(&mut self) -> Vec<WorkerResult> { /* ... */ }
    pub fn submit_decode(&mut self, job: DecodeJob) -> Result<()> { /* ... */ }
}
```

---

### 19. Dead Code: Unused Functions
**Severity**: LOW (but affects maintainability)
**Status**: TODO
**Files**: Multiple
**Effort**: 1-2 hours

#### Examples

**19a. obu.rs:415-428**
```rust
// Unused: ObuIterator::next_obu_with_offset duplicates logic from next
pub fn next_obu_with_offset(&mut self) -> Option<Result<ObuWithOffset>> {
    // Remove or refactor to share implementation
}
```

**19b. mkv.rs:21**
```rust
// Unused constant
const STREAM_TYPE_AV1: u8 = 0x06; // Never used
```

#### Fix
```rust
// Option 1: Remove dead code
// Option 2: Add #[allow(dead_code)] if intentional
// Option 3: Implement AV1 stream detection in TS parser
```

---

### 20. Magic Numbers Without Constants
**Severity**: LOW
**Status**: TODO
**Files**: Multiple
**Effort**: 2-3 hours

#### Examples
```rust
// partition.rs:314
let block_w = 16u32;  // Magic number
let block_h = 16u32;

// decoder.rs:357
const MAX_FRAMES_PER_FILE: usize = 100_000;  // Should be in shared constants

// strategy/avx2.rs:196-214
let u_val = u_plane[uv_i] as f32 - 128.0;  // Chroma offset
```

#### Fix
```rust
// crates/bitvue-core/src/constants.rs

pub mod video {
    pub const MAX_FRAMES_PER_FILE: usize = 100_000;
    pub const MAX_FRAME_SIZE_BYTES: usize = 100 * 1024 * 1024;
}

pub mod blocks {
    pub const BLOCK_16X16: u32 = 16;
    pub const MAX_GRID_BLOCKS: usize = 512 * 512;
}

pub mod yuv {
    pub const CHROMA_OFFSET: f32 = 128.0;
}

// Usage
use bitvue_core::constants::{blocks, video, yuv};

let block_w = blocks::BLOCK_16X16;
let chroma_value = u_val - yuv::CHROMA_OFFSET;
```

---

## Priority Order

### Security (Quick Wins)
1. **Chroma overflow** (#1) - 1-2 hours
2. **MP4 box validation** (#2) - 1-2 hours
3. **Remove run_with_timeout** (#3) - 1 hour
4. **LEB128 assertion** (#4) - 1 hour

### Performance (After HIGH priority)
5. **Cache Arc wrapper** (#5) - 1-2 hours
6. **Unnecessary .collect()** (#6) - 2-3 hours
7. **OBU Arc payload** (#7) - 3-4 hours
8. **Cache key hash** (#9) - 2-3 hours
9. **Nested loops** (#8) - 4-6 hours (profile first)

### Bugs (Stability)
10. **Error propagation** (#10) - 1-2 hours
11. **Timestamp calculations** (#11) - 2-3 hours
12. **FFI lifetime docs** (#12) - 2-3 hours
13. **Debug assertions** (#13) - 1-2 hours
14. **Recursion limit** (#14) - 2-3 hours
15. **Frame header parsing** (#15) - 4-6 hours

### Refactoring (Maintainability)
16. **Parse strategy** (#16) - 2-3 hours
17. **Newtype patterns** (#17) - 8-10 hours
18. **Split BitvueApp** (#18) - 6-8 hours
19. **Remove dead code** (#19) - 1-2 hours
20. **Extract constants** (#20) - 2-3 hours

---

## Testing Checklist

- [ ] Performance benchmarks for cache improvements
- [ ] Unit tests for all bug fixes
- [ ] Integration tests for refactoring
- [ ] Add property-based tests for edge cases
- [ ] Update documentation for newtypes

---

## Completion Criteria

MEDIUM issues can be addressed incrementally. Each issue should:
1. Have unit test demonstrating issue
2. Have fix implemented
3. Have regression test added
4. Pass full test suite
5. Update relevant documentation
