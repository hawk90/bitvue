//! Index Extractor API - T1-1 Indexing Core
//!
//! Deliverable: extract_api:Indexing:Core:AV1:viz_core
//!
//! Codec-agnostic API for extracting frame metadata from bitstreams.
//! Per INDEXING_STRATEGY_SPEC: Quick Index Strategy (fast open)
//!
//! Design:
//! - Trait-based: IndexExtractor trait for codec-specific logic
//! - Quick extraction: Minimal scan for keyframes/OBU boundaries
//! - Stubs: Unsupported codecs return Err with codec name
//! - Progress: Reports progress for UI feedback

use crate::indexing::{FrameMetadata, QuickIndex, SeekPoint};
use crate::BitvueError;
use std::io::{Read, Seek};

// Blanket implementation for all types that implement both Read and Seek
impl<T: Read + Seek> ReadSeek for T {}

/// Index extraction result
pub type ExtractResult<T> = Result<T, BitvueError>;

/// Progress callback: receives progress (0.0-1.0) and status message
pub type ProgressCallback<'a> = Option<&'a dyn Fn(f64, &str)>;

/// Cancellation callback: returns true to request cancellation
pub type CancelCallback<'a> = Option<&'a dyn Fn() -> bool>;

/// Index extractor trait for codec-specific extraction
///
/// Per INDEXING_STRATEGY_SPEC.md:
/// "Trait/API + codec adapters; include stubs for unsupported."
pub trait IndexExtractor {
    /// Get codec name
    fn codec_name(&self) -> &'static str;

    /// Extract quick index from stream
    ///
    /// Quick index: minimal scan for keyframes + OBU boundaries
    /// - Scans file header and first few frames
    /// - Locates all keyframes via minimal parsing
    /// - Returns seek points for fast startup
    ///
    /// Per INDEXING_STRATEGY_SPEC.md Phase 1:
    /// "Scan minimal headers to locate keyframes/OBU boundaries.
    ///  Enables first frame display ASAP."
    fn extract_quick_index(&self, reader: &mut dyn ReadSeek) -> ExtractResult<QuickIndex>;

    /// Extract full frame metadata
    ///
    /// Full extraction: complete frame-by-frame scan
    /// - Parses all frame headers
    /// - Builds complete display_idx → byte_offset map
    /// - Can be cancelled via should_cancel callback
    ///
    /// Per INDEXING_STRATEGY_SPEC.md Phase 2:
    /// "Background task with progress indicator.
    ///  Builds full frame → offset map."
    fn extract_full_index(
        &self,
        reader: &mut dyn ReadSeek,
        progress_callback: ProgressCallback<'_>,
        should_cancel: CancelCallback<'_>,
    ) -> ExtractResult<Vec<FrameMetadata>>;

    /// Check if codec is supported
    fn is_supported(&self) -> bool {
        true
    }
}

/// Trait combining Read + Seek for object safety
pub trait ReadSeek: Read + Seek {}

/// AV1 index extractor
pub struct Av1IndexExtractor;

impl Av1IndexExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Parse OBU headers to find frame boundaries
    ///
    /// Minimal parsing: only what's needed for indexing
    /// - OBU type
    /// - OBU size
    /// - Frame type (for keyframe detection)
    fn scan_obu_headers(
        &self,
        reader: &mut dyn ReadSeek,
        file_size: u64,
        quick_mode: bool,
    ) -> ExtractResult<Vec<SeekPoint>> {
        let mut seek_points = Vec::new();
        let mut display_idx = 0;
        let mut _last_keyframe_offset: Option<u64> = None;

        loop {
            let offset = reader.stream_position()?;

            if offset >= file_size {
                break;
            }

            // Read OBU header (1 byte minimum)
            let mut obu_header = [0u8; 1];
            match reader.read_exact(&mut obu_header) {
                Ok(_) => {}
                Err(_) => break, // EOF
            }

            let obu_type = (obu_header[0] >> 3) & 0x0F;
            let has_size_field = (obu_header[0] & 0x02) != 0;

            // Read OBU size if present
            let obu_size = if has_size_field {
                self.read_leb128(reader)?
            } else {
                // No size field - would need to parse to end
                // For quick mode, skip this frame
                if quick_mode {
                    break;
                }
                0
            };

            // OBU types per AV1 spec
            const OBU_SEQUENCE_HEADER: u8 = 1;
            const OBU_FRAME_HEADER: u8 = 3;
            const OBU_FRAME: u8 = 6;

            // Check if this is a keyframe OBU
            let is_keyframe_obu = matches!(obu_type, OBU_SEQUENCE_HEADER | OBU_FRAME);

            // For quick mode, only extract keyframes
            if is_keyframe_obu {
                let sp = SeekPoint {
                    display_idx,
                    byte_offset: offset,
                    is_keyframe: true,
                    pts: None, // PTS extraction requires container parsing
                };

                seek_points.push(sp);
                _last_keyframe_offset = Some(offset);
                display_idx += 1;

                // In quick mode, stop after finding a few keyframes
                if quick_mode && seek_points.len() >= 5 {
                    break;
                }
            } else if obu_type == OBU_FRAME_HEADER {
                // Frame header OBU - might be for an inter frame
                // Only include in full scan
                if !quick_mode {
                    // Would need to parse frame header to determine if keyframe
                    // For now, assume non-keyframe
                    display_idx += 1;
                }
            }

            // Skip to next OBU
            if obu_size > 0 {
                reader.seek(std::io::SeekFrom::Current(obu_size as i64))?;
            } else {
                // No size field - estimate skip
                break;
            }
        }

        // Ensure we have at least one keyframe
        if seek_points.is_empty() {
            return Err(BitvueError::Parse {
                offset: 0,
                message: "No keyframes found in stream".to_string(),
            });
        }

        Ok(seek_points)
    }

    /// Read LEB128 variable-length integer
    fn read_leb128(&self, reader: &mut dyn Read) -> ExtractResult<u64> {
        let mut result = 0u64;
        let mut shift = 0;

        loop {
            let mut byte = [0u8];
            reader.read_exact(&mut byte)?;

            result |= ((byte[0] & 0x7F) as u64) << shift;

            if (byte[0] & 0x80) == 0 {
                break;
            }

            shift += 7;
            if shift > 56 {
                return Err(BitvueError::Parse {
                    offset: 0,
                    message: "LEB128 overflow".to_string(),
                });
            }
        }

        Ok(result)
    }
}

impl Default for Av1IndexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexExtractor for Av1IndexExtractor {
    fn codec_name(&self) -> &'static str {
        "AV1"
    }

    fn extract_quick_index(&self, reader: &mut dyn ReadSeek) -> ExtractResult<QuickIndex> {
        // Get file size
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;

        // Rewind to start
        reader.seek(std::io::SeekFrom::Start(0))?;

        // Scan for keyframes (quick mode)
        let seek_points = self.scan_obu_headers(reader, file_size, true)?;

        // Estimate frame count based on keyframes
        let estimated_frame_count = if seek_points.len() >= 2 {
            // Estimate based on keyframe interval
            // This is a rough estimate
            let avg_interval = file_size / seek_points.len() as u64;
            Some((file_size / avg_interval.max(1)) as usize)
        } else {
            None
        };

        let mut quick_index = QuickIndex::new(seek_points, file_size);
        quick_index.estimated_frame_count = estimated_frame_count;

        Ok(quick_index)
    }

    fn extract_full_index(
        &self,
        reader: &mut dyn ReadSeek,
        progress_callback: ProgressCallback<'_>,
        should_cancel: CancelCallback<'_>,
    ) -> ExtractResult<Vec<FrameMetadata>> {
        // Get file size
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;

        // Rewind to start
        reader.seek(std::io::SeekFrom::Start(0))?;

        let mut frames = Vec::new();
        let mut display_idx = 0;
        let mut decode_idx = 0;

        loop {
            // Check cancellation
            if let Some(cancel_fn) = should_cancel {
                if cancel_fn() {
                    return Err(BitvueError::InvalidData(
                        "Indexing cancelled by user".to_string(),
                    ));
                }
            }

            let offset = reader.stream_position()?;

            if offset >= file_size {
                break;
            }

            // Report progress
            if let Some(callback) = progress_callback {
                let progress = offset as f64 / file_size as f64;
                callback(progress, &format!("Scanning frame {}", display_idx));
            }

            // Read OBU header
            let mut obu_header = [0u8; 1];
            match reader.read_exact(&mut obu_header) {
                Ok(_) => {}
                Err(_) => break, // EOF
            }

            let obu_type = (obu_header[0] >> 3) & 0x0F;
            let has_size_field = (obu_header[0] & 0x02) != 0;

            // Read OBU size
            let obu_size = if has_size_field {
                self.read_leb128(reader)?
            } else {
                // No size field - cannot reliably parse
                break;
            };

            // OBU types
            const OBU_SEQUENCE_HEADER: u8 = 1;
            const OBU_FRAME_HEADER: u8 = 3;
            const OBU_FRAME: u8 = 6;

            // Determine if keyframe
            let is_keyframe = matches!(obu_type, OBU_SEQUENCE_HEADER | OBU_FRAME);

            // Create frame metadata
            if matches!(obu_type, OBU_FRAME_HEADER | OBU_FRAME) || is_keyframe {
                let frame = FrameMetadata {
                    display_idx,
                    decode_idx,
                    byte_offset: offset,
                    size: obu_size + 1, // +1 for header byte
                    is_keyframe,
                    pts: None, // Would need container parsing
                    dts: None,
                    frame_type: Some(if is_keyframe {
                        "I".to_string()
                    } else {
                        "P".to_string()
                    }),
                };

                frames.push(frame);
                display_idx += 1;
                decode_idx += 1;
            }

            // Skip to next OBU
            reader.seek(std::io::SeekFrom::Current(obu_size as i64))?;
        }

        // Report completion
        if let Some(callback) = progress_callback {
            callback(1.0, &format!("Indexed {} frames", frames.len()));
        }

        Ok(frames)
    }

    fn is_supported(&self) -> bool {
        true
    }
}

/// H.264 index extractor
///
/// Deliverable: extract_api:Indexing:Core:H264:viz_core
pub struct H264IndexExtractor;

impl H264IndexExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Scan for NAL units in H.264 Annex B byte stream
    ///
    /// Looks for start codes: 0x000001 or 0x00000001
    /// Parses NAL unit headers to identify frame types
    fn scan_nal_units(
        &self,
        reader: &mut dyn ReadSeek,
        file_size: u64,
        quick_mode: bool,
    ) -> ExtractResult<Vec<SeekPoint>> {
        let mut seek_points = Vec::new();
        let mut display_idx = 0;
        let mut buffer = [0u8; 4];

        loop {
            let offset = reader.stream_position()?;

            if offset >= file_size {
                break;
            }

            // Read 4 bytes for start code detection
            match reader.read_exact(&mut buffer) {
                Ok(_) => {}
                Err(_) => break, // EOF
            }

            // Check for start codes: 0x00000001 or 0x000001
            let (_start_code_len, has_start_code) = if buffer == [0x00, 0x00, 0x00, 0x01] {
                (4, true)
            } else if buffer[0..3] == [0x00, 0x00, 0x01] {
                // Rewind 1 byte since we read 4
                reader.seek(std::io::SeekFrom::Current(-1))?;
                (3, true)
            } else {
                // Not a start code, move forward 1 byte and try again
                reader.seek(std::io::SeekFrom::Start(offset + 1))?;
                (0, false)
            };

            if !has_start_code {
                continue;
            }

            // Read NAL unit header
            let mut nal_header = [0u8; 1];
            if reader.read_exact(&mut nal_header).is_err() {
                break;
            }

            let nal_unit_type = nal_header[0] & 0x1F;

            // NAL unit types:
            // 1 = Non-IDR slice
            // 5 = IDR slice (keyframe)
            // 7 = SPS
            // 8 = PPS
            let is_keyframe = nal_unit_type == 5;
            let is_frame = matches!(nal_unit_type, 1 | 5);

            if is_frame {
                // In quick mode, only collect keyframes
                if quick_mode && !is_keyframe {
                    // Skip to next NAL unit (simplified - would need full parsing)
                    continue;
                }

                seek_points.push(SeekPoint {
                    display_idx,
                    byte_offset: offset,
                    is_keyframe,
                    pts: None,
                });

                display_idx += 1;

                // In quick mode, limit collection
                if quick_mode && seek_points.len() >= 100 {
                    break;
                }
            }

            // Note: Proper implementation would parse NAL unit size
            // For now, continue scanning for next start code
        }

        if seek_points.is_empty() {
            return Err(BitvueError::Parse {
                offset: 0,
                message: "No frames found in H.264 stream".to_string(),
            });
        }

        Ok(seek_points)
    }

    /// Find next NAL unit start code from current position
    fn find_next_start_code(
        &self,
        reader: &mut dyn ReadSeek,
        file_size: u64,
    ) -> ExtractResult<Option<u64>> {
        let mut buffer = [0u8; 4];
        let start_offset = reader.stream_position()?;

        for search_offset in start_offset..file_size {
            reader.seek(std::io::SeekFrom::Start(search_offset))?;

            if reader.read_exact(&mut buffer).is_err() {
                return Ok(None);
            }

            // Check for 4-byte start code
            if buffer == [0x00, 0x00, 0x00, 0x01] {
                return Ok(Some(search_offset));
            }

            // Check for 3-byte start code
            if buffer[0..3] == [0x00, 0x00, 0x01] {
                return Ok(Some(search_offset));
            }
        }

        Ok(None)
    }
}

impl Default for H264IndexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexExtractor for H264IndexExtractor {
    fn codec_name(&self) -> &'static str {
        "H.264"
    }

    fn extract_quick_index(&self, reader: &mut dyn ReadSeek) -> ExtractResult<QuickIndex> {
        // Get file size
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;

        // Rewind to start
        reader.seek(std::io::SeekFrom::Start(0))?;

        // Scan for keyframes (quick mode)
        let seek_points = self.scan_nal_units(reader, file_size, true)?;

        // Estimate frame count based on keyframe interval
        let estimated_frame_count = if seek_points.len() >= 2 {
            // Assume keyframe every 30-60 frames (typical GOP size)
            let avg_gop_size = 45;
            Some(seek_points.len() * avg_gop_size)
        } else {
            None
        };

        let mut quick_index = QuickIndex::new(seek_points, file_size);
        quick_index.estimated_frame_count = estimated_frame_count;

        Ok(quick_index)
    }

    fn extract_full_index(
        &self,
        reader: &mut dyn ReadSeek,
        progress_callback: ProgressCallback<'_>,
        should_cancel: CancelCallback<'_>,
    ) -> ExtractResult<Vec<FrameMetadata>> {
        // Get file size
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;

        // Rewind to start
        reader.seek(std::io::SeekFrom::Start(0))?;

        let mut frames = Vec::new();
        let mut display_idx = 0;
        let mut decode_idx = 0;
        let mut buffer = [0u8; 4];

        loop {
            // Check cancellation
            if let Some(cancel_fn) = should_cancel {
                if cancel_fn() {
                    return Err(BitvueError::InvalidData(
                        "Indexing cancelled by user".to_string(),
                    ));
                }
            }

            let offset = reader.stream_position()?;

            if offset >= file_size {
                break;
            }

            // Report progress
            if let Some(callback) = progress_callback {
                let progress = offset as f64 / file_size as f64;
                callback(progress, &format!("Scanning frame {}", display_idx));
            }

            // Read 4 bytes for start code detection
            match reader.read_exact(&mut buffer) {
                Ok(_) => {}
                Err(_) => break, // EOF
            }

            // Check for start codes
            let (_start_code_len, has_start_code) = if buffer == [0x00, 0x00, 0x00, 0x01] {
                (4, true)
            } else if buffer[0..3] == [0x00, 0x00, 0x01] {
                reader.seek(std::io::SeekFrom::Current(-1))?;
                (3, true)
            } else {
                reader.seek(std::io::SeekFrom::Start(offset + 1))?;
                (0, false)
            };

            if !has_start_code {
                continue;
            }

            // Read NAL unit header
            let mut nal_header = [0u8; 1];
            if reader.read_exact(&mut nal_header).is_err() {
                break;
            }

            let nal_unit_type = nal_header[0] & 0x1F;
            let is_keyframe = nal_unit_type == 5;
            let is_frame = matches!(nal_unit_type, 1 | 5);

            if is_frame {
                // Find next start code to determine NAL unit size
                let nal_start = offset;
                let next_start = self.find_next_start_code(reader, file_size)?;

                let size = if let Some(next_offset) = next_start {
                    next_offset - nal_start
                } else {
                    file_size - nal_start
                };

                let frame = FrameMetadata {
                    display_idx,
                    decode_idx,
                    byte_offset: nal_start,
                    size,
                    is_keyframe,
                    pts: None, // Would need container parsing
                    dts: None,
                    frame_type: Some(if is_keyframe {
                        "I".to_string()
                    } else {
                        "P".to_string()
                    }),
                };

                frames.push(frame);
                display_idx += 1;
                decode_idx += 1;

                // Seek to next start code position
                if let Some(next_offset) = next_start {
                    reader.seek(std::io::SeekFrom::Start(next_offset))?;
                }
            }
        }

        // Report completion
        if let Some(callback) = progress_callback {
            callback(1.0, &format!("Indexed {} frames", frames.len()));
        }

        if frames.is_empty() {
            return Err(BitvueError::Parse {
                offset: 0,
                message: "No frames found in H.264 stream".to_string(),
            });
        }

        Ok(frames)
    }

    fn is_supported(&self) -> bool {
        true
    }
}

/// HEVC/H.265 index extractor
///
/// Uses bitvue-hevc for NAL unit parsing
///
/// TODO: Re-enable when cyclic dependency is resolved
/// Currently disabled to avoid circular dependency between bitvue-core and bitvue-hevc
#[cfg(feature = "hevc-indexer")]
pub struct HevcIndexExtractor;

#[cfg(feature = "hevc-indexer")]
impl HevcIndexExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Scan NAL units for keyframes
    fn scan_nal_units(
        &self,
        reader: &mut dyn ReadSeek,
        #[allow(unused_variables)] file_size: u64,
        quick_mode: bool,
    ) -> ExtractResult<Vec<SeekPoint>> {
        // Read entire file for parsing
        reader.seek(std::io::SeekFrom::Start(0))?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        // Parse HEVC stream
        let stream = bitvue_hevc::parse_hevc(&data).map_err(|e| BitvueError::Parse {
            offset: 0,
            message: format!("HEVC parse error: {}", e),
        })?;

        let mut seek_points = Vec::new();
        let mut display_idx = 0;

        for nal in &stream.nal_units {
            // Check for VCL NAL units (slice data)
            if nal.is_vcl() {
                let is_keyframe = nal.is_idr() || nal.is_cra() || nal.is_bla();

                // In quick mode, only collect keyframes
                if quick_mode && !is_keyframe {
                    continue;
                }

                seek_points.push(SeekPoint {
                    display_idx,
                    byte_offset: nal.offset,
                    is_keyframe,
                    pts: None,
                });

                display_idx += 1;

                // In quick mode, limit collection
                if quick_mode && seek_points.len() >= 100 {
                    break;
                }
            }
        }

        if seek_points.is_empty() {
            return Err(BitvueError::Parse {
                offset: 0,
                message: "No frames found in HEVC stream".to_string(),
            });
        }

        Ok(seek_points)
    }
}

#[cfg(feature = "hevc-indexer")]
impl Default for HevcIndexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "hevc-indexer")]
impl IndexExtractor for HevcIndexExtractor {
    fn codec_name(&self) -> &'static str {
        "HEVC"
    }

    fn extract_quick_index(&self, reader: &mut dyn ReadSeek) -> ExtractResult<QuickIndex> {
        #[allow(unused_variables)]
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;
        reader.seek(std::io::SeekFrom::Start(0))?;

        let seek_points = self.scan_nal_units(reader, file_size, true)?;

        let estimated_frame_count = if seek_points.len() >= 2 {
            let avg_gop_size = 30;
            Some(seek_points.len() * avg_gop_size)
        } else {
            None
        };

        let mut quick_index = QuickIndex::new(seek_points, file_size);
        quick_index.estimated_frame_count = estimated_frame_count;

        Ok(quick_index)
    }

    fn extract_full_index(
        &self,
        reader: &mut dyn ReadSeek,
        progress_callback: ProgressCallback<'_>,
        should_cancel: CancelCallback<'_>,
    ) -> ExtractResult<Vec<FrameMetadata>> {
        // Check cancellation
        if let Some(cancel_fn) = should_cancel {
            if cancel_fn() {
                return Err(BitvueError::InvalidData(
                    "Indexing cancelled by user".to_string(),
                ));
            }
        }

        #[allow(unused_variables)]
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;
        reader.seek(std::io::SeekFrom::Start(0))?;

        if let Some(callback) = progress_callback {
            callback(0.1, "Reading HEVC stream...");
        }

        // Read entire file for parsing
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        if let Some(callback) = progress_callback {
            callback(0.3, "Parsing NAL units...");
        }

        // Parse HEVC stream
        let stream = bitvue_hevc::parse_hevc(&data).map_err(|e| BitvueError::Parse {
            offset: 0,
            message: format!("HEVC parse error: {}", e),
        })?;

        let mut frames = Vec::new();
        let mut display_idx = 0;
        let mut decode_idx = 0;

        for nal in &stream.nal_units {
            if nal.is_vcl() {
                let is_keyframe = nal.is_idr() || nal.is_cra() || nal.is_bla();

                let frame_type = if nal.is_idr() {
                    "IDR"
                } else if nal.is_cra() {
                    "CRA"
                } else if nal.is_bla() {
                    "BLA"
                } else {
                    match nal.header.nal_unit_type {
                        bitvue_hevc::NalUnitType::TrailR | bitvue_hevc::NalUnitType::TrailN => "P",
                        _ => "B",
                    }
                };

                frames.push(FrameMetadata {
                    display_idx,
                    decode_idx,
                    byte_offset: nal.offset,
                    size: nal.size,
                    is_keyframe,
                    pts: None,
                    dts: None,
                    frame_type: Some(frame_type.to_string()),
                });

                display_idx += 1;
                decode_idx += 1;
            }
        }

        if let Some(callback) = progress_callback {
            callback(1.0, &format!("Indexed {} frames", frames.len()));
        }

        if frames.is_empty() {
            return Err(BitvueError::Parse {
                offset: 0,
                message: "No frames found in HEVC stream".to_string(),
            });
        }

        Ok(frames)
    }

    fn is_supported(&self) -> bool {
        true
    }
}

/// VP9 index extractor
///
/// Uses bitvue-vp9 for frame parsing
#[cfg(feature = "vp9-indexer")]
pub struct Vp9IndexExtractor;

#[cfg(feature = "vp9-indexer")]
impl Vp9IndexExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Scan VP9 frames for keyframes
    fn scan_frames(
        &self,
        reader: &mut dyn ReadSeek,
        #[allow(unused_variables)] file_size: u64,
        quick_mode: bool,
    ) -> ExtractResult<Vec<SeekPoint>> {
        // Read entire file for parsing
        reader.seek(std::io::SeekFrom::Start(0))?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        // Parse VP9 stream
        let stream = bitvue_vp9::parse_vp9(&data).map_err(|e| BitvueError::Parse {
            offset: 0,
            message: format!("VP9 parse error: {}", e),
        })?;

        let mut seek_points = Vec::new();
        let mut display_idx = 0;

        for (i, frame) in stream.frames.iter().enumerate() {
            let is_keyframe = frame.frame_type == bitvue_vp9::FrameType::KeyFrame;

            // In quick mode, only collect keyframes
            if quick_mode && !is_keyframe {
                continue;
            }

            let byte_offset = stream.superframe_index.frame_offsets.get(i).copied().unwrap_or(0) as u64;

            seek_points.push(SeekPoint {
                display_idx,
                byte_offset,
                is_keyframe,
                pts: None,
            });

            display_idx += 1;

            // In quick mode, limit collection
            if quick_mode && seek_points.len() >= 100 {
                break;
            }
        }

        if seek_points.is_empty() {
            return Err(BitvueError::Parse {
                offset: 0,
                message: "No frames found in VP9 stream".to_string(),
            });
        }

        Ok(seek_points)
    }
}

#[cfg(feature = "vp9-indexer")]
impl Default for Vp9IndexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "vp9-indexer")]
impl IndexExtractor for Vp9IndexExtractor {
    fn codec_name(&self) -> &'static str {
        "VP9"
    }

    fn extract_quick_index(&self, reader: &mut dyn ReadSeek) -> ExtractResult<QuickIndex> {
        #[allow(unused_variables)]
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;
        reader.seek(std::io::SeekFrom::Start(0))?;

        let seek_points = self.scan_frames(reader, file_size, true)?;

        let estimated_frame_count = if seek_points.len() >= 2 {
            let avg_gop_size = 30;
            Some(seek_points.len() * avg_gop_size)
        } else {
            None
        };

        let mut quick_index = QuickIndex::new(seek_points, file_size);
        quick_index.estimated_frame_count = estimated_frame_count;

        Ok(quick_index)
    }

    fn extract_full_index(
        &self,
        reader: &mut dyn ReadSeek,
        progress_callback: ProgressCallback<'_>,
        should_cancel: CancelCallback<'_>,
    ) -> ExtractResult<Vec<FrameMetadata>> {
        // Check cancellation
        if let Some(cancel_fn) = should_cancel {
            if cancel_fn() {
                return Err(BitvueError::InvalidData(
                    "Indexing cancelled by user".to_string(),
                ));
            }
        }

        #[allow(unused_variables)]
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;
        reader.seek(std::io::SeekFrom::Start(0))?;

        if let Some(callback) = progress_callback {
            callback(0.1, "Reading VP9 stream...");
        }

        // Read entire file for parsing
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        if let Some(callback) = progress_callback {
            callback(0.3, "Parsing VP9 frames...");
        }

        // Parse VP9 stream
        let stream = bitvue_vp9::parse_vp9(&data).map_err(|e| BitvueError::Parse {
            offset: 0,
            message: format!("VP9 parse error: {}", e),
        })?;

        let mut frames = Vec::new();
        let mut display_idx = 0;
        let mut decode_idx = 0;

        for (i, frame) in stream.frames.iter().enumerate() {
            let is_keyframe = frame.frame_type == bitvue_vp9::FrameType::KeyFrame;

            let frame_type = if is_keyframe { "I" } else { "P" };

            let byte_offset = stream.superframe_index.frame_offsets.get(i).copied().unwrap_or(0) as u64;
            let size = stream.superframe_index.frame_sizes.get(i).copied().unwrap_or(0) as u64;

            frames.push(FrameMetadata {
                display_idx,
                decode_idx,
                byte_offset,
                size,
                is_keyframe,
                pts: None,
                dts: None,
                frame_type: Some(frame_type.to_string()),
            });

            display_idx += 1;
            decode_idx += 1;
        }

        if let Some(callback) = progress_callback {
            callback(1.0, &format!("Indexed {} frames", frames.len()));
        }

        if frames.is_empty() {
            return Err(BitvueError::Parse {
                offset: 0,
                message: "No frames found in VP9 stream".to_string(),
            });
        }

        Ok(frames)
    }

    fn is_supported(&self) -> bool {
        true
    }
}

/// VVC/H.266 index extractor
///
/// Uses bitvue-vvc for NAL unit parsing
#[cfg(feature = "vvc-indexer")]
pub struct VvcIndexExtractor;

#[cfg(feature = "vvc-indexer")]
impl VvcIndexExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Scan NAL units for keyframes
    fn scan_nal_units(
        &self,
        reader: &mut dyn ReadSeek,
        #[allow(unused_variables)] file_size: u64,
        quick_mode: bool,
    ) -> ExtractResult<Vec<SeekPoint>> {
        // Read entire file for parsing
        reader.seek(std::io::SeekFrom::Start(0))?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        // Parse VVC stream
        let stream = bitvue_vvc::parse_vvc(&data).map_err(|e| BitvueError::Parse {
            offset: 0,
            message: format!("VVC parse error: {}", e),
        })?;

        let mut seek_points = Vec::new();
        let mut display_idx = 0;

        for nal in &stream.nal_units {
            // Check for VCL NAL units
            if nal.is_vcl() {
                let is_keyframe = nal.header.nal_unit_type.is_idr()
                    || nal.header.nal_unit_type.is_cra()
                    || nal.header.nal_unit_type.is_gdr();

                // In quick mode, only collect keyframes
                if quick_mode && !is_keyframe {
                    continue;
                }

                seek_points.push(SeekPoint {
                    display_idx,
                    byte_offset: nal.offset,
                    is_keyframe,
                    pts: None,
                });

                display_idx += 1;

                // In quick mode, limit collection
                if quick_mode && seek_points.len() >= 100 {
                    break;
                }
            }
        }

        if seek_points.is_empty() {
            return Err(BitvueError::Parse {
                offset: 0,
                message: "No frames found in VVC stream".to_string(),
            });
        }

        Ok(seek_points)
    }
}

#[cfg(feature = "vvc-indexer")]
impl Default for VvcIndexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "vvc-indexer")]
impl IndexExtractor for VvcIndexExtractor {
    fn codec_name(&self) -> &'static str {
        "VVC"
    }

    fn extract_quick_index(&self, reader: &mut dyn ReadSeek) -> ExtractResult<QuickIndex> {
        #[allow(unused_variables)]
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;
        reader.seek(std::io::SeekFrom::Start(0))?;

        let seek_points = self.scan_nal_units(reader, file_size, true)?;

        let estimated_frame_count = if seek_points.len() >= 2 {
            let avg_gop_size = 30;
            Some(seek_points.len() * avg_gop_size)
        } else {
            None
        };

        let mut quick_index = QuickIndex::new(seek_points, file_size);
        quick_index.estimated_frame_count = estimated_frame_count;

        Ok(quick_index)
    }

    fn extract_full_index(
        &self,
        reader: &mut dyn ReadSeek,
        progress_callback: ProgressCallback<'_>,
        should_cancel: CancelCallback<'_>,
    ) -> ExtractResult<Vec<FrameMetadata>> {
        // Check cancellation
        if let Some(cancel_fn) = should_cancel {
            if cancel_fn() {
                return Err(BitvueError::InvalidData(
                    "Indexing cancelled by user".to_string(),
                ));
            }
        }

        #[allow(unused_variables)]
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;
        reader.seek(std::io::SeekFrom::Start(0))?;

        if let Some(callback) = progress_callback {
            callback(0.1, "Reading VVC stream...");
        }

        // Read entire file for parsing
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        if let Some(callback) = progress_callback {
            callback(0.3, "Parsing NAL units...");
        }

        // Parse VVC stream
        let stream = bitvue_vvc::parse_vvc(&data).map_err(|e| BitvueError::Parse {
            offset: 0,
            message: format!("VVC parse error: {}", e),
        })?;

        let mut frames = Vec::new();
        let mut display_idx = 0;
        let mut decode_idx = 0;

        for nal in &stream.nal_units {
            if nal.is_vcl() {
                let nal_type = &nal.header.nal_unit_type;
                let is_keyframe = nal_type.is_idr() || nal_type.is_cra() || nal_type.is_gdr();

                let frame_type = if nal_type.is_idr() {
                    "IDR"
                } else if nal_type.is_cra() {
                    "CRA"
                } else if nal_type.is_gdr() {
                    "GDR"
                } else {
                    "P"
                };

                frames.push(FrameMetadata {
                    display_idx,
                    decode_idx,
                    byte_offset: nal.offset,
                    size: nal.size,
                    is_keyframe,
                    pts: None,
                    dts: None,
                    frame_type: Some(frame_type.to_string()),
                });

                display_idx += 1;
                decode_idx += 1;
            }
        }

        if let Some(callback) = progress_callback {
            callback(1.0, &format!("Indexed {} frames", frames.len()));
        }

        if frames.is_empty() {
            return Err(BitvueError::Parse {
                offset: 0,
                message: "No frames found in VVC stream".to_string(),
            });
        }

        Ok(frames)
    }

    fn is_supported(&self) -> bool {
        true
    }
}

/// Factory for creating codec-specific extractors
pub struct ExtractorFactory;

impl ExtractorFactory {
    /// Create extractor for codec
    pub fn create(codec: &str) -> Box<dyn IndexExtractor> {
        match codec.to_lowercase().as_str() {
            "av1" => Box::new(Av1IndexExtractor::new()),
            "h264" | "h.264" | "avc" => Box::new(H264IndexExtractor::new()),
            #[cfg(feature = "hevc-indexer")]
            "hevc" | "h265" | "h.265" => Box::new(HevcIndexExtractor::new()),
            #[cfg(feature = "vvc-indexer")]
            "vvc" | "h266" | "h.266" => Box::new(VvcIndexExtractor::new()),
            #[cfg(feature = "vp9-indexer")]
            "vp9" => Box::new(Vp9IndexExtractor::new()),
            #[cfg(not(feature = "hevc-indexer"))]
            "hevc" | "h265" | "h.265" => Box::new(UnsupportedExtractor {
                codec: codec.to_string(),
            }),
            #[cfg(not(feature = "vvc-indexer"))]
            "vvc" | "h266" | "h.266" => Box::new(UnsupportedExtractor {
                codec: codec.to_string(),
            }),
            #[cfg(not(feature = "vp9-indexer"))]
            "vp9" => Box::new(UnsupportedExtractor {
                codec: codec.to_string(),
            }),
            _ => Box::new(UnsupportedExtractor {
                codec: codec.to_string(),
            }),
        }
    }

    /// Detect codec from file extension
    pub fn from_extension(ext: &str) -> Box<dyn IndexExtractor> {
        match ext.to_lowercase().as_str() {
            "ivf" | "av1" | "obu" => Box::new(Av1IndexExtractor::new()),
            "264" | "h264" => Box::new(H264IndexExtractor::new()),
            #[cfg(feature = "hevc-indexer")]
            "265" | "h265" | "hevc" => Box::new(HevcIndexExtractor::new()),
            #[cfg(feature = "vvc-indexer")]
            "266" | "h266" | "vvc" => Box::new(VvcIndexExtractor::new()),
            #[cfg(feature = "vp9-indexer")]
            "vp9" => Box::new(Vp9IndexExtractor::new()),
            #[cfg(not(feature = "hevc-indexer"))]
            "265" | "h265" | "hevc" => Box::new(UnsupportedExtractor {
                codec: ext.to_string(),
            }),
            #[cfg(not(feature = "vvc-indexer"))]
            "266" | "h266" | "vvc" => Box::new(UnsupportedExtractor {
                codec: ext.to_string(),
            }),
            #[cfg(not(feature = "vp9-indexer"))]
            "vp9" => Box::new(UnsupportedExtractor {
                codec: ext.to_string(),
            }),
            _ => Box::new(UnsupportedExtractor {
                codec: ext.to_string(),
            }),
        }
    }
}

/// Unsupported codec extractor
struct UnsupportedExtractor {
    codec: String,
}

impl IndexExtractor for UnsupportedExtractor {
    fn codec_name(&self) -> &'static str {
        "Unsupported"
    }

    fn extract_quick_index(&self, _reader: &mut dyn ReadSeek) -> ExtractResult<QuickIndex> {
        Err(BitvueError::UnsupportedCodec(self.codec.clone()))
    }

    fn extract_full_index(
        &self,
        _reader: &mut dyn ReadSeek,
        _progress_callback: ProgressCallback<'_>,
        _should_cancel: CancelCallback<'_>,
    ) -> ExtractResult<Vec<FrameMetadata>> {
        Err(BitvueError::UnsupportedCodec(self.codec.clone()))
    }

    fn is_supported(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    include!("index_extractor_test.rs");
}
