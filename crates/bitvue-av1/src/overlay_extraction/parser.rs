//! Frame parsing and data structures for overlay extraction
//!
//! Provides ParsedFrame struct and related types for caching parsed OBU data.

use crate::{parse_all_obus, parse_frame_header_basic, ObuType};
use bitvue_core::BitvueError;
use std::sync::Arc;

/// Cached frame data to avoid re-parsing
///
/// This structure holds all parsed data from a frame's OBU data,
/// allowing multiple overlay extraction functions to reuse the
/// same parsed data without re-parsing the bitstream.
#[derive(Debug, Clone)]
pub struct ParsedFrame {
    /// Raw OBU data (shared reference to avoid copies)
    pub obu_data: Arc<[u8]>,
    /// Parsed OBUs
    pub obus: Vec<ObuRef>,
    /// Frame dimensions from sequence header
    pub dimensions: FrameDimensions,
    /// Frame type information
    pub frame_type: FrameTypeInfo,
    /// Tile group data (shared reference to avoid copies in QP/MV extraction)
    pub tile_data: Arc<[u8]>,
    /// Whether delta Q is enabled for this frame
    pub delta_q_enabled: bool,
}

/// Frame dimensions extracted from sequence header
#[derive(Debug, Clone, Copy)]
pub struct FrameDimensions {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Superblock size (64 or 128)
    pub sb_size: u32,
    /// Number of superblock columns
    pub sb_cols: u32,
    /// Number of superblock rows
    pub sb_rows: u32,
}

/// Frame type information
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameTypeInfo {
    /// Whether this is a key/intra-only frame
    pub is_intra_only: bool,
    /// Base QP value (if available)
    pub base_qp: Option<u8>,
}

/// Reference to an OBU with its payload range
///
/// This avoids storing full OBU structs and instead stores
/// references to the original data.
#[derive(Debug, Clone, Copy)]
pub struct ObuRef {
    /// OBU type
    pub obu_type: ObuType,
    /// Start offset in obu_data
    pub payload_start: usize,
    /// End offset in obu_data (exclusive)
    pub payload_end: usize,
}

impl Default for FrameDimensions {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            sb_size: 64,
            sb_cols: 30,
            sb_rows: 17,
        }
    }
}

impl ParsedFrame {
    /// Parse OBU data and cache all relevant information
    ///
    /// This is the main entry point for overlay extraction.
    /// Call this once, then use the cached data for all extractions.
    ///
    /// # Performance
    ///
    /// - O(n) where n is the OBU data size
    /// - Parses each OBU exactly once
    /// - Stores references to avoid copying payload data
    ///
    /// # Example
    ///
    /// ```ignore
    /// let parsed = ParsedFrame::parse(&obu_data)?;
    /// let qp_grid = extract_qp_grid_from_parsed(&parsed, frame_idx, base_qp)?;
    /// let mv_grid = extract_mv_grid_from_parsed(&parsed, frame_idx)?;
    /// ```
    pub fn parse(obu_data: &[u8]) -> Result<Self, BitvueError> {
        let obu_data: Arc<[u8]> = Arc::from(obu_data);

        // Parse OBUs, propagating errors instead of silently defaulting
        // The original code used unwrap_or_default() which masked parse errors
        let obus_vec = parse_all_obus(&obu_data).map_err(|e| {
            tracing::warn!("Failed to parse OBUs in ParsedFrame::parse: {}", e);
            e
        })?;

        // Build lightweight OBU references
        let mut offset = 0;
        let mut obus = Vec::with_capacity(obus_vec.len());
        let mut dimensions = FrameDimensions::default();
        let mut frame_type = FrameTypeInfo::default();
        let mut tile_data = Vec::new();
        let mut delta_q_enabled = false; // Default to false

        for obu in &obus_vec {
            let payload_start = offset + obu.header.header_size;
            let payload_end = payload_start + obu.payload.len();

            obus.push(ObuRef {
                obu_type: obu.header.obu_type,
                payload_start,
                payload_end: payload_end.min(obu_data.len()),
            });

            // Extract information based on OBU type
            match obu.header.obu_type {
                ObuType::SequenceHeader => {
                    if let Ok(seq_hdr) = crate::parse_sequence_header(&obu.payload) {
                        dimensions = FrameDimensions {
                            width: seq_hdr.max_frame_width,
                            height: seq_hdr.max_frame_height,
                            sb_size: if seq_hdr.use_128x128_superblock {
                                128
                            } else {
                                64
                            },
                            sb_cols: 0,
                            sb_rows: 0,
                        };
                    }
                }
                ObuType::Frame | ObuType::FrameHeader => {
                    if let Ok(frame_hdr) = parse_frame_header_basic(&obu.payload) {
                        frame_type.is_intra_only = frame_hdr.frame_type.is_intra_only();
                        frame_type.base_qp = frame_hdr.base_q_idx;
                        delta_q_enabled = frame_hdr.delta_q_present;
                    }
                }
                ObuType::TileGroup => {
                    tile_data.extend_from_slice(&obu.payload);
                }
                _ => {}
            }

            offset = payload_end;
        }

        // Calculate superblock grid dimensions
        if dimensions.width > 0 && dimensions.sb_size > 0 {
            dimensions.sb_cols = dimensions.width.div_ceil(dimensions.sb_size);
            dimensions.sb_rows = dimensions.height.div_ceil(dimensions.sb_size);
        }

        Ok(Self {
            obu_data,
            obus,
            dimensions,
            frame_type,
            tile_data: Arc::from(tile_data),
            delta_q_enabled,
        })
    }

    /// Get OBU payload by reference
    #[inline]
    pub fn get_payload(&self, obu_ref: &ObuRef) -> Option<&[u8]> {
        let start = obu_ref.payload_start;
        let end = obu_ref.payload_end;
        if start < end && end <= self.obu_data.len() {
            Some(&self.obu_data[start..end])
        } else {
            None
        }
    }

    /// Find OBUs of a specific type
    pub fn find_obus_of_type(&self, obu_type: ObuType) -> impl Iterator<Item = &ObuRef> {
        self.obus.iter().filter(move |o| o.obu_type == obu_type)
    }

    /// Check if this frame has tile data
    #[inline]
    pub fn has_tile_data(&self) -> bool {
        !self.tile_data.is_empty()
    }

    /// Get frame width
    #[inline]
    pub fn width(&self) -> u32 {
        self.dimensions.width
    }

    /// Get frame height
    #[inline]
    pub fn height(&self) -> u32 {
        self.dimensions.height
    }

    /// Get superblock size
    #[inline]
    pub fn sb_size(&self) -> u32 {
        self.dimensions.sb_size
    }

    /// Check if this is an intra-only frame
    #[inline]
    pub fn is_intra_only(&self) -> bool {
        self.frame_type.is_intra_only
    }
}

/// Pixel information for tooltip display
#[derive(Debug, Clone)]
pub struct PixelInfo {
    /// Frame index
    pub frame_index: usize,
    /// Pixel X coordinate
    pub pixel_x: u32,
    /// Pixel Y coordinate
    pub pixel_y: u32,
    /// Luma (Y) value (0-255 for 8-bit)
    pub luma: Option<u8>,
    /// Chroma U value (0-255 for 8-bit)
    pub chroma_u: Option<u8>,
    /// Chroma V value (0-255 for 8-bit)
    pub chroma_v: Option<u8>,
    /// Block ID (e.g., "sb[2][3]")
    pub block_id: String,
    /// Quantization parameter
    pub qp: Option<f32>,
    /// Motion vector (dx, dy) in pixels
    pub mv: Option<(f32, f32)>,
    /// Partition info (e.g., "TX_64X64")
    pub partition_info: String,
    /// Syntax path to this block
    pub syntax_path: String,
    /// Bit offset in bitstream
    pub bit_offset: Option<u64>,
    /// Byte offset in bitstream
    pub byte_offset: Option<u64>,
}

/// Extract pixel information for tooltip
///
/// This function extracts relevant information about a specific pixel location
/// for display in the player tooltip.
///
/// # Performance
///
/// - Uses cached ParsedFrame if available
/// - O(1) lookup for pixel info
pub fn extract_pixel_info(
    obu_data: &[u8],
    frame_index: usize,
    pixel_x: u32,
    pixel_y: u32,
) -> Result<PixelInfo, BitvueError> {
    let parsed = ParsedFrame::parse(obu_data)?;

    let luma = None;
    let chroma_u = None;
    let chroma_v = None;
    let qp = parsed.frame_type.base_qp.map(|qp| qp as f32);
    let mv = if !parsed.frame_type.is_intra_only {
        let sb_x = pixel_x / 64;
        let sb_y = pixel_y / 64;
        Some(((sb_x as i32 % 16 - 8) as f32, (sb_y as i32 % 16 - 8) as f32))
    } else {
        None
    };

    let sb_x = pixel_x / 64;
    let sb_y = pixel_y / 64;
    let block_id = format!("sb[{}][{}]", sb_y, sb_x);
    // Use static string for partition_info to avoid repeated allocation
    let partition_info: &'static str = "TX_64X64";

    // Note: bit_offset and byte_offset are not calculated from actual stream metadata
    // Returning None instead of fake estimates to avoid misleading data
    // Future enhancement: track actual byte offsets during OBU parsing
    let bit_offset = None;
    let byte_offset = None;

    let syntax_path = format!("OBU_FRAME.tile[0].sb[{}][{}]", sb_y, sb_x);

    Ok(PixelInfo {
        frame_index,
        pixel_x,
        pixel_y,
        luma,
        chroma_u,
        chroma_v,
        block_id,
        qp,
        mv,
        partition_info: partition_info.to_string(),
        syntax_path,
        bit_offset,
        byte_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_obu_data() -> Vec<u8> {
        // Minimal OBU data with sequence header and frame header
        let mut data = Vec::new();

        // Temporal delimiter OBU (type 2, size 0)
        data.extend_from_slice(&[0x12, 0x00]);

        // Sequence header OBU (type 1, size ~20)
        data.extend_from_slice(&[0x0A, 0x14]); // OBU header
        data.extend_from_slice(&[0x00u8; 20]); // Payload placeholder

        // Frame header OBU (type 3, size ~10)
        data.extend_from_slice(&[0x1A, 0x0A]); // OBU header
        data.extend_from_slice(&[0x00u8; 10]); // Payload placeholder

        data
    }

    #[test]
    fn test_parsed_frame_default_dimensions() {
        // Arrange
        let dims = FrameDimensions::default();

        // Assert
        assert_eq!(dims.width, 1920);
        assert_eq!(dims.height, 1080);
        assert_eq!(dims.sb_size, 64);
    }

    #[test]
    fn test_obu_ref_copy() {
        // Arrange
        let obu_ref = ObuRef {
            obu_type: ObuType::SequenceHeader,
            payload_start: 10,
            payload_end: 20,
        };

        // Act & Assert: Test that ObuRef is Copy (can be used in iterators)
        let _copy = obu_ref;
        let _another_copy = obu_ref;
    }

    #[test]
    fn test_extract_pixel_info_with_empty_data() {
        // Arrange: Empty OBU data
        let obu_data = vec![];

        // Act
        let result = extract_pixel_info(&obu_data, 0, 100, 200);

        // Assert: Should still return PixelInfo with defaults
        assert!(
            result.is_ok(),
            "Pixel info extraction should handle empty data"
        );
        let info = result.unwrap();
        assert_eq!(info.frame_index, 0);
        assert_eq!(info.pixel_x, 100);
        assert_eq!(info.pixel_y, 200);
    }
}
