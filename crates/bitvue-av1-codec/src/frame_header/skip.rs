//! The header syntax elements `parse_frame_header_basic` reads past without keeping.

use crate::bitreader::BitReader;
use bitvue_engine::BitvueError;

/// Segmentation params – skip all bits.
/// Per AV1 spec Section 5.9.14 (segmentation_params).
/// This skips the segmentation block without interpreting it.
#[allow(dead_code)]
pub(super) fn skip_segmentation_params(reader: &mut BitReader) -> Result<(), BitvueError> {
    // segmentation_enabled  f(1)
    let seg_enabled = reader.read_bit()?;
    if seg_enabled {
        // segmentation_update_map  f(1)
        let update_map = reader.read_bit()?;
        if update_map {
            // segmentation_temporal_update  f(1)
            reader.read_bit()?;
        }
        // segmentation_update_data  f(1)
        let update_data = reader.read_bit()?;
        if update_data {
            // For each of 8 segments, read feature flags and values
            // Per spec: 8 segments × (SEG_LVL_MAX=8 features × (1 flag + value if enabled))
            for _seg in 0..8usize {
                for feature in 0..8usize {
                    let feature_enabled = reader.read_bit()?;
                    if feature_enabled {
                        // Feature value bit widths (per spec Table 5):
                        // 0=ALT_Q(8), 1=ALT_LF_Y_V(6), 2=ALT_LF_Y_H(6),
                        // 3=ALT_LF_U(6), 4=ALT_LF_V(6), 5=REF_FRAME(3),
                        // 6=SKIP(0), 7=GLOBALMV(0)
                        let (bits, signed) = match feature {
                            0 => (8u8, true),
                            1..=4 => (6u8, true),
                            5 => (3u8, false),
                            _ => (0u8, false),
                        };
                        if bits > 0 {
                            reader.read_bits(bits)?;
                            if signed {
                                // sign bit
                                reader.read_bit()?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Skip loop_filter_params() per AV1 spec Section 5.9.11.
#[allow(dead_code)]
pub(super) fn skip_loop_filter_params(
    reader: &mut BitReader,
    is_intra: bool,
    allow_intrabc: bool,
) -> Result<(), BitvueError> {
    if is_intra || allow_intrabc {
        return Ok(());
    }
    // loop_filter_level[0]  u(6)
    reader.read_bits(6)?;
    // loop_filter_level[1]  u(6)
    reader.read_bits(6)?;
    // loop_filter_sharpness  u(3)
    reader.read_bits(3)?;
    // loop_filter_delta_enabled  f(1)
    let delta_enabled = reader.read_bit()?;
    if delta_enabled {
        // loop_filter_delta_update  f(1)
        let delta_update = reader.read_bit()?;
        if delta_update {
            // 8 ref delta loop filters + 2 mode delta loop filters
            for _ in 0..10usize {
                let update_delta = reader.read_bit()?;
                if update_delta {
                    // loop_filter_ref_deltas[i] / loop_filter_mode_deltas[i]  su(7)
                    reader.read_su(7)?;
                }
            }
        }
    }
    Ok(())
}

/// Skip cdef_params() per AV1 spec Section 5.9.19.
#[allow(dead_code)]
pub(super) fn skip_cdef_params(
    reader: &mut BitReader,
    is_intra: bool,
    allow_intrabc: bool,
    enable_cdef: bool,
) -> Result<(), BitvueError> {
    if !enable_cdef || allow_intrabc || is_intra {
        return Ok(());
    }
    // cdef_damping_minus_3  u(2)
    reader.read_bits(2)?;
    // cdef_bits  u(2)
    let cdef_bits = reader.read_bits(2)?;
    let num_cdef_strengths = 1usize << cdef_bits;
    for _ in 0..num_cdef_strengths {
        // cdef_y_pri_strength  u(4)
        reader.read_bits(4)?;
        // cdef_y_sec_strength  u(2)
        reader.read_bits(2)?;
        // cdef_uv_pri_strength  u(4)
        reader.read_bits(4)?;
        // cdef_uv_sec_strength  u(2)
        reader.read_bits(2)?;
    }
    Ok(())
}

/// Skip lr_params() per AV1 spec Section 5.9.20.
#[allow(dead_code)]
pub(super) fn skip_lr_params(
    reader: &mut BitReader,
    is_intra: bool,
    allow_intrabc: bool,
    enable_restoration: bool,
    use_128x128_superblock: bool,
) -> Result<(), BitvueError> {
    if !enable_restoration || allow_intrabc || is_intra {
        return Ok(());
    }
    for _plane in 0..3usize {
        // lr_type  u(2)
        reader.read_bits(2)?;
    }
    // lr_unit_shift  u(1)
    let lr_unit_shift = reader.read_bits(1)?;
    if use_128x128_superblock && lr_unit_shift > 0 {
        // lr_unit_extra_shift  u(1)
        reader.read_bits(1)?;
    }
    // lr_uv_shift  u(1)  (only for sub-sampled chroma)
    reader.read_bits(1)?;
    Ok(())
}

/// Skip tx_mode_select() per AV1 spec Section 5.9.21.
#[allow(dead_code)]
pub(super) fn skip_tx_mode(reader: &mut BitReader, is_intra: bool) -> Result<(), BitvueError> {
    if !is_intra {
        // tx_mode_select  f(1)
        reader.read_bit()?;
    }
    Ok(())
}

/// Skip frame_reference_mode() per AV1 spec Section 5.9.23.
#[allow(dead_code)]
pub(super) fn skip_frame_reference_mode(
    reader: &mut BitReader,
    is_intra: bool,
) -> Result<(), BitvueError> {
    if !is_intra {
        // reference_select  f(1)
        reader.read_bit()?;
    }
    Ok(())
}

/// Skip skip_mode_params() per AV1 spec Section 5.9.22.
#[allow(dead_code)]
pub(super) fn skip_skip_mode_params(
    reader: &mut BitReader,
    is_intra: bool,
    reference_select: bool,
) -> Result<(), BitvueError> {
    // Simplified: skip_mode_allowed depends on many conditions
    // For now just try to read the flag for inter frames with reference_select
    if !is_intra && reference_select {
        // skip_mode_present  f(1)
        reader.read_bit()?;
    }
    Ok(())
}
