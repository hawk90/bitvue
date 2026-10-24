//! AV1 Advanced Features Extraction
//!
//! This module provides extraction functions for AV1-specific advanced features:
//! - CDEF (Constrained Directional Enhancement Filter)
//! - Loop Restoration
//! - Film Grain synthesis
//! - Super Resolution

use crate::frame_header::{FrameHeader, LoopRestorationType as FrameLoopRestorationType};
use serde::{Deserialize, Serialize};

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

/// Loop restoration type (re-exported from frame_header for serialization)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopRestorationType {
    None = 0,
    Wiener = 1,
    SgrProj = 2,
    Switchable = 3,
}

impl From<FrameLoopRestorationType> for LoopRestorationType {
    fn from(t: FrameLoopRestorationType) -> Self {
        match t {
            FrameLoopRestorationType::None => Self::None,
            FrameLoopRestorationType::Wiener => Self::Wiener,
            FrameLoopRestorationType::SgrProj => Self::SgrProj,
            FrameLoopRestorationType::Switchable => Self::Switchable,
        }
    }
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
    let block_size = 8u32;
    let grid_w = width.div_ceil(block_size);
    let grid_h = height.div_ceil(block_size);

    // Use frame-level strengths for all blocks (per-CTU direction requires tile decoding)
    let strength = frame_header.cdef_y_primary_strength;
    let block_strengths = (0..grid_h)
        .flat_map(|y| {
            (0..grid_w).map(move |x| CdefBlock {
                x: x * block_size,
                y: y * block_size,
                size: block_size,
                direction: 0, // direction is per-CTU; not available without tile decoding
                strength,
            })
        })
        .collect();

    Some(CdefData {
        width,
        height,
        block_strengths,
        damping: frame_header.cdef_damping.damping,
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

    let grid_w = width.div_ceil(unit_size);
    let grid_h = height.div_ceil(unit_size);

    // Use frame-level restoration type for all units (per-unit type requires tile decoding)
    let y_type: LoopRestorationType = frame_header.loop_restoration.y_type.into();
    let units: Vec<RestorationUnit> = (0..grid_h)
        .flat_map(|y| {
            (0..grid_w).map(move |x| RestorationUnit {
                x: x * unit_size,
                y: y * unit_size,
                size: unit_size,
                restoration_type: y_type,
                filter_data: None, // filter coefficients require tile decoding
            })
        })
        .collect();

    Some(LoopRestorationData {
        width,
        height,
        y_restoration_type: frame_header.loop_restoration.y_type.into(),
        u_restoration_type: frame_header.loop_restoration.u_type.into(),
        v_restoration_type: frame_header.loop_restoration.v_type.into(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame_header::{
        parse_frame_header_basic, CdefInfo, FilmGrainInfo, LoopRestorationInfo, SuperResolutionInfo,
    };

    /// A header with every tool off: `show_existing_frame` is the one parse that needs no payload.
    fn base() -> FrameHeader {
        parse_frame_header_basic(&[0b1000_0000]).unwrap()
    }

    fn with_size(width: u32, height: u32) -> FrameHeader {
        FrameHeader {
            width,
            height,
            ..base()
        }
    }

    // ---- CDEF ----

    fn cdef_header(width: u32, height: u32, bits: u8) -> FrameHeader {
        FrameHeader {
            cdef_damping: CdefInfo {
                enabled: true,
                damping: 5,
                y_primary_strength: 1,
                y_secondary_strength: 2,
                uv_primary_strength: 3,
                uv_secondary_strength: 4,
                bits,
            },
            cdef_y_primary_strength: 11,
            cdef_y_secondary_strength: 12,
            cdef_uv_primary_strength: 13,
            cdef_uv_secondary_strength: 14,
            ..with_size(width, height)
        }
    }

    #[test]
    fn cdef_is_absent_when_disabled_whatever_else_the_header_says() {
        let header = FrameHeader {
            cdef_damping: CdefInfo {
                enabled: false,
                ..cdef_header(64, 64, 2).cdef_damping
            },
            ..cdef_header(64, 64, 2)
        };
        assert!(extract_cdef_data(&header).is_none());
        assert!(extract_cdef_data(&base()).is_none());
    }

    #[test]
    fn cdef_reports_the_frame_level_values_from_their_own_header_fields() {
        let data = extract_cdef_data(&cdef_header(64, 32, 2)).unwrap();
        assert_eq!((data.width, data.height), (64, 32));
        assert_eq!(data.damping, 5);
        assert_eq!(data.y_primary_strength, 11);
        assert_eq!(data.y_secondary_strength, 12);
        assert_eq!(data.uv_primary_strength, 13);
        assert_eq!(data.uv_secondary_strength, 14);
    }

    #[test]
    fn cdef_blocks_tile_the_frame_in_8x8_raster_order() {
        let data = extract_cdef_data(&cdef_header(24, 16, 0)).unwrap();
        let positions: Vec<(u32, u32)> = data.block_strengths.iter().map(|b| (b.x, b.y)).collect();
        assert_eq!(
            positions,
            vec![(0, 0), (8, 0), (16, 0), (0, 8), (8, 8), (16, 8)]
        );
        for block in &data.block_strengths {
            assert_eq!((block.size, block.direction, block.strength), (8, 0, 11));
        }
    }

    #[test]
    fn cdef_grid_rounds_a_partial_block_up_and_is_empty_for_an_unparsed_size() {
        // 17x9 needs 3x2 blocks of 8.
        assert_eq!(
            extract_cdef_data(&cdef_header(17, 9, 0))
                .unwrap()
                .block_strengths
                .len(),
            6
        );
        // Exactly one block.
        assert_eq!(
            extract_cdef_data(&cdef_header(8, 8, 0))
                .unwrap()
                .block_strengths
                .len(),
            1
        );
        assert_eq!(
            extract_cdef_data(&cdef_header(1, 1, 0))
                .unwrap()
                .block_strengths
                .len(),
            1
        );
        // 0x0 is "not parsed": no blocks, but still Some.
        assert!(extract_cdef_data(&cdef_header(0, 0, 0))
            .unwrap()
            .block_strengths
            .is_empty());
        assert!(extract_cdef_data(&cdef_header(0, 64, 0))
            .unwrap()
            .block_strengths
            .is_empty());
        assert!(extract_cdef_data(&cdef_header(64, 0, 0))
            .unwrap()
            .block_strengths
            .is_empty());
    }

    // ---- loop restoration ----

    fn lr_header(width: u32, height: u32, unit_size: u32) -> FrameHeader {
        use crate::frame_header::LoopRestorationType as T;
        FrameHeader {
            loop_restoration: LoopRestorationInfo {
                enabled: true,
                unit_size,
                uv_unit_size: unit_size / 2,
                y_type: T::Wiener,
                u_type: T::SgrProj,
                v_type: T::Switchable,
            },
            ..with_size(width, height)
        }
    }

    #[test]
    fn loop_restoration_is_absent_when_disabled() {
        let header = FrameHeader {
            loop_restoration: LoopRestorationInfo {
                enabled: false,
                ..lr_header(64, 64, 64).loop_restoration
            },
            ..lr_header(64, 64, 64)
        };
        assert!(extract_loop_restoration_data(&header).is_none());
        assert!(extract_loop_restoration_data(&base()).is_none());
    }

    #[test]
    fn loop_restoration_keeps_each_plane_type_distinct() {
        let data = extract_loop_restoration_data(&lr_header(128, 64, 64)).unwrap();
        assert_eq!((data.width, data.height, data.unit_size), (128, 64, 64));
        assert_eq!(data.y_restoration_type, LoopRestorationType::Wiener);
        assert_eq!(data.u_restoration_type, LoopRestorationType::SgrProj);
        assert_eq!(data.v_restoration_type, LoopRestorationType::Switchable);
    }

    #[test]
    fn every_frame_restoration_type_converts_to_its_serializable_twin() {
        use crate::frame_header::LoopRestorationType as T;
        for (from, to, code) in [
            (T::None, LoopRestorationType::None, 0),
            (T::Wiener, LoopRestorationType::Wiener, 1),
            (T::SgrProj, LoopRestorationType::SgrProj, 2),
            (T::Switchable, LoopRestorationType::Switchable, 3),
        ] {
            assert_eq!(LoopRestorationType::from(from), to);
            assert_eq!(to as u8, code);
        }
    }

    #[test]
    fn restoration_units_cover_the_frame_in_raster_order_using_the_luma_type() {
        let data = extract_loop_restoration_data(&lr_header(130, 70, 64)).unwrap();
        // ceil(130/64) = 3 columns, ceil(70/64) = 2 rows.
        let positions: Vec<(u32, u32)> = data.units.iter().map(|u| (u.x, u.y)).collect();
        assert_eq!(
            positions,
            vec![(0, 0), (64, 0), (128, 0), (0, 64), (64, 64), (128, 64)]
        );
        for unit in &data.units {
            assert_eq!(unit.size, 64);
            assert_eq!(unit.restoration_type, LoopRestorationType::Wiener);
            assert!(unit.filter_data.is_none());
        }
    }

    #[test]
    fn restoration_units_use_the_frame_unit_size_for_size_and_position() {
        for unit_size in [64u32, 128, 256] {
            let data = extract_loop_restoration_data(&lr_header(300, 300, unit_size)).unwrap();
            assert_eq!(data.unit_size, unit_size);
            let expected_cols = 300u32.div_ceil(unit_size);
            assert_eq!(data.units.len() as u32, expected_cols * expected_cols);
            for (i, unit) in data.units.iter().enumerate() {
                let (col, row) = (i as u32 % expected_cols, i as u32 / expected_cols);
                assert_eq!(
                    (unit.x, unit.y, unit.size),
                    (col * unit_size, row * unit_size, unit_size),
                    "unit {i} at size {unit_size}"
                );
            }
        }
    }

    #[test]
    fn restoration_unit_grid_boundaries() {
        for (width, height, unit, expected) in [
            (64, 64, 64, 1),
            (65, 64, 64, 2),
            (64, 65, 64, 2),
            (256, 256, 256, 1),
            (257, 1, 256, 2),
            (1, 1, 64, 1),
            (0, 0, 64, 0),
            (512, 128, 128, 4),
        ] {
            let data = extract_loop_restoration_data(&lr_header(width, height, unit)).unwrap();
            assert_eq!(data.units.len(), expected, "{width}x{height} unit {unit}");
        }
    }

    // ---- film grain ----

    fn grain_info() -> FilmGrainInfo {
        FilmGrainInfo {
            enabled: true,
            update_offset: 7,
            seed: 0xBEEF,
            scaling_shift: 9,
            ar_coeff_lag: 2,
            ar_coeffs_y: vec![-3, 0, 5],
            ar_coeffs_uv: vec![1, -128, 127, 4],
            ar_coeff_shift: 8,
            grain_scale_shift: 3,
            chroma_scaling_from_luma: true,
            overlap: true,
            clip_to_restricted_range: true,
            ..Default::default()
        }
    }

    #[test]
    fn film_grain_is_absent_when_disabled() {
        let header = FrameHeader {
            film_grain: FilmGrainInfo {
                enabled: false,
                ..grain_info()
            },
            ..base()
        };
        assert!(extract_film_grain_data(&header).is_none());
        assert!(extract_film_grain_data(&base()).is_none());
    }

    #[test]
    fn film_grain_copies_every_parameter_from_its_own_field() {
        let header = FrameHeader {
            film_grain: grain_info(),
            ..base()
        };
        let data = extract_film_grain_data(&header).unwrap();
        assert!(data.enabled);
        assert_eq!(data.update_offset, 7);
        assert_eq!(data.seed, 0xBEEF);
        assert_eq!(data.scaling_shift, 9);
        assert_eq!(data.ar_coeff_lag, 2);
        assert_eq!(data.ar_coeffs_y, vec![-3, 0, 5]);
        assert_eq!(data.ar_coeffs_uv, vec![1, -128, 127, 4]);
        assert_eq!(data.ar_coeff_shift, 8);
        assert_eq!(data.grain_scale_shift, 3);
        assert!(data.chroma_scaling_from_luma);
        assert!(data.overlap);
        assert!(data.clip_to_restricted_range);
    }

    #[test]
    fn film_grain_flags_are_independent_of_each_other() {
        for flag in 0..3 {
            let mut info = FilmGrainInfo {
                chroma_scaling_from_luma: false,
                overlap: false,
                clip_to_restricted_range: false,
                ..grain_info()
            };
            match flag {
                0 => info.chroma_scaling_from_luma = true,
                1 => info.overlap = true,
                _ => info.clip_to_restricted_range = true,
            }
            let header = FrameHeader {
                film_grain: info,
                ..base()
            };
            let data = extract_film_grain_data(&header).unwrap();
            assert_eq!(
                (
                    data.chroma_scaling_from_luma,
                    data.overlap,
                    data.clip_to_restricted_range
                ),
                (flag == 0, flag == 1, flag == 2),
                "flag {flag}"
            );
        }
    }

    // ---- super resolution ----

    #[test]
    fn super_resolution_is_absent_when_disabled() {
        let header = FrameHeader {
            super_resolution: SuperResolutionInfo {
                enabled: false,
                scale_denominator: 12,
            },
            upscaled_width: 200,
            upscaled_height: 100,
            ..base()
        };
        assert!(extract_super_resolution_data(&header).is_none());
        assert!(extract_super_resolution_data(&base()).is_none());
    }

    #[test]
    fn super_resolution_reports_the_denominator_and_the_upscaled_size_not_the_coded_size() {
        let header = FrameHeader {
            super_resolution: SuperResolutionInfo {
                enabled: true,
                scale_denominator: 12,
            },
            upscaled_width: 200,
            upscaled_height: 100,
            ..with_size(100, 50)
        };
        let data = extract_super_resolution_data(&header).unwrap();
        assert!(data.enabled);
        assert_eq!(data.scale_denominator, 12);
        assert_eq!((data.upscaled_width, data.upscaled_height), (200, 100));
    }

    // ---- the four extractors do not read each other's flags ----

    #[test]
    fn enabling_one_tool_does_not_enable_the_others() {
        let header = cdef_header(32, 32, 1);
        assert!(extract_cdef_data(&header).is_some());
        assert!(extract_loop_restoration_data(&header).is_none());
        assert!(extract_film_grain_data(&header).is_none());
        assert!(extract_super_resolution_data(&header).is_none());
    }
}
