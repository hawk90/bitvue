//! Loop filter, delta LF, CDEF and loop-restoration parameters (spec 5.9.11-5.9.20).

use super::*;

const RESTORATION_TILESIZE_MAX: u32 = 256;
/// `delta_lf_params()` (spec 5.9.14) -- real `delta_lf_present`/`delta_lf_multi` (`delta_lf_res` is
/// consumed for bitstream position but not retained: it only scales the *decoded* per-CU
/// `delta_lf` magnitude, never affects tile-data bit position -- same reasoning as this crate not
/// tracking `delta_q_res` either).
pub(super) fn parse_delta_lf_params(
    reader: &mut BitReader,
    delta_q_present: bool,
    allow_intrabc: bool,
) -> Result<(bool, bool)> {
    if !delta_q_present {
        return Ok((false, false));
    }
    let delta_lf_present = if !allow_intrabc {
        reader.read_bit()?
    } else {
        false
    };
    let delta_lf_multi = if delta_lf_present {
        reader.read_bits(2)?; // delta_lf_res
        reader.read_bit()?
    } else {
        false
    };
    Ok((delta_lf_present, delta_lf_multi))
}

/// Parse `loop_filter_params()` per AV1 spec Section 5.9.11. Unlike `skip_loop_filter_params`
/// (`crate::frame_header`), this retains every value -- needed by `get_deblocking_analysis` to
/// report real per-plane filter levels/sharpness/deltas alongside boundary-strength derivation
/// (see `overlay_extraction::deblocking`'s module doc for how BS itself is computed from
/// coding-unit data, independent of these values).
///
/// Note: the spec's `loop_filter_delta_update` only rewrites deltas that are explicitly signaled
/// (`update_ref_delta`/`update_mode_delta`); un-signaled deltas keep their *previous frame's*
/// value (`PrevRefDeltas`/`PrevModeDeltas`, spec 7.20). This parser has no cross-frame delta
/// state (unlike `RefFrameState`'s order-hint tracking, which is required for correct bit
/// alignment) -- un-signaled deltas here reset to 0 rather than carrying forward. This only
/// affects the *reported* delta values, never bit consumption (every delta's presence bit is
/// still read correctly either way).
pub(super) fn parse_loop_filter_params(
    reader: &mut BitReader,
    num_planes: u8,
    coded_lossless: bool,
    allow_intrabc: bool,
) -> Result<LoopFilterInfo> {
    if coded_lossless || allow_intrabc {
        return Ok(LoopFilterInfo::default());
    }
    let mut level = [0u8; 4];
    level[0] = reader.read_bits(6)? as u8;
    level[1] = reader.read_bits(6)? as u8;
    if num_planes > 1 && (level[0] != 0 || level[1] != 0) {
        level[2] = reader.read_bits(6)? as u8;
        level[3] = reader.read_bits(6)? as u8;
    }
    let sharpness = reader.read_bits(3)? as u8;
    let delta_enabled = reader.read_bit()?;
    let mut ref_deltas = [0i8; NUM_REF_FRAMES];
    let mut mode_deltas = [0i8; 2];
    if delta_enabled {
        let delta_update = reader.read_bit()?;
        if delta_update {
            for delta in ref_deltas.iter_mut() {
                if reader.read_bit()? {
                    *delta = reader.read_su(7)? as i8;
                }
            }
            for delta in mode_deltas.iter_mut() {
                if reader.read_bit()? {
                    *delta = reader.read_su(7)? as i8;
                }
            }
        }
    }
    Ok(LoopFilterInfo {
        level,
        sharpness,
        delta_enabled,
        ref_deltas,
        mode_deltas,
    })
}

pub(super) fn parse_cdef_params(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    coded_lossless: bool,
    allow_intrabc: bool,
) -> Result<CdefInfo> {
    if coded_lossless || allow_intrabc || !seq.enable_cdef {
        return Ok(CdefInfo {
            enabled: false,
            damping: 3,
            ..Default::default()
        });
    }
    let damping = (reader.read_bits(2)? + 3) as u8;
    let cdef_bits = reader.read_bits(2)?;
    let mut y_primary_strength = 0u8;
    let mut y_secondary_strength = 0u8;
    let mut uv_primary_strength = 0u8;
    let mut uv_secondary_strength = 0u8;
    for i in 0..(1u32 << cdef_bits) {
        let y_pri = reader.read_bits(4)? as u8;
        let mut y_sec = reader.read_bits(2)? as u8;
        if y_sec == 3 {
            y_sec += 1;
        }
        let uv_pri = reader.read_bits(4)? as u8;
        let mut uv_sec = reader.read_bits(2)? as u8;
        if uv_sec == 3 {
            uv_sec += 1;
        }
        if i == 0 {
            y_primary_strength = y_pri;
            y_secondary_strength = y_sec;
            uv_primary_strength = uv_pri;
            uv_secondary_strength = uv_sec;
        }
    }
    Ok(CdefInfo {
        enabled: true,
        damping,
        y_primary_strength,
        y_secondary_strength,
        uv_primary_strength,
        uv_secondary_strength,
        bits: cdef_bits as u8,
    })
}

pub(super) fn parse_lr_params(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    all_lossless: bool,
    allow_intrabc: bool,
) -> Result<LoopRestorationInfo> {
    if all_lossless || allow_intrabc || !seq.enable_restoration {
        return Ok(LoopRestorationInfo {
            enabled: false,
            ..Default::default()
        });
    }
    // Remap_Lr_Type (AV1 spec 5.9.20): the 2-bit code is not the enum order.
    const REMAP_LR_TYPE: [LoopRestorationType; 4] = [
        LoopRestorationType::None,
        LoopRestorationType::Switchable,
        LoopRestorationType::Wiener,
        LoopRestorationType::SgrProj,
    ];
    let num_planes = seq.color_config.num_planes;
    let mut types = [LoopRestorationType::None; 3];
    let mut uses_lr = false;
    let mut uses_chroma_lr = false;
    for (i, slot) in types.iter_mut().enumerate().take(num_planes as usize) {
        let code = reader.read_bits(2)?;
        let t = REMAP_LR_TYPE[code as usize & 0x3];
        *slot = t;
        if t != LoopRestorationType::None {
            uses_lr = true;
            if i > 0 {
                uses_chroma_lr = true;
            }
        }
    }
    let mut unit_size = RESTORATION_TILESIZE_MAX;
    let mut uv_unit_size = unit_size;
    if uses_lr {
        let mut lr_unit_shift: u32 = if seq.use_128x128_superblock {
            1 + u32::from(reader.read_bit()?)
        } else {
            let shift = reader.read_bit()?;
            if shift {
                1 + u32::from(reader.read_bit()?)
            } else {
                0
            }
        };
        lr_unit_shift = lr_unit_shift.min(2);
        unit_size = RESTORATION_TILESIZE_MAX >> (2 - lr_unit_shift);
        uv_unit_size = unit_size;
        if seq.color_config.subsampling_x && seq.color_config.subsampling_y && uses_chroma_lr {
            // lr_uv_shift
            uv_unit_size >>= u32::from(reader.read_bit()?);
        }
    }
    Ok(LoopRestorationInfo {
        enabled: uses_lr,
        unit_size,
        uv_unit_size,
        y_type: types[0],
        u_type: if num_planes > 1 {
            types[1]
        } else {
            LoopRestorationType::None
        },
        v_type: if num_planes > 1 {
            types[2]
        } else {
            LoopRestorationType::None
        },
    })
}
