//! ISO Base Media File Format (MP4) parser
//!
//! Implements minimal MP4 parsing to extract AV1 video samples.
//! No external dependencies - pure Rust implementation.
//!
//! References:
//! - ISO/IEC 14496-12 (ISO Base Media File Format)
//! - AV1 Codec ISO Media File Format Binding

use crate::resource_budget::ResourceBudget;
use bitvue_engine::BitvueError;
use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom};

// ============================================================================
// Constants
// ============================================================================

/// Maximum entry count to prevent DoS via massive allocations
const MAX_ENTRY_COUNT: u32 = 10_000_000;

/// Maximum total samples to prevent memory exhaustion
const MAX_TOTAL_SAMPLES: usize = 100_000;

/// Maximum nesting depth for MP4 boxes to prevent stack overflow
const MAX_BOX_DEPTH: u8 = 16;

/// Maximum sample size to prevent memory exhaustion (100MB)
const MAX_SAMPLE_SIZE: usize = 100 * 1024 * 1024;

/// Read a single byte
fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, BitvueError> {
    let mut buf = [0u8; 1];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;
    Ok(buf[0])
}

/// Read a 32-bit big-endian integer
fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, BitvueError> {
    let mut buf = [0u8; 4];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;
    Ok(u32::from_be_bytes(buf))
}

/// Read a 64-bit big-endian integer
fn read_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64, BitvueError> {
    let mut buf = [0u8; 8];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;
    Ok(u64::from_be_bytes(buf))
}

/// Read a 4-character box type
fn read_box_type(cursor: &mut Cursor<&[u8]>) -> Result<[u8; 4], BitvueError> {
    let mut buf = [0u8; 4];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| BitvueError::UnexpectedEof(cursor.position()))?;
    Ok(buf)
}

/// ISO BMFF Box header
#[derive(Debug, Clone)]
pub struct BoxHeader {
    /// Box type (4-character code)
    pub box_type: [u8; 4],
    /// Box size including header
    pub size: u64,
    /// Offset where box data starts (after header)
    pub data_offset: u64,
    /// Length of the header as written: 8, or 16 when the 64-bit `largesize` form was used
    pub header_size: u64,
}

impl BoxHeader {
    /// Parse a box header
    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Self, BitvueError> {
        let start_pos = cursor.position();

        let size32 = read_u32(cursor)?;
        let box_type = read_box_type(cursor)?;

        let size = if size32 == 1 {
            // 64-bit size
            read_u64(cursor)?
        } else if size32 == 0 {
            // Box extends to end of file
            let file_size = cursor.get_ref().len() as u64;
            file_size - start_pos
        } else {
            size32 as u64
        };

        let data_offset = cursor.position();
        let header_size = data_offset - start_pos;

        // Validate box size to prevent infinite loops
        if size < header_size {
            return Err(BitvueError::Parse {
                offset: start_pos,
                message: format!("Invalid box size: {} < header size {}", size, header_size),
            });
        }

        Ok(BoxHeader {
            box_type,
            size,
            data_offset,
            header_size,
        })
    }

    /// Get box type as string
    pub fn box_type_str(&self) -> String {
        // PERFORMANCE: Use cached string for common ASCII box types
        // This avoids allocation for the majority of cases
        if self.box_type.iter().all(|&b| b.is_ascii()) {
            // Safe to unwrap since we verified all bytes are ASCII
            unsafe { std::str::from_utf8_unchecked(&self.box_type) }.to_string()
        } else {
            // Fallback for non-ASCII box types (rare)
            String::from_utf8_lossy(&self.box_type).to_string()
        }
    }

    /// Get data size (excluding header)
    pub fn data_size(&self) -> u64 {
        self.size.saturating_sub(self.header_size)
    }
}

/// MP4 container information
#[derive(Debug, Clone, Default)]
pub struct Mp4Info {
    /// File brand (from ftyp box)
    pub brand: Option<String>,
    /// Compatible brands
    pub compatible_brands: Vec<String>,
    /// Video codec (e.g. "av01", "avc1", "hev1")
    pub codec: Option<String>,
    /// Timescale (units per second)
    pub timescale: u32,
    /// Total number of samples
    pub sample_count: usize,
    /// Handler type of the track these samples come from (`hdlr`, e.g. "vide", "soun")
    pub handler_type: Option<String>,
    /// Chunk offsets (`stco`/`co64`)
    pub chunk_offsets: Vec<u64>,
    /// Sample-to-chunk runs (`stsc`): `(first_chunk, samples_per_chunk)`, `first_chunk` 1-based
    pub sample_to_chunk: Vec<(u32, u32)>,
    /// File offset of each sample, in sample (decode) order; derived from the chunk offsets,
    /// `stsc` and the sample sizes once the sample table is complete
    pub sample_offsets: Vec<u64>,
    /// Sample sizes
    pub sample_sizes: Vec<u32>,
    /// Sample durations (in timescale units)
    pub sample_durations: Vec<u32>,
    /// Decode timestamps (DTS) in timescale units
    pub timestamps: Vec<u64>,
    /// Composition time offsets (for PTS calculation)
    pub composition_offsets: Vec<i32>,
    /// Presentation timestamps (PTS) in timescale units
    pub presentation_timestamps: Vec<u64>,
    /// Key frame indices (sync samples)
    pub key_frames: Vec<u32>,
}

/// Codec validator for sample extraction
type CodecValidator = fn(&str) -> bool;

/// Parse MP4 file and extract samples with codec validation
///
/// Generic sample extraction that validates the codec and returns zero-copy
/// Cow slices that borrow from the input data when possible.
///
/// # Arguments
///
/// * `data` - MP4 file data
/// * `codec_name` - Human-readable codec name for error messages
/// * `validator` - Function that returns true if codec string is valid
fn extract_samples_with_validator<'a>(
    data: &'a [u8],
    codec_name: &str,
    validator: CodecValidator,
) -> Result<Vec<Cow<'a, [u8]>>, BitvueError> {
    let info = parse_mp4(data)?;

    // Verify codec using the provided validator
    match &info.codec {
        Some(codec) if validator(codec) => {
            // Codec confirmed
        }
        Some(codec) => {
            return Err(BitvueError::InvalidData(format!(
                "Not an {} file: found codec '{}'",
                codec_name, codec
            )));
        }
        None => {
            return Err(BitvueError::InvalidData(
                "No codec information found in MP4".to_string(),
            ));
        }
    }

    // Validate sample count to prevent DoS
    if info.sample_sizes.len() > MAX_TOTAL_SAMPLES {
        return Err(BitvueError::InvalidData(format!(
            "Sample count {} exceeds maximum allowed {}",
            info.sample_sizes.len(),
            MAX_TOTAL_SAMPLES
        )));
    }

    if info.sample_offsets.is_empty() {
        return Err(BitvueError::InvalidData(
            "No chunk offsets found in MP4".to_string(),
        ));
    }
    if info.sample_offsets.len() != info.sample_sizes.len() {
        return Err(BitvueError::InvalidData(format!(
            "{} sample offsets for {} sample sizes",
            info.sample_offsets.len(),
            info.sample_sizes.len()
        )));
    }

    // Validate every sample, remembering its byte range; returned in sample (decode) order.
    let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(info.sample_sizes.len());
    for (&offset, &size) in info.sample_offsets.iter().zip(info.sample_sizes.iter()) {
        // Validate u64 to usize conversion to prevent truncation on 32-bit
        let offset = usize::try_from(offset).map_err(|_| {
            BitvueError::InvalidData(format!(
                "Sample offset {} exceeds platform address space",
                offset
            ))
        })?;
        let size = usize::try_from(size).map_err(|_| {
            BitvueError::InvalidData(format!(
                "Sample size {} exceeds platform address space",
                size
            ))
        })?;

        // SECURITY: Validate sample size to prevent memory exhaustion
        if size > MAX_SAMPLE_SIZE {
            return Err(BitvueError::InvalidData(format!(
                "Sample size {} exceeds maximum allowed {}",
                size, MAX_SAMPLE_SIZE
            )));
        }

        // Check for overflow in offset + size
        let end = offset.checked_add(size).ok_or_else(|| {
            BitvueError::InvalidData("Sample offset + size would overflow".to_string())
        })?;

        // Check against file size
        if end > data.len() {
            return Err(BitvueError::InvalidData(format!(
                "Sample at offset {} with size {} exceeds file size {}",
                offset,
                size,
                data.len()
            )));
        }
        ranges.push((offset, end));
    }

    // Overlap check: neighbours in file order, whatever the sample order is.
    let mut by_offset: Vec<(usize, usize)> = ranges.clone();
    by_offset.sort_unstable();
    for pair in by_offset.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(BitvueError::InvalidData(format!(
                "Samples overlap: current sample ends at {} but next starts at {}",
                pair[0].1, pair[1].0
            )));
        }
    }

    // Zero-copy: borrowed slices of the input
    Ok(ranges
        .into_iter()
        .map(|(start, end)| Cow::Borrowed(&data[start..end]))
        .collect())
}

/// Parse MP4 file and extract AV1 samples
///
/// Returns zero-copy Cow slices that borrow from the input data when possible,
/// avoiding unnecessary memory allocation.
pub fn extract_av1_samples(data: &[u8]) -> Result<Vec<Cow<'_, [u8]>>, BitvueError> {
    extract_samples_with_validator(data, "AV1", |codec| codec == "av01")
}

/// Parse MP4 file and extract H.264/AVC samples
///
/// Extracts NAL units from MP4 container for H.264/AVC video streams.
/// Supports both 'avc1' (AVC in MP4) and 'avc3' (AVC without parameter sets) codec types.
///
/// Returns zero-copy Cow slices that borrow from the input data when possible,
/// avoiding unnecessary memory allocation.
pub fn extract_avc_samples(data: &[u8]) -> Result<Vec<Cow<'_, [u8]>>, BitvueError> {
    extract_samples_with_validator(data, "H.264/AVC", |codec| {
        codec == "avc1" || codec == "avc3"
    })
}

/// Parse MP4 file and extract H.265/HEVC samples
///
/// Extracts NAL units from MP4 container for H.265/HEVC video streams.
/// Supports both 'hev1' (HEVC with parameter sets) and 'hvc1' (HEVC in-band parameter sets) codec types.
///
/// Returns zero-copy Cow slices that borrow from the input data when possible,
/// avoiding unnecessary memory allocation.
pub fn extract_hevc_samples(data: &[u8]) -> Result<Vec<Cow<'_, [u8]>>, BitvueError> {
    extract_samples_with_validator(data, "H.265/HEVC", |codec| {
        codec == "hev1" || codec == "hvc1"
    })
}

/// Parse MP4 file structure
pub fn parse_mp4(data: &[u8]) -> Result<Mp4Info, BitvueError> {
    if data.is_empty() {
        return Err(BitvueError::InvalidData("Empty MP4 data".to_string()));
    }

    // Check data size against resource budget
    let budget = ResourceBudget::new();
    if let Err(e) = budget.check_allocation(data.len() as u64) {
        return Err(BitvueError::InvalidData(format!(
            "MP4 file too large: {}",
            e
        )));
    }

    let mut cursor = Cursor::new(data);
    let mut info = Mp4Info::default();

    // Parse top-level boxes
    while cursor.position() < data.len() as u64 {
        let header = BoxHeader::parse(&mut cursor)?;
        let box_start = header.data_offset;

        // Calculate box end with overflow protection
        let box_end = box_start
            .checked_add(header.data_size())
            .ok_or_else(|| BitvueError::InvalidData("MP4 box end offset overflow".to_string()))?;

        // Validate box_end doesn't exceed file size
        if box_end > data.len() as u64 {
            return Err(BitvueError::InvalidData(format!(
                "MP4 box extends beyond file: box_end {} > file size {}",
                box_end,
                data.len()
            )));
        }

        // Validate box_end fits in platform address space (for 32-bit systems)
        let box_end_usize = box_end.try_into().map_err(|_| {
            BitvueError::InvalidData(format!(
                "MP4 box offset {} exceeds platform address space",
                box_end
            ))
        })?;

        match &header.box_type {
            b"ftyp" => {
                // File type box
                parse_ftyp(&mut cursor, &header, &mut info)?;
            }
            b"moov" => {
                // Movie box - contains metadata (start at depth 0)
                parse_moov(&mut cursor, &header, &mut info, data, 0)?;
            }
            b"mdat" => {
                // Media data box - skip for now
            }
            _ => {
                // Skip unknown box
            }
        }

        // Move to next box
        cursor.seek(SeekFrom::Start(box_end_usize))?;
    }

    Ok(info)
}

/// Parse ftyp (File Type) box
fn parse_ftyp(
    cursor: &mut Cursor<&[u8]>,
    header: &BoxHeader,
    info: &mut Mp4Info,
) -> Result<(), BitvueError> {
    // major_brand (4) + minor_version (4) are mandatory
    if header.data_size() < 8 {
        return Err(BitvueError::InvalidData(format!(
            "ftyp box has {} data bytes, needs at least 8",
            header.data_size()
        )));
    }
    let brand = read_box_type(cursor)?;
    info.brand = Some(String::from_utf8_lossy(&brand).to_string());

    let _minor_version = read_u32(cursor)?;

    // Compatible brands
    let remaining = header.data_size() - 8; // >= 0, checked above
    for _ in 0..(remaining / 4) {
        let compat = read_box_type(cursor)?;
        info.compatible_brands
            .push(String::from_utf8_lossy(&compat).to_string());
    }

    Ok(())
}

/// Parse moov (Movie) box
fn parse_moov(
    cursor: &mut Cursor<&[u8]>,
    header: &BoxHeader,
    info: &mut Mp4Info,
    data: &[u8],
    depth: u8,
) -> Result<(), BitvueError> {
    // SECURITY: Check box nesting depth to prevent stack overflow
    if depth >= MAX_BOX_DEPTH {
        return Err(BitvueError::InvalidData(format!(
            "MP4 box depth {} exceeds maximum {}",
            depth, MAX_BOX_DEPTH
        )));
    }
    let child_depth = depth + 1;
    let box_end = header
        .data_offset
        .checked_add(header.data_size())
        .ok_or_else(|| BitvueError::InvalidData("MP4 child box end offset overflow".to_string()))?;

    let mut tracks: Vec<Mp4Info> = Vec::new();

    // Parse child boxes
    while cursor.position() < box_end {
        let child_header = BoxHeader::parse(cursor)?;
        let child_end = child_header
            .data_offset
            .checked_add(child_header.data_size())
            .ok_or_else(|| {
                BitvueError::InvalidData("MP4 child box end offset overflow".to_string())
            })?;

        match &child_header.box_type {
            b"trak" => {
                // Track box: parsed on its own so that one track's tables never mix with another's
                let mut track = Mp4Info::default();
                parse_trak(cursor, &child_header, &mut track, data, child_depth)?;
                tracks.push(track);
            }
            _ => {
                // Skip other boxes
            }
        }

        cursor.seek(SeekFrom::Start(child_end))?;
    }

    // Keep one track: the first video track; failing that a track without a handler box (minimal
    // files), then the first track of any kind so that the codec check can name what was found.
    let chosen = tracks
        .iter()
        .position(|t| t.handler_type.as_deref() == Some("vide"))
        .or_else(|| tracks.iter().position(|t| t.handler_type.is_none()))
        .or(if tracks.is_empty() { None } else { Some(0) });
    if let Some(index) = chosen {
        let track = tracks.swap_remove(index);
        let (brand, compatible_brands) = (
            info.brand.take(),
            std::mem::take(&mut info.compatible_brands),
        );
        *info = track;
        info.brand = brand;
        info.compatible_brands = compatible_brands;
    }

    Ok(())
}

/// Parse trak (Track) box
fn parse_trak(
    cursor: &mut Cursor<&[u8]>,
    header: &BoxHeader,
    info: &mut Mp4Info,
    data: &[u8],
    depth: u8,
) -> Result<(), BitvueError> {
    // SECURITY: Check box nesting depth to prevent stack overflow
    if depth >= MAX_BOX_DEPTH {
        return Err(BitvueError::InvalidData(format!(
            "MP4 box depth {} exceeds maximum {}",
            depth, MAX_BOX_DEPTH
        )));
    }
    let child_depth = depth + 1;
    let box_end = header
        .data_offset
        .checked_add(header.data_size())
        .ok_or_else(|| BitvueError::InvalidData("MP4 child box end offset overflow".to_string()))?;

    while cursor.position() < box_end {
        let child_header = BoxHeader::parse(cursor)?;
        let child_end = child_header
            .data_offset
            .checked_add(child_header.data_size())
            .ok_or_else(|| {
                BitvueError::InvalidData("MP4 child box end offset overflow".to_string())
            })?;

        if &child_header.box_type == b"mdia" {
            // Media box
            parse_mdia(cursor, &child_header, info, data, child_depth)?;
        }

        cursor.seek(SeekFrom::Start(child_end))?;
    }

    Ok(())
}

/// Parse mdia (Media) box
fn parse_mdia(
    cursor: &mut Cursor<&[u8]>,
    header: &BoxHeader,
    info: &mut Mp4Info,
    data: &[u8],
    depth: u8,
) -> Result<(), BitvueError> {
    // SECURITY: Check box nesting depth to prevent stack overflow
    if depth >= MAX_BOX_DEPTH {
        return Err(BitvueError::InvalidData(format!(
            "MP4 box depth {} exceeds maximum {}",
            depth, MAX_BOX_DEPTH
        )));
    }
    let child_depth = depth + 1;
    let box_end = header
        .data_offset
        .checked_add(header.data_size())
        .ok_or_else(|| BitvueError::InvalidData("MP4 child box end offset overflow".to_string()))?;

    while cursor.position() < box_end {
        let child_header = BoxHeader::parse(cursor)?;
        let child_end = child_header
            .data_offset
            .checked_add(child_header.data_size())
            .ok_or_else(|| {
                BitvueError::InvalidData("MP4 child box end offset overflow".to_string())
            })?;

        match &child_header.box_type {
            b"mdhd" => {
                // Media header - contains timescale
                parse_mdhd(cursor, &child_header, info)?;
            }
            b"hdlr" => {
                // Handler: version/flags (4), pre_defined (4), handler_type (4)
                cursor.seek(SeekFrom::Current(8))?;
                let handler = read_box_type(cursor)?;
                info.handler_type = Some(String::from_utf8_lossy(&handler).into_owned());
            }
            b"minf" => {
                // Media information box
                parse_minf(cursor, &child_header, info, data, child_depth)?;
            }
            _ => {}
        }

        cursor.seek(SeekFrom::Start(child_end))?;
    }

    Ok(())
}

/// Parse minf (Media Information) box
fn parse_minf(
    cursor: &mut Cursor<&[u8]>,
    header: &BoxHeader,
    info: &mut Mp4Info,
    _data: &[u8],
    depth: u8,
) -> Result<(), BitvueError> {
    // SECURITY: Check box nesting depth to prevent stack overflow
    if depth >= MAX_BOX_DEPTH {
        return Err(BitvueError::InvalidData(format!(
            "MP4 box depth {} exceeds maximum {}",
            depth, MAX_BOX_DEPTH
        )));
    }
    let child_depth = depth + 1;
    let box_end = header
        .data_offset
        .checked_add(header.data_size())
        .ok_or_else(|| BitvueError::InvalidData("MP4 child box end offset overflow".to_string()))?;

    while cursor.position() < box_end {
        let child_header = BoxHeader::parse(cursor)?;
        let child_end = child_header
            .data_offset
            .checked_add(child_header.data_size())
            .ok_or_else(|| {
                BitvueError::InvalidData("MP4 child box end offset overflow".to_string())
            })?;

        if &child_header.box_type == b"stbl" {
            // Sample table box
            parse_stbl(cursor, &child_header, info, child_depth)?;
        }

        cursor.seek(SeekFrom::Start(child_end))?;
    }

    Ok(())
}

/// Parse stbl (Sample Table) box
fn parse_stbl(
    cursor: &mut Cursor<&[u8]>,
    header: &BoxHeader,
    info: &mut Mp4Info,
    depth: u8,
) -> Result<(), BitvueError> {
    // SECURITY: Check box nesting depth to prevent stack overflow
    if depth >= MAX_BOX_DEPTH {
        return Err(BitvueError::InvalidData(format!(
            "MP4 box depth {} exceeds maximum {}",
            depth, MAX_BOX_DEPTH
        )));
    }
    let child_depth = depth + 1;
    let box_end = header
        .data_offset
        .checked_add(header.data_size())
        .ok_or_else(|| BitvueError::InvalidData("MP4 child box end offset overflow".to_string()))?;

    while cursor.position() < box_end {
        let child_header = BoxHeader::parse(cursor)?;
        let child_end = child_header
            .data_offset
            .checked_add(child_header.data_size())
            .ok_or_else(|| {
                BitvueError::InvalidData("MP4 child box end offset overflow".to_string())
            })?;

        match &child_header.box_type {
            b"stsd" => {
                // Sample description (codec information) - may have nested boxes
                parse_stsd(cursor, &child_header, info, child_depth)?;
            }
            b"stts" => {
                // Sample time to sample (durations) - leaf parser
                parse_stts(cursor, &child_header, info)?;
            }
            b"stco" => {
                // Sample chunk offsets (32-bit) - leaf parser
                parse_stco(cursor, &child_header, info)?;
            }
            b"co64" => {
                // Sample chunk offsets (64-bit) - leaf parser
                parse_co64(cursor, &child_header, info)?;
            }
            b"stsz" => {
                // Sample sizes - leaf parser
                parse_stsz(cursor, &child_header, info)?;
            }
            b"ctts" => {
                // Composition time to sample (for PTS) - leaf parser
                parse_ctts(cursor, &child_header, info)?;
            }
            b"stss" => {
                // Sync sample table (key frames) - leaf parser
                parse_stss(cursor, &child_header, info)?;
            }
            b"stsc" => {
                // Sample-to-chunk runs - leaf parser
                parse_stsc(cursor, &child_header, info)?;
            }
            _ => {}
        }

        cursor.seek(SeekFrom::Start(child_end))?;
    }

    // Per-sample file offsets from chunk offsets + stsc + sizes
    resolve_sample_offsets(info)?;

    // Calculate timestamps from durations
    calculate_timestamps(info);

    // Calculate presentation timestamps (PTS = DTS + composition_offset)
    calculate_presentation_timestamps(info);

    Ok(())
}

/// Parse stsd (Sample Description) box
fn parse_stsd(
    cursor: &mut Cursor<&[u8]>,
    _header: &BoxHeader,
    info: &mut Mp4Info,
    _depth: u8, // Depth parameter for future nesting support
) -> Result<(), BitvueError> {
    cursor.seek(SeekFrom::Current(4))?; // version + flags

    let entry_count = read_u32(cursor)?;

    // Validate entry count to prevent DoS via massive allocations
    if entry_count > MAX_ENTRY_COUNT {
        return Err(BitvueError::InvalidData(format!(
            "Entry count {} exceeds maximum allowed {}",
            entry_count, MAX_ENTRY_COUNT
        )));
    }

    if entry_count > 0 {
        // Parse first sample entry
        let _entry_size = read_u32(cursor)?; // Size of sample entry
        let codec = read_box_type(cursor)?; // Codec fourcc (e.g. 'av01', 'avc1', 'hev1')

        info.codec = Some(String::from_utf8_lossy(&codec).to_string());

        // We don't need to parse the rest of the sample entry for now
        // Just need to know the codec type
    }

    Ok(())
}

/// Parse stsc (Sample To Chunk) box
fn parse_stsc(
    cursor: &mut Cursor<&[u8]>,
    _header: &BoxHeader,
    info: &mut Mp4Info,
) -> Result<(), BitvueError> {
    cursor.seek(SeekFrom::Current(4))?; // version + flags

    let entry_count = read_u32(cursor)?;
    if entry_count > MAX_ENTRY_COUNT {
        return Err(BitvueError::InvalidData(format!(
            "Entry count {} exceeds maximum allowed {}",
            entry_count, MAX_ENTRY_COUNT
        )));
    }

    for _ in 0..entry_count {
        let first_chunk = read_u32(cursor)?;
        let samples_per_chunk = read_u32(cursor)?;
        let _sample_description_index = read_u32(cursor)?;
        info.sample_to_chunk.push((first_chunk, samples_per_chunk));
    }

    Ok(())
}

/// Fill `sample_offsets` (one file offset per sample, sample order) from the chunk offsets, the
/// `stsc` runs and the sample sizes. Samples of one chunk follow each other from the chunk offset;
/// a track interleaved with others (audio + video) has many chunks.
///
/// Without an `stsc` box (not valid in a real file) the older reading is kept: one sample per chunk
/// when the counts match, otherwise all samples back to back from the first chunk.
fn resolve_sample_offsets(info: &mut Mp4Info) -> Result<(), BitvueError> {
    info.sample_offsets.clear();
    if info.chunk_offsets.is_empty() || info.sample_sizes.is_empty() {
        return Ok(());
    }
    let overflow = || BitvueError::InvalidData("Sample offset overflows".to_string());

    if info.sample_to_chunk.is_empty() {
        if info.chunk_offsets.len() == info.sample_sizes.len() {
            info.sample_offsets = info.chunk_offsets.clone();
        } else {
            let mut at = info.chunk_offsets[0];
            for &size in &info.sample_sizes {
                info.sample_offsets.push(at);
                at = at.checked_add(size as u64).ok_or_else(overflow)?;
            }
        }
        return Ok(());
    }

    let runs = &info.sample_to_chunk;
    if runs[0].0 != 1 || runs.windows(2).any(|w| w[1].0 <= w[0].0) {
        return Err(BitvueError::InvalidData(
            "stsc first_chunk values must start at 1 and increase".to_string(),
        ));
    }

    let total = info.sample_sizes.len();
    let mut sample = 0usize;
    for (chunk_index, &chunk_offset) in info.chunk_offsets.iter().enumerate() {
        if sample == total {
            break;
        }
        let chunk_number = chunk_index as u64 + 1;
        // The run in force for this chunk is the last one starting at or before it.
        let run = runs.partition_point(|&(first, _)| first as u64 <= chunk_number) - 1;
        let mut at = chunk_offset;
        for _ in 0..runs[run].1 {
            if sample == total {
                break;
            }
            info.sample_offsets.push(at);
            at = at
                .checked_add(info.sample_sizes[sample] as u64)
                .ok_or_else(overflow)?;
            sample += 1;
        }
    }

    if sample != total {
        return Err(BitvueError::InvalidData(format!(
            "stsc and the chunk offsets describe {} samples but stsz has {}",
            sample, total
        )));
    }
    Ok(())
}

/// Parse stco (Chunk Offset) box
fn parse_stco(
    cursor: &mut Cursor<&[u8]>,
    _header: &BoxHeader,
    info: &mut Mp4Info,
) -> Result<(), BitvueError> {
    let _version = read_u8(cursor)?;
    cursor.seek(SeekFrom::Current(3))?; // flags

    let entry_count = read_u32(cursor)?;

    // Validate entry count to prevent DoS via massive allocations
    if entry_count > MAX_ENTRY_COUNT {
        return Err(BitvueError::InvalidData(format!(
            "Entry count {} exceeds maximum allowed {}",
            entry_count, MAX_ENTRY_COUNT
        )));
    }

    for _ in 0..entry_count {
        let offset = read_u32(cursor)? as u64;
        info.chunk_offsets.push(offset);
    }

    Ok(())
}

/// Parse co64 (Chunk Offset 64) box
fn parse_co64(
    cursor: &mut Cursor<&[u8]>,
    _header: &BoxHeader,
    info: &mut Mp4Info,
) -> Result<(), BitvueError> {
    cursor.seek(SeekFrom::Current(4))?; // version + flags

    let entry_count = read_u32(cursor)?;

    // Validate entry count to prevent DoS via massive allocations
    if entry_count > MAX_ENTRY_COUNT {
        return Err(BitvueError::InvalidData(format!(
            "Entry count {} exceeds maximum allowed {}",
            entry_count, MAX_ENTRY_COUNT
        )));
    }

    for _ in 0..entry_count {
        let offset = read_u64(cursor)?;
        info.chunk_offsets.push(offset);
    }

    Ok(())
}

/// Parse stsz (Sample Size) box
fn parse_stsz(
    cursor: &mut Cursor<&[u8]>,
    _header: &BoxHeader,
    info: &mut Mp4Info,
) -> Result<(), BitvueError> {
    cursor.seek(SeekFrom::Current(4))?; // version + flags

    let sample_size = read_u32(cursor)?;
    let sample_count = read_u32(cursor)?;

    // Validate sample count to prevent DoS
    if sample_count > MAX_ENTRY_COUNT {
        return Err(BitvueError::InvalidData(format!(
            "Sample count {} exceeds maximum {}",
            sample_count, MAX_ENTRY_COUNT
        )));
    }

    // Pre-allocate based on sample count
    info.sample_count = sample_count as usize;
    info.sample_sizes.reserve(sample_count as usize);

    if sample_size == 0 {
        // Variable sample sizes
        for _ in 0..sample_count {
            let size = read_u32(cursor)?;
            info.sample_sizes.push(size);
        }
    } else {
        // Fixed sample size - still need to fill the vector
        for _ in 0..sample_count {
            info.sample_sizes.push(sample_size);
        }
    }

    Ok(())
}

/// Parse mdhd (Media Header) box
fn parse_mdhd(
    cursor: &mut Cursor<&[u8]>,
    _header: &BoxHeader,
    info: &mut Mp4Info,
) -> Result<(), BitvueError> {
    let version = read_u8(cursor)?;
    cursor.seek(SeekFrom::Current(3))?; // flags

    if version == 1 {
        // Version 1: 64-bit values
        cursor.seek(SeekFrom::Current(16))?; // creation_time + modification_time
        info.timescale = read_u32(cursor)?;
    } else {
        // Version 0: 32-bit values
        cursor.seek(SeekFrom::Current(8))?; // creation_time + modification_time
        info.timescale = read_u32(cursor)?;
    }

    Ok(())
}

/// Parse stts (Sample Time to Sample) box
fn parse_stts(
    cursor: &mut Cursor<&[u8]>,
    _header: &BoxHeader,
    info: &mut Mp4Info,
) -> Result<(), BitvueError> {
    cursor.seek(SeekFrom::Current(4))?; // version + flags

    let entry_count = read_u32(cursor)?;

    // Validate entry count to prevent DoS via massive allocations
    if entry_count > MAX_ENTRY_COUNT {
        return Err(BitvueError::InvalidData(format!(
            "Entry count {} exceeds maximum allowed {}",
            entry_count, MAX_ENTRY_COUNT
        )));
    }

    // Track total samples across all entries to prevent unbounded expansion
    let mut total_samples: usize = 0;

    // SECURITY: Limit samples per entry to prevent malicious inner loops
    const MAX_SAMPLES_PER_ENTRY: u32 = 10_000;

    for _ in 0..entry_count {
        let sample_count = read_u32(cursor)?;

        // Validate individual sample_count to prevent excessive loops
        if sample_count > MAX_SAMPLES_PER_ENTRY {
            return Err(BitvueError::InvalidData(format!(
                "Sample count per entry {} exceeds maximum {}",
                sample_count, MAX_SAMPLES_PER_ENTRY
            )));
        }

        let sample_delta = read_u32(cursor)?;

        // Use checked arithmetic to prevent overflow/wraparound bypass
        total_samples = total_samples
            .checked_add(sample_count as usize)
            .ok_or_else(|| {
                BitvueError::InvalidData("Sample count overflow - malicious MP4 file".to_string())
            })?;

        if total_samples > MAX_TOTAL_SAMPLES {
            return Err(BitvueError::InvalidData(format!(
                "Total sample count {} exceeds maximum allowed {}",
                total_samples, MAX_TOTAL_SAMPLES
            )));
        }

        // Expand the durations for each sample
        for _ in 0..sample_count {
            info.sample_durations.push(sample_delta);
        }
    }

    Ok(())
}

/// Parse ctts (Composition Time to Sample) box
fn parse_ctts(
    cursor: &mut Cursor<&[u8]>,
    _header: &BoxHeader,
    info: &mut Mp4Info,
) -> Result<(), BitvueError> {
    let version = read_u8(cursor)?;
    cursor.seek(SeekFrom::Current(3))?; // flags

    let entry_count = read_u32(cursor)?;

    // Validate entry count to prevent DoS via massive allocations
    if entry_count > MAX_ENTRY_COUNT {
        return Err(BitvueError::InvalidData(format!(
            "Entry count {} exceeds maximum allowed {}",
            entry_count, MAX_ENTRY_COUNT
        )));
    }

    for _ in 0..entry_count {
        let sample_count = read_u32(cursor)?;

        let sample_offset = if version == 1 {
            // Version 1: signed 32-bit offset
            let raw = read_u32(cursor)?;
            raw as i32
        } else {
            // Version 0: unsigned 32-bit offset (treated as signed)
            let raw = read_u32(cursor)?;
            raw as i32
        };

        // Expand the offsets for each sample
        for _ in 0..sample_count {
            info.composition_offsets.push(sample_offset);
        }
    }

    Ok(())
}

/// Parse stss (Sync Sample) box
fn parse_stss(
    cursor: &mut Cursor<&[u8]>,
    _header: &BoxHeader,
    info: &mut Mp4Info,
) -> Result<(), BitvueError> {
    cursor.seek(SeekFrom::Current(4))?; // version + flags

    let entry_count = read_u32(cursor)?;

    // Validate entry count to prevent DoS via massive allocations
    if entry_count > MAX_ENTRY_COUNT {
        return Err(BitvueError::InvalidData(format!(
            "Entry count {} exceeds maximum allowed {}",
            entry_count, MAX_ENTRY_COUNT
        )));
    }

    for _ in 0..entry_count {
        let sample_number = read_u32(cursor)?;
        info.key_frames.push(sample_number);
    }

    Ok(())
}

/// Calculate timestamps from durations
fn calculate_timestamps(info: &mut Mp4Info) {
    let mut timestamp = 0u64;

    for duration in &info.sample_durations {
        info.timestamps.push(timestamp);
        timestamp += *duration as u64;
    }
}

/// Calculate presentation timestamps (PTS = DTS + composition_offset)
fn calculate_presentation_timestamps(info: &mut Mp4Info) {
    // If no composition offsets, PTS = DTS
    if info.composition_offsets.is_empty() {
        info.presentation_timestamps = info.timestamps.clone();
        return;
    }

    // Calculate PTS for each sample
    for (i, &dts) in info.timestamps.iter().enumerate() {
        let offset = info.composition_offsets.get(i).copied().unwrap_or(0);
        // PTS = DTS + composition_offset
        // Handle potential negative offsets
        let pts = if offset < 0 {
            dts.saturating_sub((-offset) as u64)
        } else {
            dts.saturating_add(offset as u64)
        };
        info.presentation_timestamps.push(pts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_data() {
        let result = parse_mp4(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_codec_detection() {
        // Create a minimal MP4 structure with ftyp and moov boxes
        // This is a simplified structure just for testing codec detection
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&24u32.to_be_bytes()); // size
        data.extend_from_slice(b"ftyp"); // type
        data.extend_from_slice(b"isom"); // major brand
        data.extend_from_slice(&0u32.to_be_bytes()); // minor version
        data.extend_from_slice(b"isom"); // compatible brand
        data.extend_from_slice(b"av01"); // compatible brand

        // This is just a basic structure test - a real MP4 would have more boxes
        let result = parse_mp4(&data);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.brand, Some("isom".to_string()));
        assert!(info.compatible_brands.contains(&"av01".to_string()));
    }

    #[test]
    fn test_extract_non_av1_file() {
        // Test that extract_av1_samples rejects non-AV1 files
        let mut data = Vec::new();

        // Create minimal MP4 with non-AV1 codec
        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"isom");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(b"isom");

        // For now, this will fail because there's no moov/stsd box
        // But it demonstrates the validation
        let result = extract_av1_samples(&data);
        // Should fail because no codec info or non-AV1 codec
        assert!(result.is_err());
    }
}

/// Box-level tests: files and boxes are assembled by hand so every table parser, limit and error path runs.
#[cfg(test)]
mod box_tests {
    use super::*;

    fn bx(t: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut v = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(t);
        v.extend_from_slice(payload);
        v
    }

    fn full(t: &[u8; 4], version: u8, payload: &[u8]) -> Vec<u8> {
        let mut p = vec![version, 0, 0, 0];
        p.extend_from_slice(payload);
        bx(t, &p)
    }

    fn be(vals: &[u32]) -> Vec<u8> {
        vals.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    fn be64(vals: &[u64]) -> Vec<u8> {
        vals.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    fn stsd(codec: &[u8; 4]) -> Vec<u8> {
        let mut p = be(&[1]);
        p.extend(bx(codec, &[0u8; 78]));
        full(b"stsd", 0, &p)
    }

    fn stts(entries: &[(u32, u32)]) -> Vec<u8> {
        let mut p = be(&[entries.len() as u32]);
        for &(c, d) in entries {
            p.extend(be(&[c, d]));
        }
        full(b"stts", 0, &p)
    }

    fn stsz(sizes: &[u32]) -> Vec<u8> {
        let mut p = be(&[0, sizes.len() as u32]);
        p.extend(be(sizes));
        full(b"stsz", 0, &p)
    }

    fn stco(offsets: &[u32]) -> Vec<u8> {
        let mut p = be(&[offsets.len() as u32]);
        p.extend(be(offsets));
        full(b"stco", 0, &p)
    }

    fn mdhd_v0(timescale: u32) -> Vec<u8> {
        let mut p = be(&[0, 0, timescale, 0]);
        p.extend_from_slice(&[0x55, 0xC4, 0, 0]);
        full(b"mdhd", 0, &p)
    }

    fn ftyp() -> Vec<u8> {
        let mut p = b"isom".to_vec();
        p.extend(be(&[512]));
        p.extend_from_slice(b"isomav01");
        bx(b"ftyp", &p)
    }

    /// A one-track file. `per_chunk_one` puts every sample in its own chunk, otherwise all in one chunk.
    fn file(
        codec: &[u8; 4],
        samples: &[&[u8]],
        per_chunk_one: bool,
        extra_stbl: &[Vec<u8>],
    ) -> Vec<u8> {
        let sizes: Vec<u32> = samples.iter().map(|s| s.len() as u32).collect();
        let moov_with = |offsets: &[u32]| {
            let mut stbl = stsd(codec);
            stbl.extend(stts(&[(samples.len() as u32, 1000)]));
            stbl.extend(stsz(&sizes));
            stbl.extend(stco(offsets));
            for e in extra_stbl {
                stbl.extend(e);
            }
            let mut minf = bx(b"stbl", &stbl);
            minf = bx(b"minf", &minf);
            let mut mdia = mdhd_v0(90000);
            mdia.extend(minf);
            bx(b"moov", &bx(b"trak", &bx(b"mdia", &mdia)))
        };
        let n = if per_chunk_one { samples.len() } else { 1 };
        let moov_len = moov_with(&vec![0; n]).len();
        let mdat_start = (ftyp().len() + moov_len + 8) as u32;
        let offsets: Vec<u32> = if per_chunk_one {
            let mut acc = mdat_start;
            sizes
                .iter()
                .map(|s| {
                    let o = acc;
                    acc += s;
                    o
                })
                .collect()
        } else {
            vec![mdat_start]
        };
        let mut f = ftyp();
        f.extend(moov_with(&offsets));
        f.extend(bx(b"mdat", &samples.concat()));
        f
    }

    /// Run a leaf parser on a box and return its result and the filled-in info.
    fn run<F>(boxed: &[u8], f: F) -> (Result<(), BitvueError>, Mp4Info)
    where
        F: FnOnce(&mut Cursor<&[u8]>, &BoxHeader, &mut Mp4Info) -> Result<(), BitvueError>,
    {
        let mut c = Cursor::new(boxed);
        let h = BoxHeader::parse(&mut c).unwrap();
        let mut info = Mp4Info::default();
        let r = f(&mut c, &h, &mut info);
        (r, info)
    }

    fn invalid(r: Result<(), BitvueError>, needle: &str) {
        let e = r.unwrap_err().to_string();
        assert!(e.contains(needle), "{e}");
    }

    // ---- BoxHeader ----

    #[test]
    fn box_header_forms() {
        let b = bx(b"free", &[1, 2, 3]);
        let h = BoxHeader::parse(&mut Cursor::new(&b[..])).unwrap();
        assert_eq!((h.size, h.data_offset, h.data_size()), (11, 8, 3));
        assert_eq!(h.box_type_str(), "free");

        // 64-bit size: header is 16 bytes
        let mut big = vec![0, 0, 0, 1];
        big.extend_from_slice(b"mdat");
        big.extend(be64(&[(u32::MAX as u64) + 100]));
        let h = BoxHeader::parse(&mut Cursor::new(&big[..])).unwrap();
        assert_eq!((h.size, h.data_offset), ((u32::MAX as u64) + 100, 16));
        assert_eq!(h.data_size(), (u32::MAX as u64) + 100 - 16);

        // size 0: extends to the end of the data, counted from the box start
        let mut z = vec![9u8; 5];
        z.extend(be(&[0]));
        z.extend_from_slice(b"mdat");
        z.extend_from_slice(&[7; 10]);
        let mut c = Cursor::new(&z[..]);
        c.set_position(5);
        let h = BoxHeader::parse(&mut c).unwrap();
        assert_eq!((h.size, h.data_offset, h.data_size()), (18, 13, 10));
    }

    #[test]
    fn box_header_rejects_bad_sizes_and_truncation() {
        let mut small = be(&[7]);
        small.extend_from_slice(b"free");
        let e = BoxHeader::parse(&mut Cursor::new(&small[..])).unwrap_err();
        assert!(matches!(e, BitvueError::Parse { offset: 0, .. }), "{e}");
        // 64-bit form announced, size field cut off
        let mut cut = be(&[1]);
        cut.extend_from_slice(b"free");
        cut.extend_from_slice(&[0, 0, 0]);
        assert!(matches!(
            BoxHeader::parse(&mut Cursor::new(&cut[..])),
            Err(BitvueError::UnexpectedEof(_))
        ));
        assert!(BoxHeader::parse(&mut Cursor::new(&[0u8, 0, 0][..])).is_err());
        assert!(BoxHeader::parse(&mut Cursor::new(&[0u8, 0, 0, 8, b'a', b'b'][..])).is_err());
        // 64-bit size smaller than its own header
        let mut tiny = be(&[1]);
        tiny.extend_from_slice(b"free");
        tiny.extend(be64(&[12]));
        assert!(BoxHeader::parse(&mut Cursor::new(&tiny[..])).is_err());
    }

    #[test]
    fn box_type_str_handles_non_ascii() {
        let h = BoxHeader {
            box_type: [b'a', 0xA9, b'c', b'd'],
            size: 8,
            data_offset: 8,
            header_size: 8,
        };
        assert_eq!(h.box_type_str(), "a\u{FFFD}cd");
        let h = BoxHeader {
            box_type: *b"moov",
            size: 8,
            data_offset: 8,
            header_size: 8,
        };
        assert_eq!(h.box_type_str(), "moov");
    }

    #[test]
    fn primitive_readers_fail_with_unexpected_eof() {
        let d = [1u8, 2, 3];
        let mut c = Cursor::new(&d[..]);
        assert_eq!(read_u8(&mut c).unwrap(), 1);
        assert!(matches!(
            read_u32(&mut c),
            Err(BitvueError::UnexpectedEof(_))
        ));
        let mut c = Cursor::new(&d[..]);
        assert!(matches!(
            read_u64(&mut c),
            Err(BitvueError::UnexpectedEof(_))
        ));
        assert!(matches!(
            read_box_type(&mut c),
            Err(BitvueError::UnexpectedEof(_))
        ));
        let mut c = Cursor::new(&[][..]);
        assert!(matches!(
            read_u8(&mut c),
            Err(BitvueError::UnexpectedEof(_))
        ));
        let mut c = Cursor::new(&[0u8, 0, 1, 2, 3, 4, 5, 6][..]);
        assert_eq!(read_u64(&mut c).unwrap(), 0x0000_0102_0304_0506);
    }

    // ---- parse_mp4 on a whole file ----

    #[test]
    fn parse_mp4_reads_the_whole_sample_table() {
        let s: [&[u8]; 3] = [&[1, 2, 3], &[4, 5], &[6]];
        let ctts = full(
            b"ctts",
            0,
            &[be(&[2]), be(&[2, 2000]), be(&[1, 0])].concat(),
        );
        let stss = full(b"stss", 0, &[be(&[2]), be(&[1, 3])].concat());
        let f = file(b"avc1", &s, true, &[ctts, stss]);
        let i = parse_mp4(&f).unwrap();
        assert_eq!(i.brand.as_deref(), Some("isom"));
        assert_eq!(i.compatible_brands, vec!["isom", "av01"]);
        assert_eq!(i.codec.as_deref(), Some("avc1"));
        assert_eq!(i.timescale, 90000);
        assert_eq!(i.sample_count, 3);
        assert_eq!(i.sample_sizes, vec![3, 2, 1]);
        assert_eq!(i.sample_durations, vec![1000, 1000, 1000]);
        assert_eq!(i.timestamps, vec![0, 1000, 2000]);
        assert_eq!(i.composition_offsets, vec![2000, 2000, 0]);
        assert_eq!(i.presentation_timestamps, vec![2000, 3000, 2000]);
        assert_eq!(i.key_frames, vec![1, 3]);
        assert_eq!(i.sample_offsets.len(), 3);
        assert_eq!(i.sample_offsets[1] - i.sample_offsets[0], 3);
        assert_eq!(&f[i.sample_offsets[0] as usize..][..3], &[1, 2, 3]);
    }

    #[test]
    fn parse_mp4_skips_unknown_boxes_and_stops_at_a_size_zero_box() {
        let mut f = bx(b"free", &[0; 5]);
        f.extend(ftyp());
        f.extend(bx(b"skip", &[]));
        // size 0 box: runs to the end of the data
        f.extend(be(&[0]));
        f.extend_from_slice(b"mdat");
        f.extend_from_slice(&[1, 2, 3]);
        let i = parse_mp4(&f).unwrap();
        assert_eq!(i.brand.as_deref(), Some("isom"));
        assert!(i.codec.is_none());
    }

    #[test]
    fn parse_mp4_rejects_boxes_that_run_past_the_file() {
        let mut f = ftyp();
        let mut tail = bx(b"free", &[0; 8]);
        tail.truncate(tail.len() - 4);
        f.extend(tail);
        let e = parse_mp4(&f).unwrap_err().to_string();
        assert!(e.contains("extends beyond file"), "{e}");
        assert!(parse_mp4(&[]).is_err());
        assert!(parse_mp4(&[0, 0]).is_err());
    }

    #[test]
    fn truncated_tables_are_errors() {
        let f = file(b"avc1", &[&[1u8, 2][..]], true, &[]);
        // chop the file inside moov: the declared box no longer fits
        assert!(parse_mp4(&f[..ftyp().len() + 20]).is_err());
    }

    #[test]
    fn child_box_end_beyond_data_is_an_error_not_a_panic() {
        // a trak whose child declares more bytes than exist: seek/parse fails cleanly
        let inner = {
            let mut v = be(&[1000]);
            v.extend_from_slice(b"mdia");
            v
        };
        let moov = bx(b"moov", &bx(b"trak", &inner));
        assert!(parse_mp4(&moov).is_err());
    }

    // ---- sample extraction ----

    #[test]
    fn extractors_return_borrowed_slices_for_chunk_per_sample_layout() {
        let s: [&[u8]; 3] = [&[1, 2, 3], &[4, 5], &[6]];
        let f = file(b"av01", &s, true, &[]);
        let got = extract_av1_samples(&f).unwrap();
        assert_eq!(got.len(), 3);
        for (g, e) in got.iter().zip(s) {
            assert_eq!(&g[..], e);
            assert!(matches!(g, Cow::Borrowed(_)));
        }
        assert!(extract_avc_samples(&f).is_err());
        assert!(extract_hevc_samples(&f).is_err());
    }

    #[test]
    fn extractors_derive_offsets_when_samples_share_one_chunk() {
        let s: [&[u8]; 3] = [&[1, 2, 3], &[4, 5], &[6]];
        for (codec, which) in [(b"avc1", 0), (b"avc3", 0), (b"hev1", 1), (b"hvc1", 1)] {
            let f = file(codec, &s, false, &[]);
            let got = if which == 0 {
                extract_avc_samples(&f)
            } else {
                extract_hevc_samples(&f)
            }
            .unwrap();
            let bytes: Vec<&[u8]> = got.iter().map(|c| &c[..]).collect();
            assert_eq!(bytes, s.to_vec(), "{}", String::from_utf8_lossy(codec));
        }
    }

    #[test]
    fn extractor_error_messages_name_the_codec() {
        let f = file(b"vp09", &[&[1u8][..]], true, &[]);
        let e = extract_av1_samples(&f).unwrap_err().to_string();
        assert!(e.contains("Not an AV1 file") && e.contains("vp09"), "{e}");
        let e = extract_avc_samples(&f).unwrap_err().to_string();
        assert!(e.contains("H.264/AVC"), "{e}");
        let e = extract_hevc_samples(&f).unwrap_err().to_string();
        assert!(e.contains("H.265/HEVC"), "{e}");
        // no stsd at all
        let e = extract_av1_samples(&ftyp()).unwrap_err().to_string();
        assert!(e.contains("No codec information"), "{e}");
    }

    #[test]
    fn extraction_validates_offsets_sizes_and_overlap() {
        let sample_file = |sizes: &[u32], offsets: &[u32], mdat_len: usize| {
            let mut stbl = stsd(b"av01");
            stbl.extend(stts(&[(sizes.len() as u32, 1)]));
            stbl.extend(stsz(sizes));
            stbl.extend(stco(offsets));
            let moov = bx(
                b"moov",
                &bx(b"trak", &bx(b"mdia", &bx(b"minf", &bx(b"stbl", &stbl)))),
            );
            let mut f = moov;
            f.extend(vec![0u8; mdat_len]);
            f
        };
        // sample runs past the end of the file
        let f = sample_file(&[100_000], &[0], 10);
        let e = extract_av1_samples(&f).unwrap_err().to_string();
        assert!(e.contains("exceeds file size"), "{e}");
        // overlapping samples (one sample per chunk, sorted by offset)
        let f = sample_file(&[10, 10], &[0, 5], 40);
        let e = extract_av1_samples(&f).unwrap_err().to_string();
        assert!(e.contains("overlap"), "{e}");
        // adjacent (touching) samples are fine
        let f = sample_file(&[10, 10], &[0, 10], 40);
        assert_eq!(extract_av1_samples(&f).unwrap().len(), 2);
        // no chunk offsets at all
        let mut stbl = stsd(b"av01");
        stbl.extend(stsz(&[4]));
        let f = bx(
            b"moov",
            &bx(b"trak", &bx(b"mdia", &bx(b"minf", &bx(b"stbl", &stbl)))),
        );
        let e = extract_av1_samples(&f).unwrap_err().to_string();
        assert!(e.contains("No chunk offsets"), "{e}");
    }

    #[test]
    fn extraction_rejects_huge_samples_and_sample_counts() {
        let mut stbl = stsd(b"av01");
        stbl.extend(stsz(&[(MAX_SAMPLE_SIZE as u32) + 1]));
        stbl.extend(stco(&[0]));
        let f = bx(
            b"moov",
            &bx(b"trak", &bx(b"mdia", &bx(b"minf", &bx(b"stbl", &stbl)))),
        );
        let e = extract_av1_samples(&f).unwrap_err().to_string();
        assert!(e.contains("exceeds maximum allowed"), "{e}");

        let mut stbl = stsd(b"av01");
        let mut p = be(&[1, (MAX_TOTAL_SAMPLES as u32) + 1]); // fixed size 1, too many samples
        p.truncate(8);
        stbl.extend(full(b"stsz", 0, &p));
        stbl.extend(stco(&[0]));
        let f = bx(
            b"moov",
            &bx(b"trak", &bx(b"mdia", &bx(b"minf", &bx(b"stbl", &stbl)))),
        );
        let e = extract_av1_samples(&f).unwrap_err().to_string();
        assert!(e.contains("Sample count"), "{e}");
    }

    // ---- leaf parsers ----

    #[test]
    fn stsd_reads_the_first_codec_and_limits_entries() {
        let (r, i) = run(&stsd(b"hev1"), |c, h, i| parse_stsd(c, h, i, 0));
        r.unwrap();
        assert_eq!(i.codec.as_deref(), Some("hev1"));
        let (r, i) = run(&full(b"stsd", 0, &be(&[0])), |c, h, i| {
            parse_stsd(c, h, i, 0)
        });
        r.unwrap();
        assert!(i.codec.is_none());
        let (r, _) = run(&full(b"stsd", 0, &be(&[MAX_ENTRY_COUNT + 1])), |c, h, i| {
            parse_stsd(c, h, i, 0)
        });
        invalid(r, "Entry count");
        let (r, _) = run(&full(b"stsd", 0, &be(&[MAX_ENTRY_COUNT])), |c, h, i| {
            parse_stsd(c, h, i, 0)
        });
        assert!(r.is_err()); // exactly at the limit is accepted by the check; the entry itself is missing
        let (r, _) = run(&full(b"stsd", 0, &be(&[1, 20])), |c, h, i| {
            parse_stsd(c, h, i, 0)
        });
        assert!(r.is_err());
    }

    #[test]
    fn stco_and_co64_read_offsets() {
        let (r, i) = run(&stco(&[10, 0xFFFF_FFFF]), parse_stco);
        r.unwrap();
        assert_eq!(i.chunk_offsets, vec![10, 0xFFFF_FFFF]);
        let mut p = be(&[2]);
        p.extend(be64(&[5, 1 << 40]));
        let (r, i) = run(&full(b"co64", 0, &p), parse_co64);
        r.unwrap();
        assert_eq!(i.chunk_offsets, vec![5, 1 << 40]);
        for (name, parse) in [
            (
                "stco",
                parse_stco
                    as fn(&mut Cursor<&[u8]>, &BoxHeader, &mut Mp4Info) -> Result<(), BitvueError>,
            ),
            ("co64", parse_co64),
        ] {
            let (r, _) = run(
                &full(
                    name.as_bytes().try_into().unwrap(),
                    0,
                    &be(&[MAX_ENTRY_COUNT + 1]),
                ),
                parse,
            );
            invalid(r, "Entry count");
            let (r, _) = run(
                &full(name.as_bytes().try_into().unwrap(), 0, &be(&[2, 1])),
                parse,
            );
            assert!(r.is_err(), "{name} truncated");
        }
    }

    #[test]
    fn stsz_fixed_and_variable_sizes() {
        let (r, i) = run(&stsz(&[5, 6, 7]), parse_stsz);
        r.unwrap();
        assert_eq!((i.sample_count, i.sample_sizes.clone()), (3, vec![5, 6, 7]));
        let (r, i) = run(&full(b"stsz", 0, &be(&[9, 4])), parse_stsz);
        r.unwrap();
        assert_eq!((i.sample_count, i.sample_sizes), (4, vec![9, 9, 9, 9]));
        let (r, _) = run(
            &full(b"stsz", 0, &be(&[0, MAX_ENTRY_COUNT + 1])),
            parse_stsz,
        );
        invalid(r, "Sample count");
        let (r, _) = run(&full(b"stsz", 0, &be(&[0, 3, 1])), parse_stsz);
        assert!(r.is_err());
    }

    #[test]
    fn mdhd_reads_the_timescale_for_both_versions() {
        let (r, i) = run(&mdhd_v0(24000), parse_mdhd);
        r.unwrap();
        assert_eq!(i.timescale, 24000);
        let mut p = be64(&[0, 0]);
        p.extend(be(&[48000]));
        p.extend(be64(&[0]));
        let (r, i) = run(&full(b"mdhd", 1, &p), parse_mdhd);
        r.unwrap();
        assert_eq!(i.timescale, 48000);
        let (r, _) = run(&full(b"mdhd", 0, &be(&[0, 0])), parse_mdhd);
        assert!(r.is_err());
    }

    #[test]
    fn stts_expands_durations_and_enforces_limits() {
        let (r, i) = run(&stts(&[(2, 100), (3, 50)]), parse_stts);
        r.unwrap();
        assert_eq!(i.sample_durations, vec![100, 100, 50, 50, 50]);
        let (r, _) = run(&stts(&[(10_001, 1)]), parse_stts);
        invalid(r, "per entry");
        let (r, i) = run(&stts(&[(10_000, 1)]), parse_stts);
        r.unwrap();
        assert_eq!(i.sample_durations.len(), 10_000);
        let ten: Vec<(u32, u32)> = vec![(10_000, 1); 11];
        let (r, _) = run(&stts(&ten), parse_stts);
        invalid(r, "Total sample count");
        let ten: Vec<(u32, u32)> = vec![(10_000, 1); 10];
        let (r, _) = run(&stts(&ten), parse_stts);
        r.unwrap();
        let (r, _) = run(&full(b"stts", 0, &be(&[MAX_ENTRY_COUNT + 1])), parse_stts);
        invalid(r, "Entry count");
        let (r, _) = run(&full(b"stts", 0, &be(&[1, 5])), parse_stts);
        assert!(r.is_err());
    }

    #[test]
    fn ctts_reads_signed_offsets_in_both_versions() {
        let body = [be(&[2]), be(&[1, 7]), be(&[2, 0xFFFF_FFFE])].concat(); // 7, then -2 twice
        for version in [0u8, 1] {
            let (r, i) = run(&full(b"ctts", version, &body), parse_ctts);
            r.unwrap();
            assert_eq!(i.composition_offsets, vec![7, -2, -2], "version {version}");
        }
        let (r, _) = run(&full(b"ctts", 0, &be(&[MAX_ENTRY_COUNT + 1])), parse_ctts);
        invalid(r, "Entry count");
        let (r, _) = run(&full(b"ctts", 0, &be(&[1, 1])), parse_ctts);
        assert!(r.is_err());
    }

    #[test]
    fn stss_reads_sync_samples() {
        let (r, i) = run(
            &full(b"stss", 0, &[be(&[3]), be(&[1, 30, 60])].concat()),
            parse_stss,
        );
        r.unwrap();
        assert_eq!(i.key_frames, vec![1, 30, 60]);
        let (r, _) = run(&full(b"stss", 0, &be(&[MAX_ENTRY_COUNT + 1])), parse_stss);
        invalid(r, "Entry count");
        let (r, _) = run(&full(b"stss", 0, &be(&[2, 1])), parse_stss);
        assert!(r.is_err());
    }

    #[test]
    fn ftyp_without_compatible_brands() {
        let mut p = b"mp42".to_vec();
        p.extend(be(&[0]));
        let (r, i) = run(&bx(b"ftyp", &p), parse_ftyp);
        r.unwrap();
        assert_eq!(i.brand.as_deref(), Some("mp42"));
        assert!(i.compatible_brands.is_empty());
        // trailing bytes that do not make a whole brand are ignored
        p.extend_from_slice(b"isomXX");
        let (_, i) = run(&bx(b"ftyp", &p), parse_ftyp);
        assert_eq!(i.compatible_brands, vec!["isom"]);
    }

    // ---- timestamps ----

    #[test]
    fn presentation_timestamps_follow_composition_offsets() {
        let mut i = Mp4Info {
            sample_durations: vec![10, 10, 10, 10],
            ..Default::default()
        };
        calculate_timestamps(&mut i);
        assert_eq!(i.timestamps, vec![0, 10, 20, 30]);
        calculate_presentation_timestamps(&mut i);
        assert_eq!(i.presentation_timestamps, i.timestamps);

        let mut i = Mp4Info {
            sample_durations: vec![10; 4],
            ..Default::default()
        };
        calculate_timestamps(&mut i);
        // negative offset larger than the DTS saturates at 0; fewer offsets than samples default to 0
        i.composition_offsets = vec![-5, 3, -100];
        calculate_presentation_timestamps(&mut i);
        assert_eq!(i.presentation_timestamps, vec![0, 13, 0, 30]);
    }

    // ---- nesting limit (the parsers recurse at most 5 deep on real files; the guard is reachable only directly) ----

    #[test]
    fn nesting_depth_limit_is_enforced_by_every_container_parser() {
        let boxed = bx(b"moov", &[]);
        let mut c = Cursor::new(&boxed[..]);
        let h = BoxHeader::parse(&mut c).unwrap();
        let mut info = Mp4Info::default();
        let data = &boxed[..];
        for depth in [MAX_BOX_DEPTH, MAX_BOX_DEPTH + 1] {
            invalid(parse_moov(&mut c, &h, &mut info, data, depth), "depth");
            invalid(parse_trak(&mut c, &h, &mut info, data, depth), "depth");
            invalid(parse_mdia(&mut c, &h, &mut info, data, depth), "depth");
            invalid(parse_minf(&mut c, &h, &mut info, data, depth), "depth");
            invalid(parse_stbl(&mut c, &h, &mut info, depth), "depth");
        }
        // one below the limit is fine for an empty box
        parse_moov(&mut c, &h, &mut info, data, MAX_BOX_DEPTH - 1).unwrap();
        parse_trak(&mut c, &h, &mut info, data, MAX_BOX_DEPTH - 1).unwrap();
        parse_mdia(&mut c, &h, &mut info, data, MAX_BOX_DEPTH - 1).unwrap();
        parse_minf(&mut c, &h, &mut info, data, MAX_BOX_DEPTH - 1).unwrap();
        parse_stbl(&mut c, &h, &mut info, MAX_BOX_DEPTH - 1).unwrap();
    }

    #[test]
    fn limits_are_inclusive() {
        // exactly at the limit passes the limit check: the error (if any) is the missing data, not the limit
        let at = be(&[MAX_ENTRY_COUNT]);
        type Leaf = fn(&mut Cursor<&[u8]>, &BoxHeader, &mut Mp4Info) -> Result<(), BitvueError>;
        let leaves: [(&[u8; 4], Leaf); 6] = [
            (b"stco", parse_stco),
            (b"co64", parse_co64),
            (b"stts", parse_stts),
            (b"ctts", parse_ctts),
            (b"stss", parse_stss),
            (b"stsd", |c, h, i| parse_stsd(c, h, i, 0)),
        ];
        for (name, parse) in leaves {
            let (r, _) = run(&full(name, 0, &at), parse);
            let e = r.unwrap_err().to_string();
            assert!(
                !e.contains("exceeds maximum"),
                "{}: {e}",
                String::from_utf8_lossy(name)
            );
        }
        let (r, _) = run(&full(b"stsz", 0, &be(&[0, MAX_ENTRY_COUNT])), parse_stsz);
        assert!(!r.unwrap_err().to_string().contains("exceeds maximum"));

        // sample table limits in the extractor: exactly MAX_TOTAL_SAMPLES samples / MAX_SAMPLE_SIZE bytes pass the check
        let wrap = |stbl: Vec<u8>| {
            bx(
                b"moov",
                &bx(b"trak", &bx(b"mdia", &bx(b"minf", &bx(b"stbl", &stbl)))),
            )
        };
        let mut stbl = stsd(b"av01");
        stbl.extend(full(b"stsz", 0, &be(&[1, MAX_TOTAL_SAMPLES as u32])));
        stbl.extend(stco(&[0]));
        let e = extract_av1_samples(&wrap(stbl)).unwrap_err().to_string();
        assert!(e.contains("exceeds file size"), "{e}");
        let mut stbl = stsd(b"av01");
        stbl.extend(stsz(&[MAX_SAMPLE_SIZE as u32]));
        stbl.extend(stco(&[0]));
        let e = extract_av1_samples(&wrap(stbl)).unwrap_err().to_string();
        assert!(e.contains("exceeds file size"), "{e}");
    }

    #[test]
    fn co64_offsets_are_used_when_parsing_a_file() {
        let mut co = be(&[1]);
        co.extend(be64(&[0x1_0000_0010]));
        let mut stbl = stsd(b"av01");
        stbl.extend(stsz(&[4]));
        stbl.extend(full(b"co64", 0, &co));
        let f = bx(
            b"moov",
            &bx(b"trak", &bx(b"mdia", &bx(b"minf", &bx(b"stbl", &stbl)))),
        );
        assert_eq!(parse_mp4(&f).unwrap().sample_offsets, vec![0x1_0000_0010]);
        // the extractor reports an offset that does not fit the file
        let e = extract_av1_samples(&f).unwrap_err().to_string();
        assert!(
            e.contains("exceeds file size") || e.contains("address space"),
            "{e}"
        );
    }

    // ---- multi-track files, stsc and sample order (#263, #265) ----

    fn stsc(runs: &[(u32, u32)]) -> Vec<u8> {
        let mut p = be(&[runs.len() as u32]);
        for &(first_chunk, per_chunk) in runs {
            p.extend(be(&[first_chunk, per_chunk, 1]));
        }
        full(b"stsc", 0, &p)
    }

    fn hdlr(kind: &[u8; 4]) -> Vec<u8> {
        let mut p = be(&[0]); // pre_defined
        p.extend_from_slice(kind);
        p.extend_from_slice(&[0; 12]); // reserved
        p.push(0); // empty name
        full(b"hdlr", 0, &p)
    }

    /// `trak` with a handler, a codec, sizes, stsc runs and chunk offsets.
    fn trak(
        kind: &[u8; 4],
        codec: &[u8; 4],
        sizes: &[u32],
        runs: &[(u32, u32)],
        chunks: &[u32],
    ) -> Vec<u8> {
        let mut stbl = stsd(codec);
        stbl.extend(stsz(sizes));
        stbl.extend(stsc(runs));
        stbl.extend(stco(chunks));
        let mut mdia = mdhd_v0(90000);
        mdia.extend(hdlr(kind));
        mdia.extend(bx(b"minf", &bx(b"stbl", &stbl)));
        bx(b"trak", &bx(b"mdia", &mdia))
    }

    /// ftyp + moov, then zero padding to 4000 bytes; media data lives at offsets from 2000 on
    /// (past any moov these tests build).
    fn mp4_with(traks: &[Vec<u8>]) -> Vec<u8> {
        let mut f = ftyp();
        f.extend(bx(b"moov", &traks.concat()));
        assert!(f.len() < 2000);
        f.resize(4000, 0);
        f
    }

    #[test]
    fn interleaved_audio_and_video_give_the_video_samples() {
        // audio track listed first; video: 3 samples in 2 chunks (2 + 1), audio chunks in between
        let mut f = mp4_with(&[
            trak(b"soun", b"mp4a", &[4, 4, 4], &[(1, 1)], &[2300, 2400, 2500]),
            trak(
                b"vide",
                b"avc1",
                &[2, 3, 1],
                &[(1, 2), (2, 1)],
                &[2200, 2350],
            ),
        ]);
        f[2200..2202].copy_from_slice(&[1, 1]);
        f[2202..2205].copy_from_slice(&[2, 2, 2]);
        f[2350] = 3;
        // audio bytes at 300.. must not be picked up
        f[2300..2304].copy_from_slice(&[9, 9, 9, 9]);
        let info = parse_mp4(&f).unwrap();
        assert_eq!(info.codec.as_deref(), Some("avc1"));
        assert_eq!(info.handler_type.as_deref(), Some("vide"));
        assert_eq!(info.sample_sizes, vec![2, 3, 1]);
        assert_eq!(info.sample_offsets, vec![2200, 2202, 2350]);
        assert_eq!(info.brand.as_deref(), Some("isom"));
        let samples = extract_avc_samples(&f).unwrap();
        assert_eq!(
            samples.iter().map(|s| s.to_vec()).collect::<Vec<_>>(),
            vec![vec![1, 1], vec![2, 2, 2], vec![3]]
        );
    }

    #[test]
    fn video_track_is_chosen_whichever_position_it_has() {
        for video_first in [true, false] {
            let v = trak(b"vide", b"av01", &[1], &[(1, 1)], &[100]);
            let a = trak(b"soun", b"mp4a", &[1], &[(1, 1)], &[200]);
            let f = mp4_with(&if video_first { [v, a] } else { [a, v] });
            assert_eq!(parse_mp4(&f).unwrap().codec.as_deref(), Some("av01"));
        }
    }

    #[test]
    fn a_file_with_only_an_audio_track_names_it_in_the_error() {
        let f = mp4_with(&[trak(b"soun", b"mp4a", &[1], &[(1, 1)], &[100])]);
        let e = extract_avc_samples(&f).unwrap_err().to_string();
        assert!(e.contains("mp4a"), "{e}");
    }

    #[test]
    fn stsc_runs_place_samples_in_each_chunk() {
        // chunks: #1 holds 2 samples, #2..#3 hold 3 each (run starting at chunk 2)
        let f = mp4_with(&[trak(
            b"vide",
            b"av01",
            &[1, 2, 3, 1, 1, 1, 2, 2],
            &[(1, 2), (2, 3)],
            &[100, 200, 300],
        )]);
        assert_eq!(
            parse_mp4(&f).unwrap().sample_offsets,
            vec![100, 101, 200, 203, 204, 300, 301, 303]
        );
    }

    #[test]
    fn stsc_that_does_not_cover_every_sample_is_an_error() {
        // 5 samples declared, the chunks only hold 2 + 2
        let f = mp4_with(&[trak(b"vide", b"av01", &[1; 5], &[(1, 2)], &[100, 200])]);
        invalid_parse(&f, "describe 4 samples but stsz has 5");
    }

    #[test]
    fn stsc_with_unordered_or_zero_based_runs_is_an_error() {
        for runs in [
            vec![(2, 1)],
            vec![(1, 1), (1, 2)],
            vec![(1, 1), (3, 1), (2, 1)],
        ] {
            let f = mp4_with(&[trak(b"vide", b"av01", &[1], &runs, &[100])]);
            invalid_parse(&f, "first_chunk");
        }
    }

    fn invalid_parse(f: &[u8], needle: &str) {
        let e = parse_mp4(f).unwrap_err().to_string();
        assert!(e.contains(needle), "{e}");
    }

    #[test]
    fn samples_come_back_in_sample_order_not_file_order() {
        // sample 0 sits later in the file than sample 1 (#265)
        let mut f = mp4_with(&[trak(b"vide", b"av01", &[2, 3], &[(1, 1)], &[2500, 2100])]);
        f[2500..2502].copy_from_slice(&[7, 7]);
        f[2100..2103].copy_from_slice(&[8, 8, 8]);
        let got: Vec<Vec<u8>> = extract_av1_samples(&f)
            .unwrap()
            .iter()
            .map(|s| s.to_vec())
            .collect();
        assert_eq!(got, vec![vec![7, 7], vec![8, 8, 8]]);
    }

    #[test]
    fn overlap_is_detected_between_file_neighbours_whatever_their_sample_order() {
        // sample 0 at 100..110, sample 1 at 50..60, sample 2 at 105..110 overlaps sample 0
        let f = mp4_with(&[trak(
            b"vide",
            b"av01",
            &[10, 10, 5],
            &[(1, 1)],
            &[100, 50, 105],
        )]);
        let e = extract_av1_samples(&f).unwrap_err().to_string();
        assert!(e.contains("overlap"), "{e}");
    }

    // ---- box header forms and short ftyp (#264) ----

    #[test]
    fn chunk_offsets_out_of_file_order_are_not_an_overlap() {
        let mut stbl = stsd(b"av01");
        stbl.extend(stsz(&[2, 3]));
        stbl.extend(stco(&[20, 0]));
        let mut f = bx(
            b"moov",
            &bx(b"trak", &bx(b"mdia", &bx(b"minf", &bx(b"stbl", &stbl)))),
        );
        f.extend(vec![0u8; 40]);
        assert_eq!(extract_av1_samples(&f).unwrap().len(), 2);
    }

    #[test]
    fn header_size_is_what_was_written_not_guessed_from_the_size() {
        let parse = |bytes: &[u8]| BoxHeader::parse(&mut Cursor::new(bytes)).unwrap();
        // 8-byte form
        let h = parse(&bx(b"free", &[1, 2, 3]));
        assert_eq!((h.header_size, h.size, h.data_size()), (8, 11, 3));
        // 16-byte form with a size that would fit the 8-byte one
        let mut large = be(&[1]);
        large.extend_from_slice(b"free");
        large.extend(be64(&[19]));
        large.extend_from_slice(&[1, 2, 3]);
        let h = parse(&large);
        assert_eq!((h.header_size, h.data_offset, h.data_size()), (16, 16, 3));
        // size 0: to the end of the file, 8-byte header
        let mut to_end = be(&[0]);
        to_end.extend_from_slice(b"mdat");
        to_end.extend_from_slice(&[7; 5]);
        let h = parse(&to_end);
        assert_eq!((h.header_size, h.size, h.data_size()), (8, 13, 5));
    }

    #[test]
    fn large_size_box_below_4gib_is_skipped_correctly() {
        let mut f = be(&[1]);
        f.extend_from_slice(b"free");
        f.extend(be64(&[20]));
        f.extend_from_slice(&[9, 9, 9, 9]);
        f.extend(ftyp());
        assert_eq!(parse_mp4(&f).unwrap().brand.as_deref(), Some("isom"));
    }

    #[test]
    fn short_ftyp_followed_by_more_boxes_is_an_error_not_a_panic() {
        let mut f = be(&[12]);
        f.extend_from_slice(b"ftypisom");
        f.extend(bx(b"free", &[]));
        let e = parse_mp4(&f).unwrap_err().to_string();
        assert!(e.contains("ftyp"), "{e}");
    }
}
