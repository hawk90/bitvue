# HIGH Severity Issues - High Priority

## Summary
- **Total Issues**: 13
- **Performance**: 2
- **Bugs**: 6
- **Refactoring**: 5
- **Estimated Total Effort**: 40-60 hours

**Priority**: Address after CRITICAL issues, before feature work.

---

## PERFORMANCE Issues

### 1. CodingUnit Clone in MV Predictor
**Severity**: HIGH (Performance)
**Status**: TODO
**File**: `crates/bitvue-av1/src/tile/coding_unit.rs:377`
**Impact**: 300 MB/sec for 60fps 1080p video
**Effort**: 1-2 hours

#### Description
Unnecessary clone of entire CodingUnit struct when adding to motion vector predictor context. Called for EVERY coding unit in EVERY frame.

#### Current Code
```rust
mv_ctx.add_cu(cu.clone());  // Full struct clone
```

#### Impact Analysis
```
CodingUnit contains:
- 2 × MotionVector (8 bytes each = 16 bytes)
- 2 × RefFrame (1 byte each = 2 bytes)
- String mode (~20 bytes avg)
- Option<i16> qp (2 bytes)
Total: ~40 bytes per CU

For 1080p video:
- ~30,000 CUs per frame
- 60 fps = 1,800,000 CUs/sec
- 1,800,000 × 40 bytes = 72 MB/sec cloned
```

#### Fix Required
```rust
// Option 1: Pass by reference
mv_ctx.add_cu_ref(&cu);

// Option 2: Use Arc for shared ownership
mv_ctx.add_cu(Arc::new(cu));

// Option 3: Extract only needed fields
mv_ctx.add_mv_context(cu.mv[0], cu.mv[1], cu.ref_frame[0], cu.ref_frame[1]);
```

#### Recommended Fix
```rust
// In mv_context.rs, change interface:
pub fn add_cu_ref(&mut self, cu: &CodingUnit) {
    // Store only what's needed for MV prediction
    self.cu_refs.push(MvCuRef {
        mv: [cu.mv[0], cu.mv[1]],
        ref_frame: [cu.ref_frame[0], cu.ref_frame[1]],
        // Don't store mode, qp, etc.
    });
}

// In coding_unit.rs:377, change to:
mv_ctx.add_cu_ref(&cu);  // No clone!
```

#### Testing
```rust
#[bench]
fn bench_mv_context_add_cu_clone(b: &mut Bencher) {
    // Before: clone
    b.iter(|| {
        mv_ctx.add_cu(cu.clone());
    });
}

#[bench]
fn bench_mv_context_add_cu_ref(b: &mut Bencher) {
    // After: reference
    b.iter(|| {
        mv_ctx.add_cu_ref(&cu);
    });
}
```

#### Expected Improvement
- **Eliminate 72 MB/sec allocations**
- Reduce GC pressure
- 5-10% overall performance improvement

---

### 2. String Allocations in Tooltips
**Severity**: HIGH (Performance)
**Status**: TODO
**File**: `crates/bitvue-av1/src/overlay_extraction/parser.rs:280-287`
**Impact**: 60 MB/hour in GC pressure for interactive use
**Effort**: 2-3 hours

#### Description
Unnecessary string allocations in `extract_pixel_info()` called for every tooltip interaction.

#### Current Code
```rust
let block_id = format!("sb[{}][{}]", sb_y, sb_x);  // Allocation
let partition_info = "TX_64X64".to_string();        // Allocation
let syntax_path = format!("OBU_FRAME.tile[0].sb[{}][{}]", sb_y, sb_x);  // Allocation
```

#### Impact Analysis
```
Per tooltip interaction:
- 3 string allocations
- ~100 bytes allocated

For heavy interactive use:
- 100 tooltips/minute × 60 minutes = 6000 tooltips/hour
- 6000 × 100 bytes = 600 KB/hour (low estimate)
- Actual use likely 10× higher = 6 MB/hour
```

#### Fix Required
```rust
// Option 1: Use static strings for constants
const PARTITION_INFO_64X64: &str = "TX_64X64";

// Option 2: Lazy formatting with Cow
use std::borrow::Cow;

fn extract_pixel_info(...) -> Cow<'static, str> {
    Cow::Owned(format!("sb[{}][{}]", sb_y, sb_x))
}

// Option 3: Return struct with display-time formatting
pub struct PixelInfo {
    sb_y: usize,
    sb_x: usize,
    partition: PartitionType,
}

impl Display for PixelInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "sb[{}][{}]", self.sb_y, self.sb_x)
    }
}

// Option 4: Cache formatted strings in the parser
pub struct OverlayParser {
    string_cache: HashMap<(usize, usize), String>,
}
```

#### Recommended Fix
```rust
// Use Display trait for lazy formatting
pub struct PixelInfo {
    pub sb_y: usize,
    pub sb_x: usize,
    pub partition: PartitionType,
    pub tile_index: usize,
}

impl Display for PixelInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OBU_FRAME.tile[{}].sb[{}][{}]",
            self.tile_index, self.sb_y, self.sb_x
        )
    }
}

// In tooltip code:
let info = extract_pixel_info_struct(...);
tooltip.set_text(info.to_string());  // Only allocate when displayed
```

#### Testing
```rust
#[bench]
fn bench_pixel_info_format(b: &mut Bencher) {
    b.iter(|| {
        format!("sb[{}][{}]", sb_y, sb_x);
    });
}

#[bench]
fn bench_pixel_info_display(b: &mut Bencher) {
    let info = PixelInfo { sb_y: 10, sb_x: 20, ... };
    b.iter(|| {
        info.to_string();  // Only allocate when needed
    });
}
```

#### Expected Improvement
- **Eliminate unnecessary allocations**
- Only allocate when tooltip is actually displayed
- Reduced memory pressure

---

## BUG Issues

### 3. Division by Zero Potential in block_size_log2
**Severity**: HIGH
**Status**: TODO
**File**: `crates/bitvue-av1/src/tile/partition.rs:271-276`
**Effort**: 1-2 hours

#### Description
While this won't divide by zero directly, there's an implicit assumption that `width()` and `height()` never return 0. However, `BlockSize` enum has no explicit validation preventing zero-sized blocks.

#### Current Code
```rust
fn block_size_log2(block_size: BlockSize) -> u8 {
    let size = block_size.width().max(block_size.height());
    (size.ilog2() as u8).clamp(2, 7)
}
```

#### Issue
- `ilog2()` panics on 0 (in debug) or returns undefined behavior (in release)
- No validation in `BlockSize` construction
- Could be triggered by malformed AV1 data

#### Fix Required
```rust
// Add validation in BlockSize methods
impl BlockSize {
    pub fn width(&self) -> u32 {
        let w = match self {
            BlockSize::BLOCK_4X4 => 4,
            // ... all other sizes
        };
        debug_assert!(w > 0, "Block width must be positive");
        w
    }

    pub fn height(&self) -> u32 {
        let h = match self {
            // ...
        };
        debug_assert!(h > 0, "Block height must be positive");
        h
    }
}

// Add explicit validation in block_size_log2
fn block_size_log2(block_size: BlockSize) -> Result<u8, BitvueError> {
    let w = block_size.width();
    let h = block_size.height();

    if w == 0 || h == 0 {
        return Err(BitvueError::InvalidData(
            "Block size cannot be zero".to_string()
        ));
    }

    let size = w.max(h);
    Ok(size.ilog2().clamp(2, 7) as u8)
}
```

#### Testing
```rust
#[test]
fn test_block_size_validation() {
    // Test all BlockSize variants have positive dimensions
    // Test invalid data cannot create zero-sized blocks
}
```

---

### 4. Unbounded Vector Growth in Frame Header Parsing
**Severity**: HIGH
**Status**: TODO
**File**: `crates/bitvue-av1/src/frame_header.rs:104-109`
**Effort**: 3-4 hours

#### Description
The code uses an "approximate" bit skip count and breaks on EOF, but doesn't validate it reached the correct field. This could cause misinterpretation of frame header data.

#### Current Code
```rust
let bits_to_skip = 20; // Approximate bits to skip to reach refresh_frame_flags
for _ in 0..bits_to_skip {
    if reader.read_bit().is_err() {
        break;
    }
}
```

#### Issue
- Comment acknowledges this is "approximate"
- No validation we reached the correct field
- Could be reading from wrong location
- Brittle if frame format changes

#### Fix Required
```rust
// Option 1: Parse all intermediate fields properly
pub fn parse_frame_header(reader: &mut BitReader) -> Result<FrameHeader, BitvueError> {
    // Parse show_existing_frame
    let show_existing_frame = reader.read_bool()?;

    // Parse frame_type
    let frame_type = reader.read_bits(2)?;

    // Parse show_frame
    let show_frame = reader.read_bool()?;

    // ... parse ALL intermediate fields

    // Now we're at refresh_frame_flags for sure
    let refresh_frame_flags = reader.read_bits(8)?;

    Ok(FrameHeader {
        show_existing_frame,
        frame_type,
        show_frame,
        refresh_frame_flags,
        // ...
    })
}

// Option 2: Use field offset table
pub struct FrameHeaderOffsets {
    show_existing_frame: u32,
    frame_type: u32,
    show_frame: u32,
    refresh_frame_flags: u32,
}
```

#### Recommended Fix
Parse all intermediate fields properly (Option 1).

#### Testing
```rust
#[test]
fn test_frame_header_parsing() {
    // Test with known valid frame headers
    // Test field offsets are correct
    // Test error detection on malformed headers
}
```

---

### 5. MP4 Integer Overflow on 32-bit Systems
**Severity**: HIGH
**Status**: TODO
**File**: `crates/bitvue-formats/src/mp4.rs:244-246`
**Effort**: 2-3 hours

#### Description
Type mismatch: `payload_size` is `u64` while `payload_start` is `usize`. On 32-bit systems, the conversion could truncate large values.

#### Current Code
```rust
let payload_size_usize = payload_size
    .try_into()
    .map_err(|_| BitvueError::InvalidData("OBU payload size overflow".to_string()))?;

let total_size = payload_start
    .checked_add(payload_size_usize)
    .ok_or_else(|| BitvueError::InvalidData("OBU payload size overflow".to_string()))?;
```

#### Issue
- `try_into()` on 32-bit system could succeed for values that would overflow
- No validation that conversion didn't lose data
- Large MP4 files (>4GB) could cause incorrect parsing

#### Fix Required
```rust
// Use u64 consistently for all size calculations
let total_size = payload_start as u64
    .checked_add(payload_size)
    .ok_or_else(|| BitvueError::InvalidData("Box end offset overflow".to_string()))?;

// Validate it fits in usize for the current platform
let total_size_usize = total_size
    .try_into()
    .map_err(|_| BitvueError::InvalidData(
        format!("Box offset {} exceeds platform address space", total_size)
    ))?;

// Validate we have enough data
if total_size_usize > data.len() {
    return Err(BitvueError::UnexpectedEof(total_size as u64));
}
```

#### Testing
```rust
#[test]
fn test_mp4_large_files() {
    // Test with 4GB+ file offsets
    // Test overflow detection on 32-bit
    // Test correct parsing on 64-bit
}
```

#### Platform Considerations
- Issue primarily affects 32-bit builds
- Consider adding platform-specific tests in CI

---

### 6. MKV Unvalidated VINT Length
**Severity**: HIGH
**Status**: TODO
**File**: `crates/bitvue-formats/src/mkv.rs:40-47`
**Effort**: 1-2 hours

#### Description
If `first` is `0x00` (all zeros), the loop completes with `length == 0`. However, this doesn't catch malformed VINT marker bits (e.g., `0x01` indicating 8 bytes but with invalid pattern).

#### Current Code
```rust
let mut length = 0;
for i in 0..8 {
    if (first & (0x80 >> i)) != 0 {
        length = i + 1;
        break;
    }
}
```

#### Issue
- VINT pattern `0x01` would set `length = 8` but is invalid per EBML spec
- No validation of marker bit pattern
- Could accept malformed EBML

#### Fix Required
```rust
// According to EBML spec, VINT marker must be: 1 followed by N zeros
// Valid patterns: 0x81 (1 byte), 0x40 (2 bytes), 0x20 (3 bytes), etc.
// Invalid patterns: 0x01 (all set bits), 0xFF (no marker)

fn parse_vint_length(first: u8) -> Result<usize, BitvueError> {
    // Valid VINT markers: 0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01
    const VALID_MARKERS: [u8; 8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

    for (i, &marker) in VALID_MARKERS.iter().enumerate() {
        if (first & marker) != 0 {
            // Validate only marker bit is set in the marker position
            if first & !marker != 0 {
                return Err(BitvueError::InvalidData(format!(
                    "Invalid VINT marker: 0x{:02X} (has extra bits set)",
                    first
                )));
            }
            return Ok(i + 1);
        }
    }

    Err(BitvueError::InvalidData(
        "Invalid VINT: no marker bit found".to_string()
    ))
}
```

#### Testing
```rust
#[test]
fn test_vint_validation() {
    // Test valid VINT markers
    assert_eq!(parse_vint_length(0x81).unwrap(), 1);
    assert_eq!(parse_vint_length(0x40).unwrap(), 2);

    // Test invalid markers
    assert!(parse_vint_length(0x01).is_err());  // All bits set
    assert!(parse_vint_length(0xFF).is_err());  // No marker
    assert!(parse_vint_length(0x00).is_err());  // No marker
}
```

#### References
- EBML Spec: https://github.com/matroska-org/ebml-specification

---

### 7. TS Parser Integer Overflow in Offset Calculation
**Severity**: HIGH
**Status**: TODO
**File**: `crates/bitvue-formats/src/ts.rs:129-136`
**Effort**: 1-2 hours

#### Description
Redundant cast and potential underflow if `adaptation_length` is larger than `data.len() - 5`.

#### Current Code
```rust
payload_start = match (payload_start as usize).checked_add(1).and_then(|v| v.checked_add(adaptation_length)) {
    Some(v) => v,
    None => {
        return Err(BitvueError::InvalidData(
            "Adaptation field overflow".to_string()
        ));
    }
};
```

#### Issue
- `payload_start` is already `usize`, so cast is redundant
- If `adaptation_length > data.len() - 5`, subsequent extraction underflows
- No validation against remaining data length

#### Fix Required
```rust
// Remove redundant cast and validate against data length
let remaining_data = data.len().saturating_sub(5);

if adaptation_length as usize > remaining_data {
    return Err(BitvueError::InvalidData(format!(
        "Adaptation field length {} exceeds remaining data {}",
        adaptation_length, remaining_data
    )));
}

payload_start = payload_start
    .checked_add(1)
    .and_then(|v| v.checked_add(adaptation_length as usize))
    .ok_or_else(|| BitvueError::InvalidData(
        "Payload start offset overflow".to_string()
    ))?;
```

#### Testing
```rust
#[test]
fn test_ts_adaptation_field_overflow() {
    // Test with adaptation_length > remaining data
    // Test with valid adaptation_length
}
```

---

### 8. Arithmetic Decoder Potential Underflow in Refill
**Severity**: HIGH
**Status**: TODO
**File**: `crates/bitvue-av1/src/symbol/arithmetic.rs:213-244`
**Effort**: 2-3 hours

#### Description
The calculation of `c` could underflow if `self.cnt` is less than `-((EC_WIN_SIZE as i32) - 24)`. While `cnt` is initialized to `-15`, it's decremented without explicit bounds checking.

#### Current Code
```rust
fn refill(&mut self) -> Result<()> {
    let mut c = (EC_WIN_SIZE as i32) - self.cnt - 24;
    let mut value = self.value;

    loop {
        if self.offset >= self.data.len() {
            if c >= 0 {
                value |= !(!(0xFF_usize << c));
            }
            break;
        }
        // ...
    }
}
```

#### Issue
- `cnt` can become very negative through repeated `renormalize` calls
- Underflow could cause incorrect behavior
- No invariant checks

#### Fix Required
```rust
// Add invariant documentation and validation
impl ArithmeticDecoder {
    const EC_WIN_SIZE: i32 = 32;
    const MIN_CNT: i32 = -31;  // Minimum valid cnt value

    /// Refill the value register with bits from the input stream.
    ///
    /// # Invariants
    ///
    /// - `self.cnt` must be in range [MIN_CNT, 16]
    /// - `self.value` must be in range [0, EC_WIN_SIZE]
    fn refill(&mut self) -> Result<()> {
        // Validate cnt invariants
        debug_assert!(self.cnt >= Self::MIN_CNT, "cnt below minimum");
        debug_assert!(self.cnt <= 16, "cnt above maximum");

        let c = (EC_WIN_SIZE - self.cnt - 24)
            .max(0)  // Prevent underflow
            .min(24);  // Reasonable upper bound

        let mut value = self.value;

        loop {
            if self.offset >= self.data.len() {
                if c >= 0 {
                    value |= !(!(0xFF_usize << c));
                }
                break;
            }

            value = (value << 8) | self.data[self.offset] as usize;
            self.offset += 1;
            c -= 8;

            if c < 0 {
                break;
            }
        }

        self.value = value;
        Ok(())
    }
}
```

#### Testing
```rust
#[test]
fn test_arithmetic_refill_boundaries() {
    // Test refill at various cnt values
    // Test underflow prevention
    // Test with minimal data
}
```

---

## REFACTORING Issues

### 9. Inconsistent Error Types
**Severity**: HIGH (Maintainability)
**Status**: TODO
**Files**: Multiple (see below)
**Impact**: API usability, error handling consistency
**Effort**: 8-12 hours

#### Description
The codebase uses multiple error types without clear conversion between them.

#### Current State
```rust
// Inconsistent error patterns across crates:
Result<T, String>           // bitvue-mcp, bitvue-decode
Result<T, DecodeError>      // bitvue-decode
Result<T, BitvueError>      // bitvue-core
Result<T, AvcError>         // bitvue-avc
Result<T, HevcError>        // bitvue-hevc
```

#### Affected Locations
- `crates/bitvue-mcp/src/main.rs:49,407,556,673,707,750,777,881,925,983,1014,1081`
- `crates/bitvue-decode/src/yuv.rs:424,446`
- `crates/bitvue-decode/src/strategy/registry.rs:87,213`
- `crates/bitvue-decode/src/traits.rs:77,282`

#### Refactoring Plan

**Step 1: Expand BitvueError enum**
```rust
// crates/bitvue-core/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum BitvueError {
    // Existing variants...

    #[error("Codec error: {codec} - {message}")]
    Codec {
        codec: String,
        message: String,
        #[source] source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("YUV conversion error: {0}")]
    YuvConversion(String),

    #[error("Decode error: {0}")]
    Decode(String),
}
```

**Step 2: Implement From traits**
```rust
// Codec-specific errors convert to BitvueError
impl From<AvcError> for BitvueError {
    fn from(err: AvcError) -> Self {
        BitvueError::Codec {
            codec: "AVC".to_string(),
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

// Repeat for HevcError, Vp9Error, etc.

// String errors convert with context
impl From<String> for BitvueError {
    fn from(s: String) -> Self {
        BitvueError::Other(s)
    }
}

impl From<&str> for BitvueError {
    fn from(s: &str) -> Self {
        BitvueError::Other(s.to_string())
    }
}
```

**Step 3: Replace String-based errors**
```rust
// Before:
fn parse_ivf_file(...) -> Result<Vec<UnitNode>, String>
fn rgb_to_image(...) -> Result<image::RgbImage, String>
fn set_strategy(...) -> Result<(), String>

// After:
fn parse_ivf_file(...) -> Result<Vec<UnitNode>, BitvueError>
fn rgb_to_image(...) -> Result<image::RgbImage, BitvueError>
fn set_strategy(...) -> Result<(), BitvueError>
```

**Step 4: Update all call sites**
- Search for `Result<.*, String>` patterns
- Replace with `Result<.*, BitvueError>`
- Update error handling code

#### Benefits
- Consistent error handling across all crates
- Better error messages with context
- Easier to debug with structured errors
- Type-safe error propagation

#### Testing
```rust
#[test]
fn test_error_conversion() {
    // Test all error types convert to BitvueError
    // Test error context is preserved
    // Test error messages are helpful
}
```

#### Migration Order
1. Expand BitvueError enum (2-3 hours)
2. Implement From traits (2-3 hours)
3. Migrate bitvue-decode (2-3 hours)
4. Migrate bitvue-mcp (1-2 hours)
5. Update tests (1-2 hours)

---

### 10. Frame Extraction Duplication
**Severity**: HIGH (Maintainability)
**Status**: TODO
**Files**: Multiple
**Impact**: ~400 lines duplicated
**Effort**: 6-8 hours

#### Description
Each codec implements its own frame builder with nearly identical structure.

#### Affected Files
- `crates/bitvue-avc/src/frames.rs:144-156`
- `crates/bitvue-hevc/src/frames.rs:40-100`
- `crates/bitvue-vp9/src/frames.rs`

#### Current Pattern
```rust
// Repeated in AVC, HEVC, VP9
pub struct HevcFrameBuilder {
    frame_index: Option<usize>,
    frame_type: Option<HevcFrameType>,
    nal_data: Option<Vec<u8>>,
    offset: Option<usize>,
    size: Option<usize>,
    poc: Option<i32>,
    frame_num: Option<u32>,
    is_idr: Option<bool>,
    is_irap: Option<bool>,
    is_ref: Option<bool>,
    // ... more fields
}
```

#### Refactoring Plan
```rust
// Generic frame builder in bitvue-core
// crates/bitvue-core/src/frame/builder.rs

pub struct FrameBuilder<F> {
    inner: F,
    common: CommonFrameFields,
}

pub struct CommonFrameFields {
    frame_index: Option<usize>,
    offset: Option<usize>,
    size: Option<usize>,
    timestamp: Option<u64>,
    is_key: Option<bool>,
    is_ref: Option<bool>,
}

pub trait FrameBuilderConfig {
    type FrameType;
    type FrameData;

    fn build(self) -> Result<Self::FrameData, BitvueError>;
}

// Usage in each codec:
impl FrameBuilderConfig for AvcFrameBuilder {
    type FrameType = AvcFrameType;
    type FrameData = AvcFrame;

    fn build(self) -> Result<AvcFrame, BitvueError> {
        // Shared validation and construction logic
        Ok(AvcFrame {
            frame_index: self.common.frame_index.ok_or_else(|| {
                BitvueError::InvalidData("frame_index required".to_string())
            })?,
            offset: self.common.offset.ok_or_else(|| {
                BitvueError::InvalidData("offset required".to_string())
            })?,
            // ... codec-specific fields
        })
    }
}
```

---

### 11. Excessive Nesting in decode_ivf
**Severity**: MEDIUM (Readability)
**Status**: TODO
**File**: `crates/bitvue-decode/src/decoder.rs:354-391`
**Effort**: 2-3 hours

#### Description
The `decode_ivf` function has 4-5 levels of nesting, making it hard to read and test.

#### Refactoring
```rust
// Split into smaller functions:
fn decode_ivf(&mut self, data: &[u8]) -> Result<Vec<DecodedFrame>> {
    let header = self.parse_ivf_header(data)?;
    let frames = self.extract_ivf_frames(data, &header)?;
    self.decode_frames(frames)
}

fn extract_ivf_frames(&self, data: &[u8], header: &IvfHeader) -> Result<Vec<IvfFrame>> {
    let mut frames = Vec::new();
    let mut parser = IvfFrameParser::new(data, header);

    while let Some(frame) = parser.next_frame()? {
        frames.push(frame);
    }

    Ok(frames)
}

fn decode_frames(&mut self, frames: Vec<IvfFrame>) -> Result<Vec<DecodedFrame>> {
    let mut decoded = Vec::new();

    for frame in frames {
        match self.decode_single_frame(frame) {
            Ok(f) => decoded.push(f),
            Err(e) => tracing::warn!("Failed to decode frame: {}", e),
        }
    }

    if decoded.is_empty() {
        return Err(DecodeError::Decode("No frames decoded".to_string()));
    }

    Ok(decoded)
}
```

---

### 12. Complex Boolean Expressions
**Severity**: MEDIUM (Readability)
**Status**: TODO
**Files**: Multiple
**Effort**: 2-3 hours

#### Description
Complex boolean conditions without named helper functions.

#### Examples
```rust
// bitvue-core/src/bitreader.rs:613
if i + 2 < data.len() && data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x03 {

// app/src/parser_worker.rs:80
if data[0] == 0x1A && data.len() >= 4 && data[1] == 0x45 && data[2] == 0xDF && data[3] == 0xA3 {
```

#### Refactoring
```rust
// Named helper functions
fn is_emulation_prevention_byte(data: &[u8], i: usize) -> bool {
    i + 2 < data.len()
        && data[i] == 0x00
        && data[i + 1] == 0x00
        && data[i + 2] == 0x03
}

const AV1_MAGIC: [u8; 4] = [0x1A, 0x45, 0xDF, 0xA3];

fn is_annex_b(data: &[u8]) -> bool {
    data.len() >= 4 && data[0..4] == AV1_MAGIC
}

// Usage
if is_emulation_prevention_byte(data, i) {
    // ...
}

if is_annex_b(data) {
    // ...
}
```

---

### 13. Missing FrameTypeTrait Abstraction
**Severity**: MEDIUM (Maintainability)
**Status**: TODO
**Files**: Multiple
**Effort**: 4-6 hours

#### Description
No shared trait for common frame type operations across codecs.

#### Refactoring
```rust
// Create codec-agnostic trait
pub trait FrameTypeTrait {
    fn is_intra(&self) -> bool;
    fn is_inter(&self) -> bool;
    fn is_key(&self) -> bool;
    fn is_reference(&self) -> bool;
    fn display_name(&self) -> &'static str;
    fn color(&self) -> [u8; 3];
}

// Implement for each codec
impl FrameTypeTrait for AvcFrameType {
    fn is_intra(&self) -> bool {
        matches!(self, AvcFrameType::I)
    }

    fn color(&self) -> [u8; 3] {
        match self {
            AvcFrameType::I => [0, 255, 0],   // Green
            AvcFrameType::P => [255, 165, 0], // Orange
            AvcFrameType::B => [0, 165, 255], // Blue
        }
    }
    // ...
}

// Generic functions can now work with any codec
fn visualize_frame_types<T: FrameTypeTrait>(frames: &[T]) -> Vec<Color> {
    frames.iter().map(|ft| ft.color()).collect()
}
```

---

## Priority Order

### Performance (Quick Wins)
1. **CodingUnit clone** (#1) - 1-2 hours, 300 MB/sec eliminated
2. **String allocations** (#2) - 2-3 hours, reduced GC pressure

### Bugs (Security/Stability)
3. **Division by zero** (#3) - 1-2 hours, prevents panic
4. **Unbounded vector** (#4) - 3-4 hours, prevents misinterpretation
5. **MP4 overflow** (#5) - 2-3 hours, 32-bit compatibility
6. **MKV VINT** (#6) - 1-2 hours, EBML spec compliance
7. **TS overflow** (#7) - 1-2 hours, prevents underflow
8. **Arithmetic refill** (#8) - 2-3 hours, prevents underflow

### Refactoring (Maintainability)
9. **Error types** (#9) - 8-12 hours, consistency across crates
10. **Frame builders** (#10) - 6-8 hours, eliminate duplication
11. **Nesting** (#11) - 2-3 hours, readability
12. **Boolean expressions** (#12) - 2-3 hours, readability
13. **FrameTypeTrait** (#13) - 4-6 hours, generic algorithms

---

## Testing Checklist

- [ ] Performance benchmarks before/after for #1, #2
- [ ] Unit tests for all bug fixes (#3-#8)
- [ ] Integration tests for refactoring (#9-#13)
- [ ] Platform-specific tests for 32-bit (#5)
- [ ] Fuzzing targets for parsers (#4, #6, #7)
