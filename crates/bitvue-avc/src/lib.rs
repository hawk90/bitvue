//! H.264/AVC bitstream parser for bitvue.
//!
//! This crate provides parsing capabilities for H.264/AVC (Advanced Video Coding)
//! bitstreams, extracting NAL units, parameter sets, and slice headers.
//!
//! # Features
//!
//! - NAL unit parsing with start code detection
//! - SPS (Sequence Parameter Set) parsing
//! - PPS (Picture Parameter Set) parsing
//! - Slice header parsing
//! - SEI (Supplemental Enhancement Information) parsing
//! - Syntax tree extraction for visualization
//!
//! # Example
//!
//! ```ignore
//! use bitvue_avc::{parse_avc, AvcStream};
//!
//! let data: &[u8] = &[/* AVC bitstream data */];
//! let stream = parse_avc(data)?;
//!
//! for nal in &stream.nal_units {
//!     println!("NAL type: {:?}", nal.nal_type());
//! }
//! ```

pub mod bitreader;
pub mod error;
pub mod frames;
pub mod nal;
pub mod overlay_extraction;
pub mod pps;
pub mod sei;
pub mod slice;
pub mod sps;

pub use bitreader::{remove_emulation_prevention_bytes, BitReader};
pub use error::{AvcError, Result};
pub use frames::{
    avc_frame_to_unit_node, avc_frames_to_unit_nodes, extract_annex_b_frames,
    extract_frame_at_index, AvcFrame, AvcFrameType,
};
pub use nal::{
    find_nal_units, parse_nal_header, parse_nal_units, NalUnit, NalUnitHeader, NalUnitType,
};
pub use overlay_extraction::{
    extract_mv_grid, extract_partition_grid, extract_qp_grid, Macroblock, MbType, MotionVector,
};
pub use pps::{parse_pps, Pps};
pub use sei::{parse_sei, SeiMessage, SeiPayloadType};
pub use slice::{parse_slice_header, SliceHeader, SliceType};
pub use sps::{parse_sps, ChromaFormat, ProfileIdc, Sps};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed H.264/AVC bitstream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvcStream {
    /// All NAL units in the stream.
    pub nal_units: Vec<NalUnit>,
    /// Sequence Parameter Sets (indexed by sps_id).
    pub sps_map: HashMap<u8, Sps>,
    /// Picture Parameter Sets (indexed by pps_id).
    pub pps_map: HashMap<u8, Pps>,
    /// Parsed slice headers.
    pub slices: Vec<ParsedSlice>,
    /// SEI messages.
    pub sei_messages: Vec<SeiMessage>,
}

/// A parsed slice with its header and associated metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSlice {
    /// Index of the NAL unit containing this slice.
    pub nal_index: usize,
    /// Parsed slice header.
    pub header: SliceHeader,
    /// POC (Picture Order Count).
    pub poc: i32,
    /// Frame number.
    pub frame_num: u32,
}

impl AvcStream {
    /// Get SPS by ID.
    pub fn get_sps(&self, id: u8) -> Option<&Sps> {
        self.sps_map.get(&id)
    }

    /// Get PPS by ID.
    pub fn get_pps(&self, id: u8) -> Option<&Pps> {
        self.pps_map.get(&id)
    }

    /// Get video dimensions from SPS.
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.sps_map
            .values()
            .next()
            .map(|sps| (sps.display_width(), sps.display_height()))
    }

    /// Get frame rate from SPS timing info.
    pub fn frame_rate(&self) -> Option<f64> {
        for sps in self.sps_map.values() {
            if let Some(ref vui) = sps.vui_parameters {
                if vui.timing_info_present_flag && vui.time_scale > 0 && vui.num_units_in_tick > 0 {
                    // H.264 frame rate = time_scale / (2 * num_units_in_tick) for interlaced
                    // or time_scale / num_units_in_tick for progressive
                    let fps = vui.time_scale as f64 / (2.0 * vui.num_units_in_tick as f64);
                    return Some(fps);
                }
            }
        }
        None
    }

    /// Get bit depth for luma.
    pub fn bit_depth_luma(&self) -> Option<u8> {
        self.sps_map.values().next().map(|sps| sps.bit_depth_luma())
    }

    /// Get bit depth for chroma.
    pub fn bit_depth_chroma(&self) -> Option<u8> {
        self.sps_map
            .values()
            .next()
            .map(|sps| sps.bit_depth_chroma())
    }

    /// Get chroma format.
    pub fn chroma_format(&self) -> Option<ChromaFormat> {
        self.sps_map
            .values()
            .next()
            .map(|sps| sps.chroma_format_idc)
    }

    /// Count frames (slices that start a new picture).
    pub fn frame_count(&self) -> usize {
        self.slices
            .iter()
            .filter(|s| s.header.first_mb_in_slice == 0)
            .count()
    }

    /// Get all IDR frames.
    pub fn idr_frames(&self) -> Vec<&ParsedSlice> {
        self.slices
            .iter()
            .filter(|s| {
                let nal = &self.nal_units[s.nal_index];
                matches!(nal.header.nal_unit_type, NalUnitType::IdrSlice)
            })
            .collect()
    }

    /// Get profile string.
    pub fn profile_string(&self) -> Option<String> {
        self.sps_map
            .values()
            .next()
            .map(|sps| sps.profile_idc.to_string())
    }

    /// Get level string (e.g., "4.1").
    pub fn level_string(&self) -> Option<String> {
        self.sps_map.values().next().map(|sps| {
            let major = sps.level_idc / 10;
            let minor = sps.level_idc % 10;
            format!("{}.{}", major, minor)
        })
    }
}

/// Parse H.264/AVC bitstream from Annex B byte stream.
pub fn parse_avc(data: &[u8]) -> Result<AvcStream> {
    let nal_units = parse_nal_units(data)?;

    let mut sps_map = HashMap::new();
    let mut pps_map = HashMap::new();
    let mut slices = Vec::new();
    let mut sei_messages = Vec::new();

    // POC calculation state
    let mut prev_poc_msb: i32 = 0;
    let mut prev_poc_lsb: i32 = 0;
    let mut prev_frame_num: u32 = 0;
    let mut prev_frame_num_offset: i32 = 0;

    for (nal_index, nal) in nal_units.iter().enumerate() {
        match nal.header.nal_unit_type {
            NalUnitType::Sps => {
                if let Ok(sps) = sps::parse_sps(&nal.payload) {
                    sps_map.insert(sps.seq_parameter_set_id, sps);
                }
            }
            NalUnitType::Pps => {
                if let Ok(pps) = pps::parse_pps(&nal.payload) {
                    pps_map.insert(pps.pic_parameter_set_id, pps);
                }
            }
            NalUnitType::Sei => {
                if let Ok(messages) = parse_sei(&nal.payload) {
                    sei_messages.extend(messages);
                }
            }
            NalUnitType::IdrSlice | NalUnitType::NonIdrSlice => {
                // Parse slice header
                if let Ok(header) = slice::parse_slice_header(
                    &nal.payload,
                    &sps_map,
                    &pps_map,
                    nal.header.nal_unit_type,
                    nal.header.nal_ref_idc,
                ) {
                    // Calculate POC
                    let sps = pps_map
                        .get(&header.pic_parameter_set_id)
                        .and_then(|pps| sps_map.get(&pps.seq_parameter_set_id));

                    let poc = if let Some(sps) = sps {
                        calculate_poc(
                            sps,
                            &header,
                            nal.header.nal_unit_type == NalUnitType::IdrSlice,
                            &mut prev_poc_msb,
                            &mut prev_poc_lsb,
                            &mut prev_frame_num,
                            &mut prev_frame_num_offset,
                        )
                    } else {
                        0
                    };

                    slices.push(ParsedSlice {
                        nal_index,
                        header: header.clone(),
                        poc,
                        frame_num: header.frame_num,
                    });
                }
            }
            _ => {}
        }
    }

    Ok(AvcStream {
        nal_units,
        sps_map,
        pps_map,
        slices,
        sei_messages,
    })
}

/// Calculate Picture Order Count for H.264.
fn calculate_poc(
    sps: &Sps,
    header: &SliceHeader,
    is_idr: bool,
    prev_poc_msb: &mut i32,
    prev_poc_lsb: &mut i32,
    prev_frame_num: &mut u32,
    prev_frame_num_offset: &mut i32,
) -> i32 {
    match sps.pic_order_cnt_type {
        0 => {
            // POC type 0
            if is_idr {
                *prev_poc_msb = 0;
                *prev_poc_lsb = 0;
                return 0;
            }

            let max_poc_lsb = 1i32 << (sps.log2_max_pic_order_cnt_lsb_minus4 + 4);
            let poc_lsb = header.pic_order_cnt_lsb as i32;

            let poc_msb =
                if poc_lsb < *prev_poc_lsb && (*prev_poc_lsb - poc_lsb) >= (max_poc_lsb / 2) {
                    *prev_poc_msb + max_poc_lsb
                } else if poc_lsb > *prev_poc_lsb && (poc_lsb - *prev_poc_lsb) > (max_poc_lsb / 2) {
                    *prev_poc_msb - max_poc_lsb
                } else {
                    *prev_poc_msb
                };

            *prev_poc_msb = poc_msb;
            *prev_poc_lsb = poc_lsb;

            poc_msb + poc_lsb
        }
        1 => {
            // POC type 1 - frame_num based
            let max_frame_num = 1u32 << (sps.log2_max_frame_num_minus4 + 4);

            let frame_num_offset = if is_idr {
                0
            } else if *prev_frame_num > header.frame_num {
                *prev_frame_num_offset + max_frame_num as i32
            } else {
                *prev_frame_num_offset
            };

            *prev_frame_num = header.frame_num;
            *prev_frame_num_offset = frame_num_offset;

            // Simplified POC type 1 calculation
            (frame_num_offset + header.frame_num as i32) * 2 + header.delta_pic_order_cnt[0]
        }
        2 => {
            // POC type 2 - display order equals decode order
            let max_frame_num = 1u32 << (sps.log2_max_frame_num_minus4 + 4);

            let frame_num_offset = if is_idr {
                0
            } else if *prev_frame_num > header.frame_num {
                *prev_frame_num_offset + max_frame_num as i32
            } else {
                *prev_frame_num_offset
            };

            *prev_frame_num = header.frame_num;
            *prev_frame_num_offset = frame_num_offset;

            if is_idr {
                0
            } else {
                (frame_num_offset + header.frame_num as i32) * 2
            }
        }
        _ => 0,
    }
}

/// Quick parse to extract basic stream info without full parsing.
pub fn parse_avc_quick(data: &[u8]) -> Result<AvcQuickInfo> {
    let nal_units = parse_nal_units(data)?;

    let mut info = AvcQuickInfo {
        nal_count: nal_units.len(),
        sps_count: 0,
        pps_count: 0,
        idr_count: 0,
        frame_count: 0,
        width: None,
        height: None,
        profile: None,
        level: None,
    };

    for nal in &nal_units {
        match nal.header.nal_unit_type {
            NalUnitType::Sps => {
                info.sps_count += 1;
                if info.width.is_none() {
                    if let Ok(sps) = sps::parse_sps(&nal.payload) {
                        info.width = Some(sps.display_width());
                        info.height = Some(sps.display_height());
                        info.profile = Some(sps.profile_idc as u8);
                        info.level = Some(sps.level_idc);
                    }
                }
            }
            NalUnitType::Pps => info.pps_count += 1,
            NalUnitType::IdrSlice => {
                info.idr_count += 1;
                info.frame_count += 1;
            }
            NalUnitType::NonIdrSlice => {
                info.frame_count += 1;
            }
            _ => {}
        }
    }

    Ok(info)
}

/// Quick stream info without full parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvcQuickInfo {
    /// Total NAL unit count.
    pub nal_count: usize,
    /// SPS count.
    pub sps_count: usize,
    /// PPS count.
    pub pps_count: usize,
    /// IDR frame count.
    pub idr_count: usize,
    /// Total frame count.
    pub frame_count: usize,
    /// Video width.
    pub width: Option<u32>,
    /// Video height.
    pub height: Option<u32>,
    /// Profile IDC.
    pub profile: Option<u8>,
    /// Level IDC.
    pub level: Option<u8>,
}

// Test-only exports for coverage testing
#[cfg(test)]
pub mod test_exports {
    use super::*;

    // Export calculate_poc for testing (0% coverage)
    pub fn test_calculate_poc(
        sps: &Sps,
        header: &SliceHeader,
        is_idr: bool,
        prev_poc_msb: &mut i32,
        prev_poc_lsb: &mut i32,
        prev_frame_num: &mut u32,
        prev_frame_num_offset: &mut i32,
    ) -> i32 {
        calculate_poc(
            sps,
            header,
            is_idr,
            prev_poc_msb,
            prev_poc_lsb,
            prev_frame_num,
            prev_frame_num_offset,
        )
    }
}

#[cfg(test)]
mod tests;
