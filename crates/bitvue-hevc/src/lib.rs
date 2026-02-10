//! HEVC/H.265 bitstream parser for bitvue.
//!
//! This crate provides parsing capabilities for HEVC (High Efficiency Video Coding)
//! bitstreams, extracting NAL units, parameter sets, and slice headers.
//!
//! # Features
//!
//! - NAL unit parsing with start code detection
//! - VPS (Video Parameter Set) parsing
//! - SPS (Sequence Parameter Set) parsing
//! - PPS (Picture Parameter Set) parsing
//! - Slice header parsing
//! - Syntax tree extraction for visualization
//!
//! # Example
//!
//! ```ignore
//! use bitvue_hevc::{parse_hevc, HevcStream};
//!
//! let data: &[u8] = &[/* HEVC bitstream data */];
//! let stream = parse_hevc(data)?;
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
pub mod slice;
pub mod sps;
pub mod syntax;
pub mod vps;

pub use bitreader::{remove_emulation_prevention_bytes, BitReader};
pub use error::{HevcError, Result};
pub use frames::{
    extract_annex_b_frames, extract_frame_at_index, hevc_frame_to_unit_node,
    hevc_frames_to_unit_nodes, HevcFrame, HevcFrameType,
};
pub use nal::{
    find_nal_units, parse_nal_header, parse_nal_units, NalUnit, NalUnitHeader, NalUnitType,
};
pub use overlay_extraction::{
    extract_mv_grid, extract_partition_grid, extract_qp_grid, CodingTreeUnit, CodingUnit,
    IntraMode, MotionVector, PartMode, PredMode,
};
pub use pps::{parse_pps, Pps};
use serde::{Deserialize, Serialize};
pub use slice::{SliceHeader, SliceType};
pub use sps::{ChromaFormat, parse_sps, ProfileTierLevel, Sps};
use std::collections::HashMap;
pub use vps::Vps;

/// Parsed HEVC bitstream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HevcStream {
    /// All NAL units in the stream.
    pub nal_units: Vec<NalUnit>,
    /// Video Parameter Sets (indexed by vps_id).
    pub vps_map: HashMap<u8, Vps>,
    /// Sequence Parameter Sets (indexed by sps_id).
    pub sps_map: HashMap<u8, Sps>,
    /// Picture Parameter Sets (indexed by pps_id).
    pub pps_map: HashMap<u8, Pps>,
    /// Parsed slice headers.
    pub slices: Vec<ParsedSlice>,
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
}

impl HevcStream {
    /// Get VPS by ID.
    pub fn get_vps(&self, id: u8) -> Option<&Vps> {
        self.vps_map.get(&id)
    }

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

    /// Get frame rate from VPS/SPS timing info.
    pub fn frame_rate(&self) -> Option<f64> {
        // Check VPS first
        for vps in self.vps_map.values() {
            if let Some(ref timing) = vps.timing_info {
                if timing.time_scale > 0 && timing.num_units_in_tick > 0 {
                    return Some(timing.time_scale as f64 / timing.num_units_in_tick as f64);
                }
            }
        }
        // Fall back to SPS
        for sps in self.sps_map.values() {
            if let Some(ref vui) = sps.vui_parameters {
                if vui.timing_info_present_flag {
                    if let (Some(time_scale), Some(num_units)) =
                        (vui.time_scale, vui.num_units_in_tick)
                    {
                        if time_scale > 0 && num_units > 0 {
                            return Some(time_scale as f64 / num_units as f64);
                        }
                    }
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

    /// Count frames (VCL NAL units that start a new picture).
    pub fn frame_count(&self) -> usize {
        self.slices
            .iter()
            .filter(|s| s.header.first_slice_segment_in_pic_flag)
            .count()
    }

    /// Get all IDR frames.
    pub fn idr_frames(&self) -> Vec<&ParsedSlice> {
        self.slices
            .iter()
            .filter(|s| {
                let nal = &self.nal_units[s.nal_index];
                nal.header.nal_unit_type.is_idr()
            })
            .collect()
    }

    /// Get all IRAP frames (IDR, CRA, BLA).
    pub fn irap_frames(&self) -> Vec<&ParsedSlice> {
        self.slices
            .iter()
            .filter(|s| {
                let nal = &self.nal_units[s.nal_index];
                nal.header.nal_unit_type.is_irap()
            })
            .collect()
    }
}

/// Parse HEVC bitstream from Annex B byte stream.
pub fn parse_hevc(data: &[u8]) -> Result<HevcStream> {
    let nal_units = parse_nal_units(data)?;

    let mut vps_map = HashMap::new();
    let mut sps_map = HashMap::new();
    let mut pps_map = HashMap::new();
    let mut slices = Vec::new();

    // POC calculation state
    let mut prev_poc_msb: i32 = 0;
    let mut prev_poc_lsb: i32 = 0;

    for (nal_index, nal) in nal_units.iter().enumerate() {
        match nal.header.nal_unit_type {
            NalUnitType::VpsNut => {
                if let Ok(vps) = vps::parse_vps(&nal.payload) {
                    vps_map.insert(vps.vps_video_parameter_set_id, vps);
                }
            }
            NalUnitType::SpsNut => {
                if let Ok(sps) = sps::parse_sps(&nal.payload) {
                    sps_map.insert(sps.sps_seq_parameter_set_id, sps);
                }
            }
            NalUnitType::PpsNut => {
                if let Ok(pps) = pps::parse_pps(&nal.payload) {
                    pps_map.insert(pps.pps_pic_parameter_set_id, pps);
                }
            }
            nal_type if nal_type.is_vcl() => {
                // Parse slice header
                if let Ok(header) =
                    slice::parse_slice_header(&nal.payload, &sps_map, &pps_map, nal_type)
                {
                    // Calculate POC
                    let poc = if nal_type.is_idr() {
                        prev_poc_msb = 0;
                        prev_poc_lsb = 0;
                        0
                    } else {
                        // Get max_poc_lsb from SPS
                        let sps = pps_map
                            .get(&header.slice_pic_parameter_set_id)
                            .and_then(|pps| sps_map.get(&pps.pps_seq_parameter_set_id));

                        if let Some(sps) = sps {
                            let max_poc_lsb =
                                1 << sps.log2_max_pic_order_cnt_lsb_minus4.saturating_add(4);
                            let poc_lsb = header.slice_pic_order_cnt_lsb as i32;

                            let poc_msb = if poc_lsb < prev_poc_lsb
                                && (prev_poc_lsb - poc_lsb) >= (max_poc_lsb / 2)
                            {
                                prev_poc_msb + max_poc_lsb
                            } else if poc_lsb > prev_poc_lsb
                                && (poc_lsb - prev_poc_lsb) > (max_poc_lsb / 2)
                            {
                                prev_poc_msb - max_poc_lsb
                            } else {
                                prev_poc_msb
                            };

                            if nal_type.is_reference() {
                                prev_poc_msb = poc_msb;
                                prev_poc_lsb = poc_lsb;
                            }

                            poc_msb + poc_lsb
                        } else {
                            0
                        }
                    };

                    slices.push(ParsedSlice {
                        nal_index,
                        header,
                        poc,
                    });
                }
            }
            _ => {}
        }
    }

    Ok(HevcStream {
        nal_units,
        vps_map,
        sps_map,
        pps_map,
        slices,
    })
}

/// Quick parse to extract basic stream info without full parsing.
pub fn parse_hevc_quick(data: &[u8]) -> Result<HevcQuickInfo> {
    let nal_units = parse_nal_units(data)?;

    let mut info = HevcQuickInfo {
        nal_count: nal_units.len(),
        vps_count: 0,
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
            NalUnitType::VpsNut => info.vps_count += 1,
            NalUnitType::SpsNut => {
                info.sps_count += 1;
                // Parse SPS for dimensions
                if info.width.is_none() {
                    if let Ok(sps) = sps::parse_sps(&nal.payload) {
                        info.width = Some(sps.display_width());
                        info.height = Some(sps.display_height());
                        info.profile = Some(sps.profile_tier_level.general_profile_idc.idc());
                        info.level = Some(sps.profile_tier_level.general_level_idc);
                    }
                }
            }
            NalUnitType::PpsNut => info.pps_count += 1,
            nal_type if nal_type.is_idr() => {
                info.idr_count += 1;
                info.frame_count += 1;
            }
            nal_type if nal_type.is_vcl() => {
                info.frame_count += 1;
            }
            _ => {}
        }
    }

    Ok(info)
}

/// Quick stream info without full parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HevcQuickInfo {
    /// Total NAL unit count.
    pub nal_count: usize,
    /// VPS count.
    pub vps_count: usize,
    /// SPS count.
    pub sps_count: usize,
    /// PPS count.
    pub pps_count: usize,
    /// IDR frame count.
    pub idr_count: usize,
    /// Total frame count (VCL NAL units).
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


#[cfg(test)]
mod tests;

