//! ISO Base Media File Format (MP4) parser
//!
//! Implements minimal MP4 parsing to extract AV1 video samples.
//! No external dependencies - pure Rust implementation.
//!
//! References:
//! - ISO/IEC 14496-12 (ISO Base Media File Format)
//! - AV1 Codec ISO Media File Format Binding

use bitvue_core::BitvueError;
use crate::resource_budget::ResourceBudget;
use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom};

// ============================================================================
// Constants
// ============================================================================

/// Maximum entry count to prevent DoS via massive allocations
const MAX_ENTRY_COUNT: u32 = 10_000_000;

/// Maximum total samples to prevent memory exhaustion
const MAX_TOTAL_SAMPLES: usize = 100_000;

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
        })
    }

    /// Get box type as string
    pub fn box_type_str(&self) -> String {
        String::from_utf8_lossy(&self.box_type).to_string()
    }

    /// Get data size (excluding header)
    pub fn data_size(&self) -> u64 {
        self.size.saturating_sub(self.header_size())
    }

    /// Get header size
    fn header_size(&self) -> u64 {
        if self.size > u32::MAX as u64 {
            16 // size(4) + type(4) + largesize(8)
        } else {
            8 // size(4) + type(4)
        }
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
    /// Sample offsets
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

/// Parse MP4 file and extract AV1 samples
///
/// Returns zero-copy Cow slices that borrow from the input data when possible,
/// avoiding unnecessary memory allocation.
pub fn extract_av1_samples(data: &[u8]) -> Result<Vec<Cow<'_, [u8]>>, BitvueError> {
    let info = parse_mp4(data)?;

    // Verify this is an AV1 file
    match &info.codec {
        Some(codec) if codec == "av01" => {
            // AV1 codec confirmed
        }
        Some(codec) => {
            return Err(BitvueError::InvalidData(format!(
                "Not an AV1 file: found codec '{}'",
                codec
            )));
        }
        None => {
            return Err(BitvueError::InvalidData(
                "No codec information found in MP4".to_string(),
            ));
        }
    }

    // Validate sample count to prevent DoS
    if info.sample_offsets.len() > MAX_TOTAL_SAMPLES {
        return Err(BitvueError::InvalidData(format!(
            "Sample count {} exceeds maximum allowed {}",
            info.sample_offsets.len(),
            MAX_TOTAL_SAMPLES
        )));
    }

    // Pre-allocate with exact capacity since we know the sample count
    let mut samples = Vec::with_capacity(info.sample_offsets.len());

    // Sort samples by offset to detect overlaps
    let mut sorted_samples: Vec<_> = info.sample_offsets.iter()
        .zip(info.sample_sizes.iter())
        .enumerate()
        .collect();
    sorted_samples.sort_by_key(|(_, (offset, _))| *offset);

    for (i, (offset_ptr, size_ptr)) in sorted_samples.iter() {
        let offset = **offset_ptr as usize;
        let size = **size_ptr as usize;

        // Check for overflow in offset + size
        let end = match offset.checked_add(size) {
            Some(e) => e,
            None => {
                return Err(BitvueError::InvalidData(
                    "Sample offset + size would overflow".to_string()
                ));
            }
        };

        // Check against file size
        if end > data.len() {
            return Err(BitvueError::InvalidData(format!(
                "Sample at offset {} with size {} exceeds file size {}",
                offset, size, data.len()
            )));
        }

        // Check for overlap with next sample
        if i + 1 < sorted_samples.len() {
            let (_, (next_offset_ptr, _)) = sorted_samples[i + 1];
            let next_offset = *next_offset_ptr as usize;
            if end > next_offset {
                return Err(BitvueError::InvalidData(
                    format!("Samples overlap: current sample ends at {} but next starts at {}", end, next_offset)
                ));
            }
        }

        // Zero-copy: return borrowed slice instead of cloning
        samples.push(Cow::Borrowed(&data[offset..end]));
    }

    Ok(samples)
}

/// Parse MP4 file and extract H.264/AVC samples
///
/// Extracts NAL units from MP4 container for H.264/AVC video streams.
/// Supports both 'avc1' (AVC in MP4) and 'avc3' (AVC without parameter sets) codec types.
///
/// Returns zero-copy Cow slices that borrow from the input data when possible,
/// avoiding unnecessary memory allocation.
pub fn extract_avc_samples(data: &[u8]) -> Result<Vec<Cow<'_, [u8]>>, BitvueError> {
    let info = parse_mp4(data)?;

    // Verify this is an H.264/AVC file
    match &info.codec {
        Some(codec) if codec == "avc1" || codec == "avc3" => {
            // H.264/AVC codec confirmed
        }
        Some(codec) => {
            return Err(BitvueError::InvalidData(format!(
                "Not an H.264/AVC file: found codec '{}'",
                codec
            )));
        }
        None => {
            return Err(BitvueError::InvalidData(
                "No codec information found in MP4".to_string(),
            ));
        }
    }

    // Validate sample count to prevent DoS
    if info.sample_offsets.len() > MAX_TOTAL_SAMPLES {
        return Err(BitvueError::InvalidData(format!(
            "Sample count {} exceeds maximum allowed {}",
            info.sample_offsets.len(),
            MAX_TOTAL_SAMPLES
        )));
    }

    // Pre-allocate with exact capacity since we know the sample count
    let mut samples = Vec::with_capacity(info.sample_offsets.len());

    // Sort samples by offset to detect overlaps
    let mut sorted_samples: Vec<_> = info.sample_offsets.iter()
        .zip(info.sample_sizes.iter())
        .enumerate()
        .collect();
    sorted_samples.sort_by_key(|(_, (offset, _))| *offset);

    for (i, (offset_ptr, size_ptr)) in sorted_samples.iter() {
        let offset = **offset_ptr as usize;
        let size = **size_ptr as usize;

        // Check for overflow in offset + size
        let end = match offset.checked_add(size) {
            Some(e) => e,
            None => {
                return Err(BitvueError::InvalidData(
                    "Sample offset + size would overflow".to_string()
                ));
            }
        };

        // Check against file size
        if end > data.len() {
            return Err(BitvueError::InvalidData(format!(
                "Sample at offset {} with size {} exceeds file size {}",
                offset, size, data.len()
            )));
        }

        // Check for overlap with next sample
        if i + 1 < sorted_samples.len() {
            let (_, (next_offset_ptr, _)) = sorted_samples[i + 1];
            let next_offset = *next_offset_ptr as usize;
            if end > next_offset {
                return Err(BitvueError::InvalidData(
                    format!("Samples overlap: current sample ends at {} but next starts at {}", end, next_offset)
                ));
            }
        }

        // Zero-copy: return borrowed slice instead of cloning
        samples.push(Cow::Borrowed(&data[offset..end]));
    }

    Ok(samples)
}

/// Parse MP4 file and extract H.265/HEVC samples
///
/// Extracts NAL units from MP4 container for H.265/HEVC video streams.
/// Supports both 'hev1' (HEVC with parameter sets) and 'hvc1' (HEVC in-band parameter sets) codec types.
///
/// Returns zero-copy Cow slices that borrow from the input data when possible,
/// avoiding unnecessary memory allocation.
pub fn extract_hevc_samples(data: &[u8]) -> Result<Vec<Cow<'_, [u8]>>, BitvueError> {
    let info = parse_mp4(data)?;

    // Verify this is an H.265/HEVC file
    match &info.codec {
        Some(codec) if codec == "hev1" || codec == "hvc1" => {
            // H.265/HEVC codec confirmed
        }
        Some(codec) => {
            return Err(BitvueError::InvalidData(format!(
                "Not an H.265/HEVC file: found codec '{}'",
                codec
            )));
        }
        None => {
            return Err(BitvueError::InvalidData(
                "No codec information found in MP4".to_string(),
            ));
        }
    }

    // Validate sample count to prevent DoS
    if info.sample_offsets.len() > MAX_TOTAL_SAMPLES {
        return Err(BitvueError::InvalidData(format!(
            "Sample count {} exceeds maximum allowed {}",
            info.sample_offsets.len(),
            MAX_TOTAL_SAMPLES
        )));
    }

    // Pre-allocate with exact capacity since we know the sample count
    let mut samples = Vec::with_capacity(info.sample_offsets.len());

    // Sort samples by offset to detect overlaps
    let mut sorted_samples: Vec<_> = info.sample_offsets.iter()
        .zip(info.sample_sizes.iter())
        .enumerate()
        .collect();
    sorted_samples.sort_by_key(|(_, (offset, _))| *offset);

    for (i, (offset_ptr, size_ptr)) in sorted_samples.iter() {
        let offset = **offset_ptr as usize;
        let size = **size_ptr as usize;

        // Check for overflow in offset + size
        let end = match offset.checked_add(size) {
            Some(e) => e,
            None => {
                return Err(BitvueError::InvalidData(
                    "Sample offset + size would overflow".to_string()
                ));
            }
        };

        // Check against file size
        if end > data.len() {
            return Err(BitvueError::InvalidData(format!(
                "Sample at offset {} with size {} exceeds file size {}",
                offset, size, data.len()
            )));
        }

        // Check for overlap with next sample
        if i + 1 < sorted_samples.len() {
            let (_, (next_offset_ptr, _)) = sorted_samples[i + 1];
            let next_offset = *next_offset_ptr as usize;
            if end > next_offset {
                return Err(BitvueError::InvalidData(
                    format!("Samples overlap: current sample ends at {} but next starts at {}", end, next_offset)
                ));
            }
        }

        // Zero-copy: return borrowed slice instead of cloning
        samples.push(Cow::Borrowed(&data[offset..end]));
    }

    Ok(samples)
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
        let box_end = box_start + header.data_size();

        match &header.box_type {
            b"ftyp" => {
                // File type box
                parse_ftyp(&mut cursor, &header, &mut info)?;
            }
            b"moov" => {
                // Movie box - contains metadata
                parse_moov(&mut cursor, &header, &mut info, data)?;
            }
            b"mdat" => {
                // Media data box - skip for now
            }
            _ => {
                // Skip unknown box
            }
        }

        // Move to next box
        cursor.seek(SeekFrom::Start(box_end))?;
    }

    Ok(info)
}

/// Parse ftyp (File Type) box
fn parse_ftyp(
    cursor: &mut Cursor<&[u8]>,
    header: &BoxHeader,
    info: &mut Mp4Info,
) -> Result<(), BitvueError> {
    let brand = read_box_type(cursor)?;
    info.brand = Some(String::from_utf8_lossy(&brand).to_string());

    let _minor_version = read_u32(cursor)?;

    // Compatible brands
    let remaining = header.data_size() - 8;
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
) -> Result<(), BitvueError> {
    let box_end = header.data_offset + header.data_size();

    // Parse child boxes
    while cursor.position() < box_end {
        let child_header = BoxHeader::parse(cursor)?;
        let child_end = child_header.data_offset + child_header.data_size();

        match &child_header.box_type {
            b"trak" => {
                // Track box
                parse_trak(cursor, &child_header, info, data)?;
            }
            _ => {
                // Skip other boxes
            }
        }

        cursor.seek(SeekFrom::Start(child_end))?;
    }

    Ok(())
}

/// Parse trak (Track) box
fn parse_trak(
    cursor: &mut Cursor<&[u8]>,
    header: &BoxHeader,
    info: &mut Mp4Info,
    data: &[u8],
) -> Result<(), BitvueError> {
    let box_end = header.data_offset + header.data_size();

    while cursor.position() < box_end {
        let child_header = BoxHeader::parse(cursor)?;
        let child_end = child_header.data_offset + child_header.data_size();

        if &child_header.box_type == b"mdia" {
            // Media box
            parse_mdia(cursor, &child_header, info, data)?;
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
) -> Result<(), BitvueError> {
    let box_end = header.data_offset + header.data_size();

    while cursor.position() < box_end {
        let child_header = BoxHeader::parse(cursor)?;
        let child_end = child_header.data_offset + child_header.data_size();

        match &child_header.box_type {
            b"mdhd" => {
                // Media header - contains timescale
                parse_mdhd(cursor, &child_header, info)?;
            }
            b"minf" => {
                // Media information box
                parse_minf(cursor, &child_header, info, data)?;
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
) -> Result<(), BitvueError> {
    let box_end = header.data_offset + header.data_size();

    while cursor.position() < box_end {
        let child_header = BoxHeader::parse(cursor)?;
        let child_end = child_header.data_offset + child_header.data_size();

        if &child_header.box_type == b"stbl" {
            // Sample table box
            parse_stbl(cursor, &child_header, info)?;
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
) -> Result<(), BitvueError> {
    let box_end = header.data_offset + header.data_size();

    while cursor.position() < box_end {
        let child_header = BoxHeader::parse(cursor)?;
        let child_end = child_header.data_offset + child_header.data_size();

        match &child_header.box_type {
            b"stsd" => {
                // Sample description (codec information)
                parse_stsd(cursor, &child_header, info)?;
            }
            b"stts" => {
                // Sample time to sample (durations)
                parse_stts(cursor, &child_header, info)?;
            }
            b"stco" => {
                // Sample chunk offsets (32-bit)
                parse_stco(cursor, &child_header, info)?;
            }
            b"co64" => {
                // Sample chunk offsets (64-bit)
                parse_co64(cursor, &child_header, info)?;
            }
            b"stsz" => {
                // Sample sizes
                parse_stsz(cursor, &child_header, info)?;
            }
            b"ctts" => {
                // Composition time to sample (for PTS)
                parse_ctts(cursor, &child_header, info)?;
            }
            b"stss" => {
                // Sync sample table (key frames)
                parse_stss(cursor, &child_header, info)?;
            }
            _ => {}
        }

        cursor.seek(SeekFrom::Start(child_end))?;
    }

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
        info.sample_offsets.push(offset);
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
        info.sample_offsets.push(offset);
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

    for _ in 0..entry_count {
        let sample_count = read_u32(cursor)?;
        let sample_delta = read_u32(cursor)?;

        // Check if total would exceed maximum
        total_samples = total_samples.saturating_add(sample_count as usize);
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
