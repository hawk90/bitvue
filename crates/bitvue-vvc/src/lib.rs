//! VVC/H.266 bitstream parser for bitvue.
//!
//! This crate provides parsing capabilities for VVC (Versatile Video Coding)
//! bitstreams, including new features like dual tree, ALF, LMCS, and GDR.
//!
//! # Features
//!
//! - NAL unit parsing with VVC-specific types
//! - SPS parsing with dual tree and advanced tool flags
//! - PPS parsing
//! - Syntax tree extraction for visualization
//!
//! # Example
//!
//! ```ignore
//! use bitvue_vvc::{parse_vvc, VvcStream};
//!
//! let data: &[u8] = &[/* VVC bitstream data */];
//! let stream = parse_vvc(data)?;
//!
//! for nal in &stream.nal_units {
//!     println!("NAL type: {:?}", nal.nal_type());
//! }
//! ```

pub mod bitreader;
pub mod error;
pub mod nal;
pub mod overlay_extraction;
pub mod pps;
pub mod sps;
pub mod syntax;

pub use bitreader::{remove_emulation_prevention_bytes, BitReader};
pub use error::{Result, VvcError};
pub use nal::{
    find_nal_units, parse_nal_header, parse_nal_units, NalUnit, NalUnitHeader, NalUnitType,
};
pub use overlay_extraction::{
    extract_mv_grid, extract_partition_grid, extract_qp_grid, CodingTreeUnit, CodingUnit,
    MotionVector, PredMode, SplitMode,
};
pub use pps::{parse_pps, Pps};
pub use sps::{
    parse_sps, AlfConfig, ChromaFormat, DualTreeConfig, LmcsConfig, Profile, ProfileTierLevel, Sps,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed VVC bitstream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VvcStream {
    /// All NAL units in the stream.
    pub nal_units: Vec<NalUnit>,
    /// Sequence Parameter Sets (indexed by sps_id).
    pub sps_map: HashMap<u8, Sps>,
    /// Picture Parameter Sets (indexed by pps_id).
    pub pps_map: HashMap<u8, Pps>,
}

impl VvcStream {
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

    /// Get bit depth from SPS.
    pub fn bit_depth(&self) -> Option<u8> {
        self.sps_map.values().next().map(|sps| sps.bit_depth())
    }

    /// Get chroma format from SPS.
    pub fn chroma_format(&self) -> Option<ChromaFormat> {
        self.sps_map
            .values()
            .next()
            .map(|sps| sps.sps_chroma_format_idc)
    }

    /// Count frames (VCL NAL units).
    pub fn frame_count(&self) -> usize {
        self.nal_units.iter().filter(|n| n.is_vcl()).count()
    }

    /// Get all IDR frames.
    pub fn idr_frames(&self) -> Vec<&NalUnit> {
        self.nal_units
            .iter()
            .filter(|n| n.header.nal_unit_type.is_idr())
            .collect()
    }

    /// Get all IRAP frames (IDR, CRA, GDR).
    pub fn irap_frames(&self) -> Vec<&NalUnit> {
        self.nal_units
            .iter()
            .filter(|n| n.header.nal_unit_type.is_irap())
            .collect()
    }

    /// Get all GDR frames.
    pub fn gdr_frames(&self) -> Vec<&NalUnit> {
        self.nal_units
            .iter()
            .filter(|n| n.header.nal_unit_type.is_gdr())
            .collect()
    }

    /// Check if stream uses GDR.
    pub fn uses_gdr(&self) -> bool {
        self.sps_map.values().any(|sps| sps.sps_gdr_enabled_flag)
    }

    /// Check if stream uses dual tree.
    pub fn uses_dual_tree(&self) -> bool {
        self.sps_map.values().any(|sps| sps.has_dual_tree_intra())
    }

    /// Check if stream uses ALF.
    pub fn uses_alf(&self) -> bool {
        self.sps_map.values().any(|sps| sps.alf.alf_enabled_flag)
    }

    /// Check if stream uses LMCS.
    pub fn uses_lmcs(&self) -> bool {
        self.sps_map.values().any(|sps| sps.lmcs.lmcs_enabled_flag)
    }
}

/// Parse VVC bitstream from Annex B byte stream.
pub fn parse_vvc(data: &[u8]) -> Result<VvcStream> {
    let nal_units = parse_nal_units(data)?;

    let mut sps_map = HashMap::new();
    let mut pps_map = HashMap::new();

    for nal in &nal_units {
        match nal.header.nal_unit_type {
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
            _ => {}
        }
    }

    Ok(VvcStream {
        nal_units,
        sps_map,
        pps_map,
    })
}

/// Quick parse to extract basic stream info.
pub fn parse_vvc_quick(data: &[u8]) -> Result<VvcQuickInfo> {
    let nal_units = parse_nal_units(data)?;

    let mut info = VvcQuickInfo {
        nal_count: nal_units.len(),
        sps_count: 0,
        pps_count: 0,
        idr_count: 0,
        gdr_count: 0,
        frame_count: 0,
        width: None,
        height: None,
        profile: None,
        level: None,
        uses_gdr: false,
        uses_dual_tree: false,
        uses_alf: false,
        uses_lmcs: false,
    };

    for nal in &nal_units {
        match nal.header.nal_unit_type {
            NalUnitType::SpsNut => {
                info.sps_count += 1;
                if info.width.is_none() {
                    if let Ok(sps) = sps::parse_sps(&nal.payload) {
                        info.width = Some(sps.display_width());
                        info.height = Some(sps.display_height());
                        info.profile = Some(sps.profile_tier_level.general_profile_idc.idc());
                        info.level = Some(sps.profile_tier_level.general_level_idc);
                        info.uses_gdr = sps.sps_gdr_enabled_flag;
                        info.uses_dual_tree = sps.has_dual_tree_intra();
                        info.uses_alf = sps.alf.alf_enabled_flag;
                        info.uses_lmcs = sps.lmcs.lmcs_enabled_flag;
                    }
                }
            }
            NalUnitType::PpsNut => info.pps_count += 1,
            nal_type if nal_type.is_idr() => {
                info.idr_count += 1;
                info.frame_count += 1;
            }
            nal_type if nal_type.is_gdr() => {
                info.gdr_count += 1;
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
pub struct VvcQuickInfo {
    /// Total NAL unit count.
    pub nal_count: usize,
    /// SPS count.
    pub sps_count: usize,
    /// PPS count.
    pub pps_count: usize,
    /// IDR frame count.
    pub idr_count: usize,
    /// GDR frame count.
    pub gdr_count: usize,
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
    /// Uses GDR.
    pub uses_gdr: bool,
    /// Uses dual tree.
    pub uses_dual_tree: bool,
    /// Uses ALF.
    pub uses_alf: bool,
    /// Uses LMCS.
    pub uses_lmcs: bool,
}

#[cfg(test)]
mod tests;
