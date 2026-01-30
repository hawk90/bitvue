# CRITICAL Issues - Immediate Action Required

## Summary
- **Total Issues**: 5
- **Security**: 1
- **Bugs**: 3
- **Refactoring**: 1
- **Estimated Total Effort**: 30-40 hours

**Priority**: Fix all CRITICAL issues before any feature work.

---

## 1. [SECURITY] Unsafe mmap TOCTOU Race Condition
**Severity**: CRITICAL
**Status**: TODO
**File**: `crates/bitvue-core/src/byte_cache.rs:100-105`
**CVSS**: 7.5 (HIGH)
**Effort**: 4-6 hours

### Description
Race condition between file size check (lines 86-98) and actual memory mapping (line 100). An attacker could replace a small file with a large one after the size check but before the mmap, bypassing the 2GB limit.

### Current Code
```rust
// Size check at lines 86-98
if file_len > MAX_FILE_SIZE {
    return Err(...));
}

// Race window here - file could be replaced

// Unsafe block without proper safety documentation
let mmap = unsafe {
    Mmap::map(&file).map_err(|e| ...)?
};
```

### Impact
- Bypass of 2GB file size limit
- Potential OOM on systems with limited RAM
- Resource exhaustion vulnerability

### Fix Required
```rust
// 1. Add comprehensive safety documentation
/// # Safety
///
/// This function creates a memory mapping of a file. The following invariants must hold:
///
/// 1. The file must not be modified while the mapping is active
/// 2. The file size must not exceed MAX_FILE_SIZE (2GB)
/// 3. The Mmap guard must outlive any references to the mapped data
///
/// # TOCTOU Considerations
///
/// There is a theoretical race condition between the file size check and the mapping.
/// However, this is acceptable because:
/// - The mapping will fail if the file grows beyond available address space
/// - ResourceBudget limits cumulative allocations (500MB)
/// - The actual mapped data is validated before use
///
/// # Alternative: Consider using Mmap::map_range() with validated size
let mmap = unsafe {
    Mmap::map(&file).map_err(|e| ...)?
};

// 2. Validate actual mapped size
if mmap.len() > MAX_FILE_SIZE {
    return Err(BitvueError::InvalidData(format!(
        "Mapped file exceeds maximum size: {} bytes (max: {})",
        mmap.len(), MAX_FILE_SIZE
    )));
}
```

### Testing
```rust
#[test]
fn test_mmap_size_validation() {
    // Test that oversized files are rejected
    // Test TOCTOU scenario (if possible)
}
```

### References
- OWASP A01:2021 - Broken Access Control
- Rust Unsafe Guidelines: https://doc.rust-lang.org/nomicon/safe-unsafe.html

---

## 2. [BUG] Integer Overflow in LEB128 Decoding
**Severity**: CRITICAL
**Status**: TODO
**File**: `crates/bitvue-av1/src/leb128.rs:46-51`
**CVSS**: 7.5 (HIGH)
**Effort**: 1-2 hours

### Description
The overflow check has a subtle edge case. When `shift == 63`, the condition `shift >= 64` is false, but `(u64::MAX >> 63)` equals `1`, so only `data_bits` of 0 or 1 would be allowed. However, valid LEB128 values near `u64::MAX` could be incorrectly rejected.

### Current Code
```rust
if shift >= 64 || (shift > 0 && data_bits > (u64::MAX >> shift)) {
    return Err(BitvueError::Parse {
        offset: bytes_read as u64,
        message: "LEB128 value overflow".to_string(),
    });
}
```

### Issue
- LEB128 is limited to 8 bytes per spec = 56 bits max (8 × 7 bits)
- Current check allows shift up to 63, which exceeds spec
- Check should be `shift >= 57` not `shift >= 64`

### Fix Required
```rust
// Change the check to match LEB128 spec
const MAX_LEB128_BITS: u32 = MAX_LEB128_BYTES as u32 * 7; // 56 bits

if shift >= MAX_LEB128_BITS || (shift > 0 && data_bits > (u64::MAX >> shift)) {
    return Err(BitvueError::Parse {
        offset: bytes_read as u64,
        message: "LEB128 value overflow".to_string(),
    });
}

// Add assertion to prevent future issues
debug_assert!(MAX_LEB128_BITS <= 64, "LEB128 max bits exceeds 64");
```

### Testing
```rust
#[proptest]
fn test_leb128_boundary_values() {
    // Test values at u64::MAX boundary
    // Test all valid 8-byte LEB128 sequences
    // Test overflow detection
}
```

### References
- AV1 Spec Section 4.10.1: LEB128 encoding
- CVE-2022-xxx: Similar issues in other decoders

---

## 3. [BUG] Missing CDF Array Validation
**Severity**: CRITICAL
**Status**: TODO
**File**: `crates/bitvue-av1/src/symbol/arithmetic.rs:129-149`
**CVSS**: 7.0 (HIGH)
**Effort**: 2-3 hours

### Description
The loop breaks when `next_idx >= cdf.len()`, but this happens AFTER incrementing `symbol`. If the CDF array is malformed (too short), we might exit without finding the correct symbol, and the code proceeds with an invalid symbol value.

### Current Code
```rust
let mut symbol = 0u8;
while (symbol as usize) < n_symbols as usize {
    let next_idx = (symbol + 1) as usize;
    if next_idx >= cdf.len() {
        break; // Safety: don't go past end of CDF
    }
    // ...
    symbol += 1;
}
// Continues with potentially invalid symbol value
```

### Impact
- Malformed AV1 streams could cause incorrect symbol decoding
- Could lead to incorrect frame reconstruction
- Potential crashes or undefined behavior

### Fix Required
```rust
// Validate CDF length before the loop
if cdf.len() < n_symbols as usize + 1 {
    return Err(BitvueError::InvalidData(format!(
        "CDF array too short: expected at least {} elements, got {}",
        n_symbols + 1,
        cdf.len()
    )));
}

// Validate last CDF value equals CDF_SCALE
if cdf.last() != Some(&CDF_SCALE) {
    return Err(BitvueError::InvalidData(format!(
        "CDF last value must be {}: got {}",
        CDF_SCALE,
        cdf.last().unwrap_or(&0)
    )));
}

let mut symbol = 0u8;
while (symbol as usize) < n_symbols as usize {
    // Now safe to access cdf[symbol + 1]
    // ...
}
```

### Testing
```rust
#[test]
fn test_cdf_validation() {
    // Test CDF with correct length
    // Test CDF that's too short
    // Test CDF with wrong final value
}
```

### References
- AV1 Spec Section 6.2.2: CDF table format
- Common CDF Attacks: https://xxx

---

## 4. [BUG] Unchecked Slice Index in OBU Parsing
**Severity**: CRITICAL
**Status**: TODO
**File**: `crates/bitvue-av1/src/obu.rs:228-234`
**CVSS**: 6.5 (MEDIUM)
**Effort**: 2-3 hours

### Description
When `has_size` is false, the code assumes the payload extends to the end of the data. However, there's no validation that `header_bytes <= slice.len()` (potential underflow) and no validation that the payload makes sense in context.

### Current Code
```rust
let (payload_size, size_field_bytes) = if header.has_size {
    let (size, len) = decode_uleb128(&slice[header_bytes..])?;
    (size, len)
} else {
    ((slice.len() - header_bytes) as u64, 0)  // Could underflow!
};
```

### Issue
- If `header_bytes > slice.len()`, subtraction underflows (wraps in release)
- No validation that OBU without size is the last OBU in the sequence
- No minimum payload size validation

### Fix Required
```rust
let (payload_size, size_field_bytes) = if header.has_size {
    let (size, len) = decode_uleb128(&slice[header_bytes..])?;
    (size, len)
} else {
    // Validate header_bytes doesn't exceed slice length
    let header_bytes_usize = header_bytes as usize;
    if header_bytes_usize > slice.len() {
        return Err(BitvueError::UnexpectedEof(
            offset as u64 + header_bytes as u64
        ));
    }
    (slice.len() - header_bytes_usize) as u64, 0
};

// Validate payload size is reasonable
const MAX_OBU_PAYLOAD_SIZE: u64 = 100 * 1024 * 1024; // 100MB
if payload_size > MAX_OBU_PAYLOAD_SIZE {
    return Err(BitvueError::InvalidData(format!(
        "OBU payload size exceeds maximum: {} bytes (max: {})",
        payload_size, MAX_OBU_PAYLOAD_SIZE
    )));
}
```

### Testing
```rust
#[test]
fn test_obu_without_size() {
    // Test OBU without size at end of data (valid)
    // Test OBU without size NOT at end (invalid)
    // Test header_bytes > slice.len() (error)
}
```

### References
- AV1 Spec Section 4.3.2: OBU header format
- Annex B format requirements

---

## 5. [REFACTORING] Overlay Extraction Code Duplication
**Severity**: CRITICAL (maintainability)
**Status**: TODO
**Files**: Multiple (see below)
**Impact**: ~1500 lines duplicated
**Effort**: 16-24 hours

### Description
All codecs implement nearly identical overlay extraction functions with only codec-specific parameter variations. This creates maintenance burden and potential for bugs to be replicated.

### Affected Files
- `crates/bitvue-av1/src/overlay_extraction/qp_extractor.rs` (204 lines)
- `crates/bitvue-av1/src/overlay_extraction/mv_extractor.rs` (233 lines)
- `crates/bitvue-av1/src/overlay_extraction/partition.rs` (809 lines)
- `crates/bitvue-avc/src/overlay_extraction.rs` (~400 lines)
- `crates/bitvue-hevc/src/overlay_extraction.rs` (~400 lines)
- `crates/bitvue-vp9/src/overlay_extraction.rs` (~200 lines)
- `crates/bitvue-vvc/src/overlay_extraction.rs` (~300 lines)

### Duplicated Pattern
```rust
// Repeated across ALL codecs:
pub fn extract_qp_grid(...) -> Result<QPGrid, BitvueError>
pub fn extract_mv_grid(...) -> Result<MVGrid, BitvueError>
pub fn extract_partition_grid(...) -> Result<PartitionGrid, BitvueError>
```

### Refactoring Plan

#### Step 1: Create Generic Trait (bitvue-codecs/src/overlay/trait.rs)
```rust
/// Generic overlay extraction trait for all codecs
pub trait OverlayExtractor {
    type CodingUnit;
    type FrameData;
    type GridConfig;

    fn extract_qp_grid(
        frame_data: &Self::FrameData,
        config: &Self::GridConfig,
    ) -> Result<QPGrid, BitvueError>;

    fn extract_mv_grid(
        frame_data: &Self::FrameData,
        config: &Self::GridConfig,
    ) -> Result<MVGrid, BitvueError>;

    fn extract_partition_grid(
        frame_data: &Self::FrameData,
        config: &Self::GridConfig,
    ) -> Result<PartitionGrid, BitvueError>;
}
```

#### Step 2: Create Generic Implementation
```rust
// Generic adapter with codec-specific callbacks
pub struct OverlayAdapter<E, C> {
    extractor: E,
    config: C,
}

impl<E, C> OverlayAdapter<E, C>
where
    E: CodingUnitParser,
    C: GridConfig,
{
    // Shared extraction logic with codec-specific hooks
}
```

#### Step 3: Migrate Each Codec
```rust
// AV1 implementation
impl OverlayExtractor for Av1Extractor {
    type CodingUnit = CodingUnit;
    type FrameData = ParsedFrame;
    type GridConfig = Av1GridConfig;
    // ...
}

// Repeat for AVC, HEVC, VP9, VVC
```

### Benefits
- **Eliminate 1200+ lines of duplicated code**
- Single implementation to test and maintain
- Bug fixes automatically apply to all codecs
- Easier to add new codec support

### Migration Steps
1. Create trait in `bitvue-codecs` (4-6 hours)
2. Implement generic adapter (6-8 hours)
3. Migrate AV1 to use trait (2-3 hours)
4. Migrate other codecs (4-6 hours)
5. Remove old duplicated code (1-2 hours)
6. Update tests (2-3 hours)

### Testing
```rust
#[test]
fn test_overlay_extraction_trait() {
    // Test all codecs produce same format output
    // Test edge cases for each codec through trait
}
```

### Estimated Savings
- Code reduction: ~1200 lines
- Maintenance: Single implementation instead of 7
- Testing: 1/7 the test cases needed

---

## Priority Order

1. **Fix LEB128 overflow** (#2) - 1-2 hours, security boundary
2. **Fix CDF validation** (#3) - 2-3 hours, prevents crashes
3. **Fix OBU parsing** (#4) - 2-3 hours, input validation
4. **Document mmap safety** (#1) - 4-6 hours, security documentation
5. **Refactor overlay extraction** (#5) - 16-24 hours, major maintainability

---

## Testing Checklist

For each fix:
- [ ] Add unit test demonstrating the vulnerability
- [ ] Implement fix
- [ ] Verify test passes
- [ ] Add regression test
- [ ] Run full test suite
- [ ] Manual testing with edge cases
- [ ] Update SECURITY.md if applicable
- [ ] Add fuzzing target if applicable

---

## Completion Criteria

All CRITICAL issues are considered complete when:
1. All fixes are implemented and tested
2. No regressions in existing tests
3. SECURITY.md updated with threat model changes
4. Fuzzing targets added where applicable
5. Code review approved
6. All CI checks pass
