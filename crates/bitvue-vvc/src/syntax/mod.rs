//! VVC syntax tree extraction for visualization.

use crate::{NalUnitType, VvcStream};
use serde::{Deserialize, Serialize};

/// A node in the VVC syntax tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxNode {
    pub name: String,
    pub value: Option<String>,
    pub bit_offset: Option<u64>,
    pub bit_length: Option<u64>,
    pub children: Vec<SyntaxNode>,
    pub node_type: SyntaxNodeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyntaxNodeType {
    Root,
    NalUnit,
    ParameterSet,
    PictureHeader,
    SliceHeader,
    Field,
    Array,
    Structure,
}

impl SyntaxNode {
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

    pub fn add_child(&mut self, child: SyntaxNode) {
        self.children.push(child);
    }
}

/// Build a syntax tree from a parsed VVC stream.
pub fn build_syntax_tree(stream: &VvcStream) -> SyntaxNode {
    let mut root = SyntaxNode::new("VVC Bitstream", SyntaxNodeType::Root);

    // Add stream info
    if let Some((width, height)) = stream.dimensions() {
        root.add_child(SyntaxNode::field("Resolution", format!("{}x{}", width, height)));
    }
    if let Some(depth) = stream.bit_depth() {
        root.add_child(SyntaxNode::field("Bit Depth", depth.to_string()));
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
            format!("{:?} ({})", nal.header.nal_unit_type, nal.header.nal_unit_type as u8),
        ));
        header_node.add_child(SyntaxNode::field(
            "nuh_layer_id",
            nal.header.nuh_layer_id.to_string(),
        ));
        header_node.add_child(SyntaxNode::field(
            "nuh_temporal_id_plus1",
            nal.header.nuh_temporal_id_plus1.to_string(),
        ));
        nal_node.add_child(header_node);

        // Add parameter set details
        match nal.header.nal_unit_type {
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

    root
}

fn build_sps_tree(sps: &crate::Sps) -> SyntaxNode {
    let mut node = SyntaxNode::new("Sequence Parameter Set", SyntaxNodeType::ParameterSet);

    node.add_child(SyntaxNode::field(
        "sps_seq_parameter_set_id",
        sps.sps_seq_parameter_set_id.to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "profile",
        sps.profile_name().to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "level",
        format!("{:.1}", sps.level()),
    ));
    node.add_child(SyntaxNode::field(
        "chroma_format",
        format!("{:?}", sps.sps_chroma_format_idc),
    ));
    node.add_child(SyntaxNode::field(
        "bit_depth",
        sps.bit_depth().to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "ctu_size",
        sps.ctu_size().to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "resolution",
        format!("{}x{}", sps.sps_pic_width_max_in_luma_samples, sps.sps_pic_height_max_in_luma_samples),
    ));

    // VVC-specific features
    let mut features_node = SyntaxNode::new("VVC Features", SyntaxNodeType::Structure);
    features_node.add_child(SyntaxNode::field("gdr_enabled", sps.sps_gdr_enabled_flag.to_string()));
    features_node.add_child(SyntaxNode::field("dual_tree_intra", sps.has_dual_tree_intra().to_string()));
    features_node.add_child(SyntaxNode::field("alf_enabled", sps.alf.alf_enabled_flag.to_string()));
    features_node.add_child(SyntaxNode::field("lmcs_enabled", sps.lmcs.lmcs_enabled_flag.to_string()));
    features_node.add_child(SyntaxNode::field("ibc_enabled", sps.sps_ibc_enabled_flag.to_string()));
    features_node.add_child(SyntaxNode::field("affine_enabled", sps.sps_affine_enabled_flag.to_string()));
    node.add_child(features_node);

    node
}

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
    node.add_child(SyntaxNode::field(
        "weighted_pred",
        pps.pps_weighted_pred_flag.to_string(),
    ));
    node.add_child(SyntaxNode::field(
        "weighted_bipred",
        pps.pps_weighted_bipred_flag.to_string(),
    ));

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
    }
}
