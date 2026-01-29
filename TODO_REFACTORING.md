# Refactoring Opportunities - TODO

## Summary
- **Purpose**: Improve code quality, maintainability, and testability
- **Priority**: LOW - Not blocking, incremental improvements
- **Approach**: Refactor opportunistically during feature work

---

## Code Duplication

### 1. `parse_all_coding_units` Duplication
**Issue**: Identical function in 3 files
**Locations**:
- `crates/bitvue-av1/src/overlay_extraction/qp_extractor.rs:141-221`
- `crates/bitvue-av1/src/overlay_extraction/mv_extractor.rs:145-225`
- `crates/bitvue-av1/src/overlay_extraction/partition.rs` (similar)

**Impact**: 80+ lines duplicated × 3 = 240 lines

**Fix**: Extract to shared module
```rust
// crates/bitvue-av1/src/overlay_extraction/cu_parser.rs
pub fn parse_all_coding_units(
    parsed: &ParsedFrame,
) -> Result<Vec<CodingUnit>> {
    let base_qp = parsed.frame_type.base_qp.unwrap_or(128) as i16;
    let cache_key = compute_cache_key(&parsed.tile_data, base_qp);

    get_or_parse_coding_units(cache_key, || {
        // ... shared implementation ...
    })
}
```

**Estimated effort**: 2 hours
**Benefits**: Single source of truth, easier maintenance

---

### 2. Spatial Index Duplication
**Issue**: Similar spatial index in qp_extractor and mv_extractor
**Locations**:
- `qp_extractor.rs:91-138` - `CuSpatialIndex`
- `mv_extractor.rs:18-65` - `build_cu_spatial_index`

**Impact**: ~50 lines duplicated

**Fix**: Extract to shared module
```rust
// crates/bitvue-av1/src/overlay_extraction/spatial_index.rs
pub struct CuSpatialIndex {
    grid: Vec<Option<usize>>,
    grid_w: u32,
}

impl CuSpatialIndex {
    pub fn new(
        coding_units: &[CodingUnit],
        grid_w: u32,
        grid_h: u32,
        block_w: u32,
        block_h: u32,
    ) -> Self { ... }

    pub fn get_cu_index(&self, grid_x: u32, grid_y: u32) -> Option<usize> { ... }
}
```

**Estimated effort**: 1 hour
**Benefits**: Consistent behavior, single implementation

---

### 3. OBU Parsing Patterns
**Issue**: Similar OBU iteration patterns
**Locations**:
- `crates/bitvue-av1/src/parser.rs` - parse_all_obus
- `crates/bitvue-av1/src/overlay_extraction/parser.rs` - ParsedFrame::parse

**Fix**: Create OBU iterator
```rust
pub struct ObuIterator<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for ObuIterator<'a> {
    type Item = Result<Obu<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // ... parse next OBU ...
    }
}

// Usage
for obu in ObuIterator::new(data) {
    match obu? {
        Obu::SequenceHeader(hdr) => { ... }
        Obu::Frame(frame) => { ... }
    }
}
```

**Estimated effort**: 3-4 hours
**Benefits**: Cleaner API, easier to use

---

## Large Functions

### 4. `decode_ivf` Function
**Issue**: 150+ lines, multiple responsibilities
**Location**: `crates/bitvue-decode/src/decoder.rs:338-455`

**Responsibilities**:
1. Parse IVF header
2. Iterate frames
3. Extract timestamps
4. Send to decoder
5. Collect results
6. Error handling

**Fix**: Split into focused functions
```rust
fn decode_ivf(&mut self, data: &[u8]) -> Result<Vec<DecodedFrame>> {
    let header = parse_ivf_header(data)?;
    let frames = extract_ivf_frames(data, header)?;
    self.decode_frames(frames)
}

fn parse_ivf_header(data: &[u8]) -> Result<IvfHeader> { ... }

fn extract_ivf_frames(data: &[u8], header: IvfHeader) -> Result<Vec<IvfFrame>> {
    let mut frames = Vec::new();
    let mut parser = IvfFrameParser::new(data, header);

    while let Some(frame) = parser.next_frame()? {
        frames.push(frame);
    }

    Ok(frames)
}

fn decode_frames(&mut self, frames: Vec<IvfFrame>) -> Result<Vec<DecodedFrame>> { ... }
```

**Estimated effort**: 3-4 hours
**Benefits**: Better testability, clearer logic

---

### 5. `parse_superblock` Function
**Issue**: 100+ lines with nested logic
**Location**: `crates/bitvue-av1/src/tile/mod.rs`

**Fix**: Extract sub-functions
```rust
fn parse_superblock(...) -> Result<Superblock> {
    let qp = parse_qp_delta(...)?;
    let partition = parse_partition(...)?;
    let coding_units = parse_coding_units_in_partition(...)?;

    Ok(Superblock {
        coding_units,
        qp,
    })
}
```

**Estimated effort**: 2-3 hours
**Benefits**: Easier to understand, test individual steps

---

## Complex Conditionals

### 6. Nested Frame Type Checks
**Issue**: Repeated if-else chains for frame types
**Locations**: Throughout codebase

**Current**:
```rust
if frame_type == FrameType::Key {
    // ...
} else if frame_type == FrameType::Inter {
    // ...
} else if frame_type == FrameType::IntraOnly {
    // ...
} else if frame_type == FrameType::Switch {
    // ...
}
```

**Fix**: Use match expressions
```rust
match frame_type {
    FrameType::Key | FrameType::IntraOnly => {
        // Intra frames
    }
    FrameType::Inter | FrameType::Switch => {
        // Inter frames
    }
}

// Or use enum methods
if frame_type.is_intra() {
    // ...
} else {
    // ...
}
```

**Estimated effort**: 2-3 hours
**Benefits**: Exhaustiveness checking, clearer intent

---

### 7. Pixel Format Handling
**Issue**: Complex conditionals for YUV formats
**Locations**: Color conversion, plane extraction

**Current**:
```rust
if format == Yuv420 && bit_depth == 8 {
    // ...
} else if format == Yuv420 && bit_depth == 10 {
    // ...
} else if format == Yuv422 && bit_depth == 8 {
    // ...
}
```

**Fix**: Use type-safe enum + match
```rust
enum PixelFormat {
    Yuv420p8,
    Yuv420p10,
    Yuv422p8,
    Yuv422p10,
    Yuv444p8,
    Yuv444p10,
}

match pixel_format {
    PixelFormat::Yuv420p8 => convert_420p8(data),
    PixelFormat::Yuv420p10 => convert_420p10(data),
    PixelFormat::Yuv422p8 => convert_422p8(data),
    // Compiler ensures all cases handled
}
```

**Estimated effort**: 4-5 hours
**Benefits**: Type safety, exhaustiveness checking

---

## Magic Numbers

### 8. Hard-Coded Constants
**Issue**: Magic numbers scattered throughout code

**Examples**:
```rust
if data.len() < 32 { ... }           // IVF header size
let header_size = 12;                // Frame header size
let max_size = 100 * 1024 * 1024;   // 100 MB
if timestamp_u64 > 9223372036854775807 { ... } // i64::MAX
```

**Fix**: Named constants
```rust
// crates/bitvue-decode/src/ivf.rs
pub mod ivf {
    pub const HEADER_SIZE: usize = 32;
    pub const FRAME_HEADER_SIZE: usize = 12;
    pub const MAX_FRAME_SIZE: usize = 100 * 1024 * 1024; // 100 MB
}

// Usage
if data.len() < ivf::HEADER_SIZE {
    return Err(DecodeError::InvalidIvfHeader);
}
```

**Estimated effort**: 2-3 hours
**Benefits**: Self-documenting, easier to maintain

---

## Inconsistent Error Handling

### 9. Mixed Error Types
**Issue**: Different error handling patterns
**Patterns**:
- `Result<T, DecodeError>`
- `Result<T, BitvueError>`
- `Result<T, String>`
- Panic on error

**Current**:
```rust
// Different patterns in same crate
fn parse_obu() -> Result<Obu, String> { ... }
fn decode() -> Result<Frame, DecodeError> { ... }
fn extract() -> Result<Grid, BitvueError> { ... }
```

**Fix**: Consistent error hierarchy
```rust
// Top-level error
#[derive(Debug, thiserror::Error)]
pub enum BitvueError {
    #[error("Decode error: {0}")]
    Decode(#[from] DecodeError),

    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// Domain-specific errors
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("Invalid frame size: {0}")]
    InvalidFrameSize(usize),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}
```

**Estimated effort**: 4-5 hours
**Benefits**: Consistent API, better error context

---

### 10. String-Based Errors
**Issue**: Errors using `String` lose type information
**Locations**: Many `format!()` calls in error returns

**Current**:
```rust
return Err(DecodeError::Decode(format!(
    "Frame size {} exceeds maximum {}",
    size, max_size
)));
```

**Fix**: Structured errors
```rust
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("Frame size {actual} exceeds maximum {maximum}")]
    FrameSizeExceeded {
        actual: usize,
        maximum: usize,
    },
}

// Usage
return Err(DecodeError::FrameSizeExceeded {
    actual: size,
    maximum: MAX_FRAME_SIZE,
});
```

**Estimated effort**: 5-6 hours
**Benefits**: Structured error data, better debugging

---

## Module Organization

### 11. Flat Module Structure
**Issue**: Too many items in root modules
**Location**: `crates/bitvue-av1/src/lib.rs` - 50+ pub items

**Current**:
```rust
// lib.rs
pub struct Obu { ... }
pub struct SequenceHeader { ... }
pub struct FrameHeader { ... }
pub fn parse_obu() { ... }
pub fn parse_sequence_header() { ... }
// ... 40+ more items
```

**Fix**: Hierarchical organization
```rust
// lib.rs
pub mod obu;
pub mod sequence;
pub mod frame;
pub mod tile;

// Re-export commonly used items
pub use obu::{Obu, ObuType};
pub use sequence::SequenceHeader;
pub use frame::FrameHeader;
```

**Estimated effort**: 3-4 hours
**Benefits**: Better discoverability, clearer API

---

## Type Safety

### 12. Primitive Obsession
**Issue**: Using primitives for domain concepts

**Examples**:
```rust
fn set_qp(qp: i16) { ... }           // What's valid range?
fn set_timestamp(ts: i64) { ... }    // What unit? Microseconds?
fn set_dimensions(w: u32, h: u32) { ... } // What constraints?
```

**Fix**: Newtype pattern
```rust
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Qp(i16);

impl Qp {
    pub const MIN: i16 = 0;
    pub const MAX: i16 = 255;

    pub fn new(value: i16) -> Result<Self> {
        if value < Self::MIN || value > Self::MAX {
            return Err(Error::InvalidQp(value));
        }
        Ok(Self(value))
    }

    pub fn value(self) -> i16 { self.0 }
}

// Similar for Timestamp, Dimensions, etc.
```

**Estimated effort**: 6-8 hours
**Benefits**: Compile-time validation, self-documenting

---

## Testing Improvements

### 13. Test Organization
**Issue**: Tests mixed with implementation
**Current**: Tests in same file as implementation

**Fix**: Separate test modules
```rust
// src/decoder.rs
pub struct Decoder { ... }

impl Decoder {
    pub fn decode() { ... }
}

// tests/decoder_test.rs
use bitvue_decode::Decoder;

#[test]
fn test_decode_valid_frame() { ... }

#[test]
fn test_decode_invalid_frame() { ... }
```

**Estimated effort**: 2-3 hours
**Benefits**: Cleaner source files, better test discovery

---

### 14. Property-Based Testing
**Issue**: Only example-based tests
**Opportunity**: Add property tests for parsers

**Example**:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_parse_doesnt_panic(data in prop::collection::vec(any::<u8>(), 0..1000)) {
        // Parser should never panic, only return Err
        let _ = parse_obu(&data);
    }

    #[test]
    fn test_roundtrip(obu in arbitrary_obu()) {
        // Serialize then parse should give same result
        let bytes = obu.to_bytes();
        let parsed = parse_obu(&bytes).unwrap();
        assert_eq!(obu, parsed);
    }
}
```

**Estimated effort**: 8-10 hours
**Benefits**: Find edge cases, better coverage

---

## Documentation

### 15. API Documentation
**Issue**: Missing or incomplete doc comments
**Coverage**: ~30% of public items documented

**Requirements**:
- All `pub` items should have doc comments
- Examples for complex APIs
- Error conditions documented
- Safety requirements for `unsafe` code

**Example**:
```rust
/// Decodes AV1 bitstream data into frames.
///
/// This decoder supports:
/// - Annex B format (OBU sequence)
/// - IVF container format
/// - All AV1 profiles (0, 1, 2)
///
/// # Examples
///
/// ```
/// use bitvue_decode::{Av1Decoder, VideoDecoder};
///
/// let mut decoder = Av1Decoder::new()?;
/// let frames = decoder.decode_all(&ivf_data)?;
/// assert_eq!(frames.len(), 30); // 30 fps @ 1 second
/// ```
///
/// # Errors
///
/// Returns `DecodeError` if:
/// - Input data is malformed
/// - Unsupported AV1 features are used
/// - Memory allocation fails
///
/// # Performance
///
/// Decoding is single-threaded. For parallel decoding of multiple
/// files, create separate decoder instances.
pub struct Av1Decoder { ... }
```

**Estimated effort**: 10-15 hours
**Benefits**: Better API usability, easier onboarding

---

## Priority Order

**High Priority** (improve quality, prevent bugs):
1. #1 - Duplicate parse_all_coding_units
2. #2 - Duplicate spatial index
3. #8 - Magic numbers to constants
4. #9 - Consistent error handling

**Medium Priority** (improve maintainability):
5. #4 - Split decode_ivf
6. #6 - Use match for frame types
7. #11 - Module organization
8. #15 - API documentation

**Low Priority** (nice to have):
9. #3 - OBU iterator
10. #7 - Pixel format enum
11. #12 - Newtype patterns
12. #13 - Test organization
13. #14 - Property-based testing

**Can be skipped**:
14. #5 - parse_superblock split (complex, low value)
15. #10 - Structured errors (nice but verbose)

---

## Refactoring Guidelines

1. **Make it work, make it right, make it fast**
   - Correctness first
   - Clean code second
   - Performance last

2. **Boy Scout Rule**
   - Leave code cleaner than you found it
   - Refactor opportunistically during feature work

3. **Test First**
   - Write test demonstrating issue
   - Refactor
   - Verify test still passes

4. **Small Steps**
   - One refactoring at a time
   - Commit frequently
   - Easy to revert if needed

5. **Measure Impact**
   - Run benchmarks before/after
   - Ensure no performance regression
   - Document improvements

---

## Tools

```bash
# Find duplicate code
cargo clippy -- -W clippy::nursery

# Find large functions
tokei --files crates/ | sort -k 5 -n

# Find complex functions
cargo bloat --release

# Code coverage
cargo tarpaulin --out Html
```
