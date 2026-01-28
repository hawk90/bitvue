//! HEVC syntax tree extraction for visualization.
//!
//! This module builds a hierarchical syntax tree from parsed HEVC structures,
//! suitable for UI display in tree views.

use crate::{HevcStream, NalUnitType};
use serde::{Deserialize, Serialize};

/// A node in the HEVC syntax tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxNode {
    /// Node name/label.
    pub name: String,
    /// Node value (if applicable).
    pub value: Option<String>,
    /// Bit offset in the stream.
    pub bit_offset: Option<u64>,
    /// Bit length of this element.
    pub bit_length: Option<u64>,
    /// Child nodes.
    pub children: Vec<SyntaxNode>,
    /// Node type for styling.
    pub node_type: SyntaxNodeType,
}

/// Type of syntax node for UI styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyntaxNodeType {
    /// Root node.
    Root,
    /// NAL unit container.
    NalUnit,
    /// Parameter set (VPS, SPS, PPS).
    ParameterSet,
    /// Slice header.
    SliceHeader,
    /// Field/value.
    Field,
    /// Array/list.
    Array,
    /// Structure.
    Structure,
}

impl SyntaxNode {
    /// Create a new syntax node.
    pub fn new(name: impl Into<String>, node_type: SyntaxNodeType) -> Self {
        Self {
            name: name.into(),
            value: None,
            bit_offset: None,
            bit_length: None,
            children: Vec::new(),
            node_type,
        }
    }

    /// Create a field node with a value.
    pub fn field(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: Some(value.into()),
            bit_offset: None,
            bit_length: None,
            children: Vec::new(),
            node_type: SyntaxNodeType::Field,
        }
    }

    /// Add a child node.
    pub fn add_child(&mut self, child: SyntaxNode) {
        self.children.push(child);
    }

    /// Set bit position.
    pub fn with_position(mut self, offset: u64, length: u64) -> Self {
        self.bit_offset = Some(offset);
        self.bit_length = Some(length);
        self
    }
}

/// Build a syntax tree from a parsed HEVC stream.
pub fn build_syntax_tree(stream: &HevcStream) -> SyntaxNode {
    let mut root = SyntaxNode::new("HEVC Bitstream", SyntaxNodeType::Root);

    // Add stream info
    if let Some((width, height)) = stream.dimensions() {
        root.add_child(SyntaxNode::field(
            "Resolution",
            format!("{}x{}", width, height),
        ));
    }
    if let Some(fps) = stream.frame_rate() {
        root.add_child(SyntaxNode::field("Frame Rate", format!("{:.2} fps", fps)));
    }

    // Add NAL units
    let mut nal_units_node = SyntaxNode::new("NAL Units", SyntaxNodeType::Array);

    for (i, nal) in stream.nal_units.iter().enumerate() {
        let nal_name = format!(
            "[{}] {} (offset: {}, size: {})",
            i,
            nal.header.nal_unit_type.name(),
            nal.offset,
            nal.size
        );
        let mut nal_node = SyntaxNode::new(nal_name, SyntaxNodeType::NalUnit);

        // Add NAL header fields
        let mut header_node = SyntaxNode::new("NAL Header", SyntaxNodeType::Structure);
        header_node.add_child(SyntaxNode::field(
            "nal_unit_type",
            format!(
                "{:?} ({})",
                nal.header.nal_unit_type, nal.header.nal_unit_type as u8
            ),
        ));
        header_node.add_child(SyntaxNode::field(
            "nuh_layer_id",
            nal.header.nuh_layer_id.to_string(),
        ));
        header_node.add_child(SyntaxNode::field(
            "nuh_temporal_id_plus1",
            nal.header.nuh_temporal_id_plus1.to_string(),
        ));
        header_node.add_child(SyntaxNode::field(
            "temporal_id",
            nal.header.temporal_id().to_string(),
        ));
        nal_node.add_child(header_node);

        // Add parameter set details
        match nal.header.nal_unit_type {
            NalUnitType::VpsNut => {
                if let Some(vps) = stream.vps_map.values().next() {
                    nal_node.add_child(build_vps_tree(vps));
                }
            }
            NalUnitType::SpsNut => {
                if let Some(sps) = stream.sps_map.values().next() {
                    nal_node.add_child(build_sps_tree(sps));
                }
            }
            NalUnitType::PpsNut => {
                if let Some(pps) = stream.pps_map.values().next() {
                    nal_node.add_child(build_pps_tree(pps));
                }
            }
            _ => {}
        }

        nal_units_node.add_child(nal_node);
    }

    root.add_child(nal_units_node);

    // Add slices summary
    let mut slices_node = SyntaxNode::new(
        format!("Slices ({})", stream.slices.len()),
        SyntaxNodeType::Array,
    );

    for slice in &stream.slices {
        let slice_name = format!(
            "POC {} - {} slice",
            slice.poc,
            slice.header.slice_type.name()
        );
        let mut slice_node = SyntaxNode::new(slice_name, SyntaxNodeType::SliceHeader);

        slice_node.add_child(SyntaxNode::field("poc", slice.poc.to_string()));
        slice_node.add_child(SyntaxNode::field(
            "slice_type",
            slice.header.slice_type.name().to_string(),
        ));
        slice_node.add_child(SyntaxNode::field(
            "first_slice_segment_in_pic_flag",
            slice.header.first_slice_segment_in_pic_flag.to_string(),
        ));
        slice_node.add_child(SyntaxNode::field(
            "slice_qp_delta",
            slice.header.slice_qp_delta.to_string(),
        ));

        if slice.header.slice_type.is_inter() {
            slice_node.add_child(SyntaxNode::field(
                "num_ref_idx_l0_active",
                slice.header.num_ref_idx_l0_active().to_string(),
            ));
            if slice.header.slice_type == crate::SliceType::B {
                slice_node.add_child(SyntaxNode::field(
                    "num_ref_idx_l1_active",
                    slice.header.num_ref_idx_l1_active().to_string(),
                ));
            }
        }

        slices_node.add_child(slice_node);
    }

    root.add_child(slices_node);

    root
}

/// Build syntax tree for VPS.
fn build_vps_tree(vps: &crate::Vps) -> SyntaxNode {
    let mut node = SyntaxNode::new("Video Parameter Set", SyntaxNodeType::ParameterSet);

    node.add_child(SyntaxNode::field(
        "vps_video_parameter_set_id",
        vps.vps_video_parameter_set_id.to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "vps_max_layers_minus1",
        vps.vps_max_layers_minus1.to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "vps_max_sub_layers_minus1",
        vps.vps_max_sub_layers_minus1.to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "vps_temporal_id_nesting_flag",
        vps.vps_temporal_id_nesting_flag.to_string(),
    ));

    // Profile/Tier/Level
    let mut ptl_node = SyntaxNode::new("profile_tier_level", SyntaxNodeType::Structure);
    ptl_node.add_child(SyntaxNode::field(
        "general_profile_idc",
        format!(
            "{} ({})",
            vps.profile_tier_level.general_profile_idc,
            vps.profile_name()
        ),
    ));
    ptl_node.add_child(SyntaxNode::field(
        "general_tier_flag",
        format!(
            "{} ({})",
            vps.profile_tier_level.general_tier_flag,
            vps.tier_name()
        ),
    ));
    ptl_node.add_child(SyntaxNode::field(
        "general_level_idc",
        format!(
            "{} (Level {:.1})",
            vps.profile_tier_level.general_level_idc,
            vps.level()
        ),
    ));
    node.add_child(ptl_node);

    // Timing info
    if let Some(ref timing) = vps.timing_info {
        let mut timing_node = SyntaxNode::new("timing_info", SyntaxNodeType::Structure);
        timing_node.add_child(SyntaxNode::field(
            "num_units_in_tick",
            timing.num_units_in_tick.to_string(),
        ));
        timing_node.add_child(SyntaxNode::field(
            "time_scale",
            timing.time_scale.to_string(),
        ));
        if timing.time_scale > 0 && timing.num_units_in_tick > 0 {
            let fps = timing.time_scale as f64 / timing.num_units_in_tick as f64;
            timing_node.add_child(SyntaxNode::field("frame_rate", format!("{:.2} fps", fps)));
        }
        node.add_child(timing_node);
    }

    node
}

/// Build syntax tree for SPS.
fn build_sps_tree(sps: &crate::Sps) -> SyntaxNode {
    let mut node = SyntaxNode::new("Sequence Parameter Set", SyntaxNodeType::ParameterSet);

    node.add_child(SyntaxNode::field(
        "sps_seq_parameter_set_id",
        sps.sps_seq_parameter_set_id.to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "chroma_format_idc",
        format!("{:?}", sps.chroma_format_idc),
    ));
    node.add_child(SyntaxNode::field(
        "pic_width_in_luma_samples",
        sps.pic_width_in_luma_samples.to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "pic_height_in_luma_samples",
        sps.pic_height_in_luma_samples.to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "display_size",
        format!("{}x{}", sps.display_width(), sps.display_height()),
    ));
    node.add_child(SyntaxNode::field(
        "bit_depth_luma",
        sps.bit_depth_luma().to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "bit_depth_chroma",
        sps.bit_depth_chroma().to_string(),
    ));
    node.add_child(SyntaxNode::field("ctb_size", sps.ctb_size().to_string()));
    node.add_child(SyntaxNode::field(
        "pic_size_in_ctbs",
        format!("{}x{}", sps.pic_width_in_ctbs(), sps.pic_height_in_ctbs()),
    ));

    // Features
    let mut features_node = SyntaxNode::new("features", SyntaxNodeType::Structure);
    features_node.add_child(SyntaxNode::field(
        "amp_enabled_flag",
        sps.amp_enabled_flag.to_string(),
    ));
    features_node.add_child(SyntaxNode::field(
        "sample_adaptive_offset_enabled_flag",
        sps.sample_adaptive_offset_enabled_flag.to_string(),
    ));
    features_node.add_child(SyntaxNode::field(
        "sps_temporal_mvp_enabled_flag",
        sps.sps_temporal_mvp_enabled_flag.to_string(),
    ));
    features_node.add_child(SyntaxNode::field(
        "strong_intra_smoothing_enabled_flag",
        sps.strong_intra_smoothing_enabled_flag.to_string(),
    ));
    node.add_child(features_node);

    node
}

/// Build syntax tree for PPS.
fn build_pps_tree(pps: &crate::Pps) -> SyntaxNode {
    let mut node = SyntaxNode::new("Picture Parameter Set", SyntaxNodeType::ParameterSet);

    node.add_child(SyntaxNode::field(
        "pps_pic_parameter_set_id",
        pps.pps_pic_parameter_set_id.to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "pps_seq_parameter_set_id",
        pps.pps_seq_parameter_set_id.to_string(),
    ));
    node.add_child(SyntaxNode::field("init_qp", pps.init_qp().to_string()));

    // Prediction modes
    let mut pred_node = SyntaxNode::new("prediction", SyntaxNodeType::Structure);
    pred_node.add_child(SyntaxNode::field(
        "constrained_intra_pred_flag",
        pps.constrained_intra_pred_flag.to_string(),
    ));
    pred_node.add_child(SyntaxNode::field(
        "weighted_pred_flag",
        pps.weighted_pred_flag.to_string(),
    ));
    pred_node.add_child(SyntaxNode::field(
        "weighted_bipred_flag",
        pps.weighted_bipred_flag.to_string(),
    ));
    node.add_child(pred_node);

    // Parallelism
    let mut parallel_node = SyntaxNode::new("parallelism", SyntaxNodeType::Structure);
    parallel_node.add_child(SyntaxNode::field(
        "tiles_enabled_flag",
        pps.tiles_enabled_flag.to_string(),
    ));
    if let Some(ref tile_config) = pps.tile_config {
        parallel_node.add_child(SyntaxNode::field(
            "num_tiles",
            format!("{}x{}", tile_config.num_columns(), tile_config.num_rows()),
        ));
    }
    parallel_node.add_child(SyntaxNode::field(
        "entropy_coding_sync_enabled_flag",
        pps.entropy_coding_sync_enabled_flag.to_string(),
    ));
    node.add_child(parallel_node);

    // Transform
    let mut transform_node = SyntaxNode::new("transform", SyntaxNodeType::Structure);
    transform_node.add_child(SyntaxNode::field(
        "transform_skip_enabled_flag",
        pps.transform_skip_enabled_flag.to_string(),
    ));
    transform_node.add_child(SyntaxNode::field(
        "transquant_bypass_enabled_flag",
        pps.transquant_bypass_enabled_flag.to_string(),
    ));
    node.add_child(transform_node);

    node
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_node_creation() {
        let node = SyntaxNode::new("test", SyntaxNodeType::Root);
        assert_eq!(node.name, "test");
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_field_node() {
        let node = SyntaxNode::field("width", "1920");
        assert_eq!(node.name, "width");
        assert_eq!(node.value, Some("1920".to_string()));
        assert_eq!(node.node_type, SyntaxNodeType::Field);
    }
}
