//! Slice-data walk that turns a slice NAL into macroblocks.

use crate::bitreader::BitReader;
use crate::nal::{NalUnit, NalUnitType};
use crate::pps::Pps;
use crate::slice::{parse_slice_header_reader, SliceType};
use crate::sps::Sps;
use bitvue_engine::BitvueError;
use std::collections::HashMap;

use super::cabac::*;
use super::cavlc::*;
use super::types::*;

/// Parse macroblocks from slice data.
///
/// Parses the slice header to determine slice type and entropy mode, then:
/// - For CAVLC-coded P slices: extracts skip/non-skip MB classification and
///   motion vectors for P_L0_16x16 blocks.
/// - For CABAC or I slices: classifies MBs by slice type without MV data.
///
/// Note: CAVLC residual parsing is not implemented, so after the first
/// non-skip MB the scan stops and remaining MBs get type-based defaults.
pub(super) fn parse_slice_macroblocks(
    nal: &NalUnit,
    sps_map: &HashMap<u8, Sps>,
    pps_map: &HashMap<u8, Pps>,
    fallback_sps: &Sps,
    fallback_qp: i16,
) -> Result<Vec<Macroblock>, BitvueError> {
    let nal_type = nal.header.nal_unit_type;
    let nal_ref_idc = nal.header.nal_ref_idc;

    // Parse the real slice header to get slice type, QP, and entropy mode.
    let mut reader = BitReader::new(&nal.payload);
    let header =
        match parse_slice_header_reader(&mut reader, sps_map, pps_map, nal_type, nal_ref_idc) {
            Ok(h) => h,
            Err(_) => {
                // Fall back to dimension-based scaffold
                return Ok(build_scaffold_mbs(nal_type, fallback_sps, fallback_qp));
            }
        };

    let pps = match pps_map.get(&header.pic_parameter_set_id) {
        Some(p) => p,
        None => {
            // Unlike the slice-header-parse-failure fallback above, the header (and its real
            // slice_type) is already known here -- build_typed_mbs uses it to distinguish
            // B-slices (B16x16) from P-slices (PLuma), which build_scaffold_mbs can't do (it
            // only sees nal_type, which conflates every non-IDR slice into PLuma regardless of
            // whether it's actually a B slice).
            let pic_width_in_mbs = fallback_sps.pic_width_in_mbs_minus1 + 1;
            let total_mbs = pic_width_in_mbs * (fallback_sps.pic_height_in_map_units_minus1 + 1);
            return Ok(build_typed_mbs(
                header.slice_type,
                0,
                total_mbs,
                pic_width_in_mbs,
                fallback_qp,
            ));
        }
    };
    let sps = match sps_map.get(&pps.seq_parameter_set_id) {
        Some(s) => s,
        None => fallback_sps,
    };

    let slice_qp = (26 + pps.pic_init_qp_minus26 + header.slice_qp_delta).clamp(0, 51) as i16;
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let pic_height_in_mbs = sps.pic_height_in_map_units_minus1 + 1;
    let total_mbs = pic_width_in_mbs * pic_height_in_mbs;
    let slice_type = header.slice_type;

    // CABAC: decode mb_skip_flag for each MB using the correct context model.
    // Full residual/MV decoding is not implemented, but skip classification is correct.
    if pps.entropy_coding_mode_flag {
        // The reader is positioned at the first byte after the slice header.
        // Extract remaining bytes for the CABAC decoder.
        let bit_offset = reader.bit_position();
        let byte_offset = bit_offset / 8;
        let payload_after_header = nal.payload.get(byte_offset..).unwrap_or(&[]);
        return Ok(parse_cabac_slice_mbs(
            payload_after_header,
            header.first_mb_in_slice,
            total_mbs,
            pic_width_in_mbs,
            slice_qp,
            slice_type,
            header.cabac_init_idc,
        ));
    }

    // CAVLC parsing
    let num_ref_l0 = header.num_ref_idx_l0_active_minus1;
    let mut mbs = Vec::with_capacity(total_mbs as usize);
    let mut mb_addr = header.first_mb_in_slice;

    'outer: while mb_addr < total_mbs {
        // For P/B slices: read mb_skip_run (ue)
        if !slice_type.is_intra() {
            let mb_skip_run = match reader.read_ue() {
                Ok(v) => v,
                Err(_) => break,
            };
            let skip_type = if slice_type.is_b() {
                MbType::BSkip
            } else {
                MbType::PSkip
            };
            for _ in 0..mb_skip_run {
                if mb_addr >= total_mbs {
                    break 'outer;
                }
                let x = (mb_addr % pic_width_in_mbs) * 16;
                let y = (mb_addr / pic_width_in_mbs) * 16;
                mbs.push(Macroblock {
                    mb_addr,
                    x,
                    y,
                    mb_type: skip_type,
                    skip: true,
                    qp: slice_qp,
                    mv_l0: None,
                    mv_l1: None,
                    ref_idx_l0: None,
                    ref_idx_l1: None,
                });
                mb_addr += 1;
            }
            if mb_addr >= total_mbs {
                break;
            }
        }

        // Parse mb_type for this non-skip MB
        let mb_type_raw = match reader.read_ue() {
            Ok(v) => v,
            Err(_) => break,
        };
        let x = (mb_addr % pic_width_in_mbs) * 16;
        let y = (mb_addr / pic_width_in_mbs) * 16;

        let mb_type = decode_p_mb_type(mb_type_raw, slice_type);
        let is_p16x16 = !slice_type.is_intra() && mb_type_raw == 0;

        if is_p16x16 {
            // te(num_ref_l0): read reference index
            let ref_idx = if num_ref_l0 == 0 {
                0u32
            } else if num_ref_l0 == 1 {
                match reader.read_bit() {
                    Ok(b) => {
                        if b {
                            0
                        } else {
                            1
                        }
                    }
                    Err(_) => break,
                }
            } else {
                match reader.read_ue() {
                    Ok(v) => v,
                    Err(_) => break,
                }
            };
            // mvd_l0[0][0]: horizontal then vertical, SE-coded
            let mvd_x = match reader.read_se() {
                Ok(v) => v,
                Err(_) => break,
            };
            let mvd_y = match reader.read_se() {
                Ok(v) => v,
                Err(_) => break,
            };

            // Emit this MB then try to advance past its CAVLC residuals.
            // CBP index 0 means no coded blocks (CBP=0): most common in P-frames.
            // If skip succeeds we continue parsing; otherwise scaffold the rest.
            mbs.push(Macroblock {
                mb_addr,
                x,
                y,
                mb_type,
                skip: false,
                qp: slice_qp,
                mv_l0: Some(MotionVector::new(mvd_x, mvd_y)),
                mv_l1: None,
                ref_idx_l0: Some(ref_idx.min(127) as i8),
                ref_idx_l1: None,
            });
            mb_addr += 1;
            if !try_skip_cavlc_residuals(&mut reader) {
                break;
            }
            continue;
        }

        // P16x8 or P8x16: 2 partitions, each with one reference index and one MVD pair.
        if !slice_type.is_intra() && (mb_type_raw == 1 || mb_type_raw == 2) {
            // Read 2 reference indices (te() per partition).
            let mut ref_idxs = [0u32; 2];
            for slot in &mut ref_idxs {
                *slot = if num_ref_l0 == 0 {
                    0u32
                } else if num_ref_l0 == 1 {
                    match reader.read_bit() {
                        Ok(b) => {
                            if b {
                                0
                            } else {
                                1
                            }
                        }
                        Err(_) => break 'outer,
                    }
                } else {
                    match reader.read_ue() {
                        Ok(v) => v,
                        Err(_) => break 'outer,
                    }
                };
            }
            // Read 2 MVD pairs.
            let mut sum_x = 0i32;
            let mut sum_y = 0i32;
            for _ in 0..2 {
                let mvd_x = match reader.read_se() {
                    Ok(v) => v,
                    Err(_) => break 'outer,
                };
                let mvd_y = match reader.read_se() {
                    Ok(v) => v,
                    Err(_) => break 'outer,
                };
                sum_x += mvd_x;
                sum_y += mvd_y;
            }
            mbs.push(Macroblock {
                mb_addr,
                x,
                y,
                mb_type,
                skip: false,
                qp: slice_qp,
                mv_l0: Some(MotionVector::new(sum_x / 2, sum_y / 2)),
                mv_l1: None,
                ref_idx_l0: Some(ref_idxs[0].min(127) as i8),
                ref_idx_l1: None,
            });
            mb_addr += 1;
            if !try_skip_cavlc_residuals(&mut reader) {
                break;
            }
            continue;
        }

        // P8x8: 4 sub-partitions. Read sub_mb_type[] then ref_idx[] then MVDs.
        if !slice_type.is_intra() && (mb_type_raw == 3 || mb_type_raw == 4) {
            // Read 4 sub_mb_type values (ue each).
            let mut sub_mb_types = [0u32; 4];
            for smt in &mut sub_mb_types {
                *smt = match reader.read_ue() {
                    Ok(v) => v,
                    Err(_) => break 'outer,
                };
            }
            // Read 4 reference indices (te() each). Skip if B-slice (L1 sub_mb_types
            // would also need reading, but we only track L0 for simplicity).
            let mut first_ref = 0u32;
            for i in 0..4 {
                let ref_idx = if num_ref_l0 == 0 {
                    0u32
                } else if num_ref_l0 == 1 {
                    match reader.read_bit() {
                        Ok(b) => {
                            if b {
                                0
                            } else {
                                1
                            }
                        }
                        Err(_) => break 'outer,
                    }
                } else {
                    match reader.read_ue() {
                        Ok(v) => v,
                        Err(_) => break 'outer,
                    }
                };
                if i == 0 {
                    first_ref = ref_idx;
                }
            }
            // Read MVDs: count per sub-partition depends on sub_mb_type.
            // P_L0_8x8=0 → 1 pair, P_L0_8x4=1 → 2 pairs,
            // P_L0_4x8=2 → 2 pairs, P_L0_4x4=3 → 4 pairs.
            let mut sum_x = 0i32;
            let mut sum_y = 0i32;
            let mut total_mvd_count = 0i32;
            for &smt in &sub_mb_types {
                let mvd_count: u32 = match smt {
                    0 => 1,
                    1 | 2 => 2,
                    3 => 4,
                    _ => 1, // unknown: treat as 1 to avoid stalling
                };
                for _ in 0..mvd_count {
                    let mvd_x = match reader.read_se() {
                        Ok(v) => v,
                        Err(_) => break 'outer,
                    };
                    let mvd_y = match reader.read_se() {
                        Ok(v) => v,
                        Err(_) => break 'outer,
                    };
                    sum_x += mvd_x;
                    sum_y += mvd_y;
                    total_mvd_count += 1;
                }
            }
            let avg_x = if total_mvd_count > 0 {
                sum_x / total_mvd_count
            } else {
                0
            };
            let avg_y = if total_mvd_count > 0 {
                sum_y / total_mvd_count
            } else {
                0
            };
            mbs.push(Macroblock {
                mb_addr,
                x,
                y,
                mb_type,
                skip: false,
                qp: slice_qp,
                mv_l0: Some(MotionVector::new(avg_x, avg_y)),
                mv_l1: None,
                ref_idx_l0: Some(first_ref.min(127) as i8),
                ref_idx_l1: None,
            });
            mb_addr += 1;
            if !try_skip_cavlc_residuals(&mut reader) {
                break;
            }
            continue;
        }

        // Non-P16x16/P16x8/P8x16/P8x8 non-skip MB: emit without MV, cannot parse further.
        mbs.push(Macroblock {
            mb_addr,
            x,
            y,
            mb_type,
            skip: false,
            qp: slice_qp,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        });
        mb_addr += 1;
        break;
    }

    // Fill remaining MBs with slice-type defaults (no MV data)
    let default_type = match slice_type {
        SliceType::I | SliceType::Si => MbType::I16x16,
        SliceType::B => MbType::B16x16,
        _ => MbType::PLuma,
    };
    while mb_addr < total_mbs {
        let x = (mb_addr % pic_width_in_mbs) * 16;
        let y = (mb_addr / pic_width_in_mbs) * 16;
        mbs.push(Macroblock {
            mb_addr,
            x,
            y,
            mb_type: default_type,
            skip: false,
            qp: slice_qp,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        });
        mb_addr += 1;
    }

    Ok(mbs)
}

/// Decode P-slice mb_type from raw ue value.
pub(super) fn decode_p_mb_type(raw: u32, slice_type: SliceType) -> MbType {
    if slice_type.is_intra() {
        // I slice mb_type table
        return match raw {
            0 => MbType::I4x4,
            25 => MbType::IPCM,
            _ => MbType::I16x16,
        };
    }
    // P slice: 0=P16x16, 1=P16x8, 2=P8x16, 3=P8x8, 4=P8x8ref0, 5+=I
    // B slice: 0=BDirect, 1=B16x16, 2=B16x8, 3=B8x16, 4=B8x8, 5+=I
    if slice_type.is_b() {
        return match raw {
            0 => MbType::BDirect,
            1 => MbType::B16x16,
            2..=22 => MbType::B16x8,
            23 => MbType::B8x8,
            _ => MbType::I16x16,
        };
    }
    match raw {
        0 => MbType::PLuma,     // P_L0_16x16
        1 | 2 => MbType::PLuma, // P_L0_L0_16x8 / 8x16
        3 | 4 => MbType::P8x8,
        _ => MbType::I16x16, // I MB in P slice (raw >= 5, subtract 5 for I table)
    }
}

// ---------------------------------------------------------------------------
// Minimal H.264 CABAC decoder — used for mb_skip_flag detection in P/B slices.
// Full residual decoding is not implemented; we only read skip flags so that
// CABAC-coded slices report correct skip/non-skip MB classification.
// ---------------------------------------------------------------------------

/// Build scaffold MBs based only on NAL unit type (no slice header parsed).
pub(super) fn build_scaffold_mbs(nal_type: NalUnitType, sps: &Sps, qp: i16) -> Vec<Macroblock> {
    let pic_width_in_mbs = sps.pic_width_in_mbs_minus1 + 1;
    let total_mbs = pic_width_in_mbs * (sps.pic_height_in_map_units_minus1 + 1);
    let mb_type = if nal_type == NalUnitType::IdrSlice {
        MbType::I16x16
    } else {
        MbType::PLuma
    };
    (0..total_mbs)
        .map(|mb_addr| Macroblock {
            mb_addr,
            x: (mb_addr % pic_width_in_mbs) * 16,
            y: (mb_addr / pic_width_in_mbs) * 16,
            mb_type,
            skip: false,
            qp,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        })
        .collect()
}

/// Build MBs using correct slice type (no MV data).
pub(super) fn build_typed_mbs(
    slice_type: SliceType,
    first_mb: u32,
    total_mbs: u32,
    width: u32,
    qp: i16,
) -> Vec<Macroblock> {
    let mb_type = match slice_type {
        SliceType::I | SliceType::Si => MbType::I16x16,
        SliceType::B => MbType::B16x16,
        _ => MbType::PLuma,
    };
    (first_mb..total_mbs)
        .map(|mb_addr| Macroblock {
            mb_addr,
            x: (mb_addr % width) * 16,
            y: (mb_addr / width) * 16,
            mb_type,
            skip: false,
            qp,
            mv_l0: None,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        })
        .collect()
}
