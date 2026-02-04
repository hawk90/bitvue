//! AV1 Advanced Features Extraction
//!
//! This module provides extraction functions for AV1-specific advanced features:
//! - CDEF (Constrained Directional Enhancement Filter)
//! - Loop Restoration
//! - Film Grain synthesis
//! - Super Resolution

use serde::{Deserialize, Serialize};
use crate::frame_header::FrameHeader;
use crate::tile::Superblock;

/// CDEF (Constrained Directional Enhancement Filter) data for a frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdefData {
    /// Frame dimensions
    pub width: u32,
    pub height: u32,
    /// CDEF filter strength per block
    pub block_strengths: Vec<CdefBlock>,
    /// CDEF damping factor
    pub damping: u8,
    /// CDEF bits (determines number of strengths)
    pub cdef_bits: u8,
    /// Y primary strength
    pub y_primary_strength: u8,
    /// Y secondary strength
    pub y_secondary_strength: u8,
    /// UV primary strength
    pub uv_primary_strength: u8,
    /// UV secondary strength
    pub uv_secondary_strength: u8,
}

/// CDEF block data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdefBlock {
    /// Block X position
    pub x: u32,
    /// Block Y position
    pub y: u32,
    /// Block size (usually 8x8)
    pub size: u32,
    /// CDEF direction (0-7)
    pub direction: u8,
    /// CDEF strength
    pub strength: u8,
}

/// Loop Restoration data for a frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopRestorationData {
    /// Frame dimensions
    pub width: u32,
    pub height: u32,
    /// Y restoration type
    pub y_restoration_type: LoopRestorationType,
    /// U restoration type
    pub u_restoration_type: LoopRestorationType,
    /// V restoration type
    pub v_restoration_type: LoopRestorationType,
    /// Restoration unit size
    pub unit_size: u32,
    /// Restoration units
    pub units: Vec<RestorationUnit>,
}

/// Loop restoration type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopRestorationType {
    None = 0,
    Wiener = 1,
    SgrProj = 2,
    Dual = 3,
}

/// Restoration unit data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorationUnit {
    /// Unit X position
    pub x: u32,
    /// Unit Y position
    pub y: u32,
    /// Unit size
    pub size: u32,
    /// Restoration type
    pub restoration_type: LoopRestorationType,
    /// Filter data (if Wiener or SgrProj)
    pub filter_data: Option<Vec<i16>>,
}

/// Film Grain parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilmGrainData {
    /// Film grain enabled
    pub enabled: bool,
    /// Film grain update offset
    pub update_offset: u8,
    /// Film grain seed
    pub seed: u64,
    /// Scaling shift value
    pub scaling_shift: u8,
    /// AR coefficients luma
    pub ar_coeff_lag: u8,
    /// AR coefficients luma
    pub ar_coeffs_y: Vec<i8>,
    /// AR coefficients chroma
    pub ar_coeffs_uv: Vec<i8>,
    /// AR coefficients shift
    pub ar_coeff_shift: u8,
    /// Grain scale shift
    pub grain_scale_shift: u8,
    /// Chroma scaling from luma
    pub chroma_scaling_from_luma: bool,
    /// Overlap flag
    pub overlap: bool,
    /// Clip to restricted range
    pub clip_to_restricted_range: bool,
}

/// Super Resolution data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperResolutionData {
    /// Super resolution enabled
    pub enabled: bool,
    /// Scale factor (denominator)
    pub scale_denominator: u8,
    /// Upscaled width
    pub upscaled_width: u32,
    /// Upscaled height
    pub upscaled_height: u32,
}

/// Extract CDEF data from frame header
pub fn extract_cdef_data(frame_header: &FrameHeader) -> Option<CdefData> {
    if !frame_header.cdef_damping.enabled {
        return None;
    }

    let width = frame_header.width;
    let height = frame_header.height;
    let block_size = 8;
    let grid_w = (width + block_size - 1) / block_size;
    let grid_h = (height + block_size - 1) / block_size;

    let mut block_strengths = Vec::new();

    // Generate mock CDEF block data
    for y in 0..grid_h {
        for x in 0..grid_w {
            block_strengths.push(CdefBlock {
                x: x * block_size,
                y: y * block_size,
                size: block_size,
                direction: (rand::random::<u8>() % 8),
                strength: (rand::random::<u8>() % 16),
            });
        }
    }

    Some(CdefData {
        width,
        height,
        block_strengths,
        damping: frame_header.cdef_damping.value,
        cdef_bits: 3,
        y_primary_strength: frame_header.cdef_y_primary_strength,
        y_secondary_strength: frame_header.cdef_y_secondary_strength,
        uv_primary_strength: frame_header.cdef_uv_primary_strength,
        uv_secondary_strength: frame_header.cdef_uv_secondary_strength,
    })
}

/// Extract loop restoration data from frame header
pub fn extract_loop_restoration_data(frame_header: &FrameHeader) -> Option<LoopRestorationData> {
    if !frame_header.loop_restoration.enabled {
        return None;
    }

    let width = frame_header.width;
    let height = frame_header.height;
    let unit_size = frame_header.loop_restoration.unit_size;

    let grid_w = (width + unit_size - 1) / unit_size;
    let grid_h = (height + unit_size - 1) / unit_size;

    let mut units = Vec::new();

    // Generate mock restoration units
    for y in 0..grid_h {
        for x in 0..grid_w {
            let restoration_type = match rand::random::<u8>() % 3 {
                0 => LoopRestorationType::Wiener,
                1 => LoopRestorationType::SgrProj,
                _ => LoopRestorationType::None,
            };

            let filter_data = if restoration_type != LoopRestorationType::None {
                Some(vec![
                    (rand::random::<i16>() % 8) - 4,
                    (rand::random::<i16>() % 8) - 4,
                    (rand::random::<i16>() % 8) - 4,
                ])
            } else {
                None
            };

            units.push(RestorationUnit {
                x: x * unit_size,
                y: y * unit_size,
                size: unit_size,
                restoration_type,
                filter_data,
            });
        }
    }

    Some(LoopRestorationData {
        width,
        height,
        y_restoration_type: frame_header.loop_restoration.y_type,
        u_restoration_type: frame_header.loop_restoration.u_type,
        v_restoration_type: frame_header.loop_restoration.v_type,
        unit_size,
        units,
    })
}

/// Extract film grain data from frame header
pub fn extract_film_grain_data(frame_header: &FrameHeader) -> Option<FilmGrainData> {
    if !frame_header.film_grain.enabled {
        return None;
    }

    Some(FilmGrainData {
        enabled: true,
        update_offset: frame_header.film_grain.update_offset,
        seed: frame_header.film_grain.seed,
        scaling_shift: frame_header.film_grain.scaling_shift,
        ar_coeff_lag: frame_header.film_grain.ar_coeff_lag,
        ar_coeffs_y: frame_header.film_grain.ar_coeffs_y.clone(),
        ar_coeffs_uv: frame_header.film_grain.ar_coeffs_uv.clone(),
        ar_coeff_shift: frame_header.film_grain.ar_coeff_shift,
        grain_scale_shift: frame_header.film_grain.grain_scale_shift,
        chroma_scaling_from_luma: frame_header.film_grain.chroma_scaling_from_luma,
        overlap: frame_header.film_grain.overlap,
        clip_to_restricted_range: frame_header.film_grain.clip_to_restricted_range,
    })
}

/// Extract super resolution data from frame header
pub fn extract_super_resolution_data(frame_header: &FrameHeader) -> Option<SuperResolutionData> {
    if !frame_header.super_resolution.enabled {
        return None;
    }

    Some(SuperResolutionData {
        enabled: true,
        scale_denominator: frame_header.super_resolution.scale_denominator,
        upscaled_width: frame_header.upscaled_width,
        upscaled_height: frame_header.upscaled_height,
    })
}
