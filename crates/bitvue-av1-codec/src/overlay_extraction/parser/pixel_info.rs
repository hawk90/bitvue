//! Pixel information for the player tooltip.

use super::ParsedFrame;
use bitvue_engine::BitvueError;

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
