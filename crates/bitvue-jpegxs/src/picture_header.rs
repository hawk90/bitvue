//! JPEG XS Picture Information Header (PIH) parser.
//!
//! PIH marker (0xFF51) contains frame dimensions, decomposition level count,
//! component count, and profile/level information.

use crate::error::{JpegXsError, Result};

/// JPEG XS decomposition (wavelet) levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecompLevels {
    /// Number of horizontal decomposition levels (Nd_x, 0-5).
    pub horizontal: u8,
    /// Number of vertical decomposition levels (Nd_y, 0-5).
    pub vertical: u8,
}

/// JPEG XS slice geometry.
#[derive(Debug, Clone, Copy)]
pub struct SliceGeometry {
    /// Width of each slice column in pixels.
    pub slice_width: u32,
    /// Height of each slice row in pixels.
    pub slice_height: u32,
    /// Number of slice columns.
    pub num_cols: u32,
    /// Number of slice rows.
    pub num_rows: u32,
}

/// JPEG XS Picture Information Header.
#[derive(Debug, Clone)]
pub struct PictureHeader {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Number of colour components (1-4).
    pub num_comps: u8,
    /// Bit depth per component (8, 10, 12).
    pub bit_depth: u8,
    /// Decomposition levels.
    pub decomp: DecompLevels,
    /// Slice geometry (derived from frame dimensions and PIH slice sizes).
    pub slice_geom: Option<SliceGeometry>,
    /// Profile: 0x00=Main444.12, 0x01=Main444.10, 0x02=Main420.10 etc.
    pub profile: u8,
    /// Level byte.
    pub level: u8,
    /// Sublevel byte.
    pub sublevel: u8,
    /// Target bit-rate × 1000 (bits per pixel × 1000).
    pub bpp_x1000: u32,
}

/// Parse a PIH marker payload (bytes after the length field).
///
/// ISO 21122-1 Table A.2: PIH syntax
pub fn parse_pih(data: &[u8]) -> Result<PictureHeader> {
    // Minimum valid PIH is 26 bytes
    if data.len() < 26 {
        return Err(JpegXsError::UnexpectedEof(0));
    }

    let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

    // Byte 8: Nc (number of components, 4 bits) | Ng (decomp group bits, 4 bits)
    let num_comps = ((data[8] >> 4) & 0x0F) + 1;
    // Byte 9-10: slice height
    let slice_height = u16::from_be_bytes([data[9], data[10]]) as u32;
    // Byte 11-12: slice width
    let slice_width = u16::from_be_bytes([data[11], data[12]]) as u32;
    // Byte 13: bit depth (Bw, 4 bits) | padding
    let bit_depth = ((data[13] >> 4) & 0x0F) + 8; // encoded as (bd - 8)
                                                  // Byte 14: profile
    let profile = data[14];
    // Byte 15: level
    let level = data[15];
    // Byte 16: sublevel
    let sublevel = data[16];
    // Bytes 17-18: decomp levels (Nd_x, Nd_y each 3 bits in upper 6)
    let nd_x = (data[17] >> 5) & 0x07;
    let nd_y = (data[17] >> 2) & 0x07;
    // Bytes 19-22: bpp × 1000
    let bpp_x1000 = u32::from_be_bytes([data[19], data[20], data[21], data[22]]);

    let slice_geom = if slice_width > 0 && slice_height > 0 && width > 0 && height > 0 {
        let num_cols = width.div_ceil(slice_width);
        let num_rows = height.div_ceil(slice_height);
        Some(SliceGeometry {
            slice_width,
            slice_height,
            num_cols,
            num_rows,
        })
    } else {
        None
    };

    Ok(PictureHeader {
        width,
        height,
        num_comps,
        bit_depth,
        decomp: DecompLevels {
            horizontal: nd_x,
            vertical: nd_y,
        },
        slice_geom,
        profile,
        level,
        sublevel,
        bpp_x1000,
    })
}
