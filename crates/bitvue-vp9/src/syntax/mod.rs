//! VP9 syntax tree extraction for visualization.
//!
//! This module builds a hierarchical syntax tree from parsed VP9 structures,
//! suitable for UI display in tree views.

use crate::{FrameHeader, SuperframeIndex, Vp9Stream};
use serde::{Deserialize, Serialize};

/// A node in the VP9 syntax tree.
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
    /// Superframe container.
    Superframe,
    /// Frame.
    Frame,
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

/// Build a syntax tree from a parsed VP9 stream.
pub fn build_syntax_tree(stream: &Vp9Stream) -> SyntaxNode {
    let mut root = SyntaxNode::new("VP9 Bitstream", SyntaxNodeType::Root);

    // Add stream info
    if let Some((width, height)) = stream.dimensions() {
        root.add_child(SyntaxNode::field("Resolution", format!("{}x{}", width, height)));
    }
    root.add_child(SyntaxNode::field("Frame Count", stream.frames.len().to_string()));
    root.add_child(SyntaxNode::field("Key Frames", stream.key_frames().len().to_string()));

    // Add superframe info if present
    if stream.superframe_index.is_superframe() {
        root.add_child(build_superframe_tree(&stream.superframe_index));
    }

    // Add frames
    let mut frames_node = SyntaxNode::new(
        format!("Frames ({})", stream.frames.len()),
        SyntaxNodeType::Array,
    );

    for (i, frame) in stream.frames.iter().enumerate() {
        frames_node.add_child(build_frame_tree(i, frame));
    }

    root.add_child(frames_node);

    root
}

/// Build syntax tree for superframe index.
fn build_superframe_tree(index: &SuperframeIndex) -> SyntaxNode {
    let mut node = SyntaxNode::new("Superframe Index", SyntaxNodeType::Superframe);

    node.add_child(SyntaxNode::field("frame_count", index.frame_count.to_string()));
    node.add_child(SyntaxNode::field(
        "total_size",
        format!("{} bytes", index.total_frame_size()),
    ));

    let mut sizes_node = SyntaxNode::new("frame_sizes", SyntaxNodeType::Array);
    for (i, size) in index.frame_sizes.iter().enumerate() {
        sizes_node.add_child(SyntaxNode::field(
            format!("[{}]", i),
            format!("{} bytes", size),
        ));
    }
    node.add_child(sizes_node);

    node
}

/// Build syntax tree for a frame.
fn build_frame_tree(index: usize, header: &FrameHeader) -> SyntaxNode {
    let frame_type = if header.is_key_frame() { "Key" } else { "Inter" };
    let show = if header.show_frame { ", show" } else { ", hidden" };

    let mut node = SyntaxNode::new(
        format!("[{}] {} frame{} ({}x{})", index, frame_type, show, header.width, header.height),
        SyntaxNodeType::Frame,
    );

    // Basic info
    node.add_child(SyntaxNode::field("frame_type", format!("{:?}", header.frame_type)));
    node.add_child(SyntaxNode::field("show_frame", header.show_frame.to_string()));
    node.add_child(SyntaxNode::field("error_resilient_mode", header.error_resilient_mode.to_string()));

    // Size
    let mut size_node = SyntaxNode::new("size", SyntaxNodeType::Structure);
    size_node.add_child(SyntaxNode::field("width", header.width.to_string()));
    size_node.add_child(SyntaxNode::field("height", header.height.to_string()));
    if header.render_width != header.width || header.render_height != header.height {
        size_node.add_child(SyntaxNode::field("render_width", header.render_width.to_string()));
        size_node.add_child(SyntaxNode::field("render_height", header.render_height.to_string()));
    }
    node.add_child(size_node);

    // Reference info for inter frames
    if !header.is_key_frame() && !header.intra_only {
        let mut ref_node = SyntaxNode::new("references", SyntaxNodeType::Structure);
        ref_node.add_child(SyntaxNode::field("ref_frame_idx[0] (LAST)", header.ref_frame_idx[0].to_string()));
        ref_node.add_child(SyntaxNode::field("ref_frame_idx[1] (GOLDEN)", header.ref_frame_idx[1].to_string()));
        ref_node.add_child(SyntaxNode::field("ref_frame_idx[2] (ALTREF)", header.ref_frame_idx[2].to_string()));
        ref_node.add_child(SyntaxNode::field(
            "interpolation_filter",
            format!("{:?}", header.interpolation_filter),
        ));
        node.add_child(ref_node);
    }

    // Quantization
    let mut quant_node = SyntaxNode::new("quantization", SyntaxNodeType::Structure);
    quant_node.add_child(SyntaxNode::field("base_q_idx", header.quantization.base_q_idx.to_string()));
    quant_node.add_child(SyntaxNode::field("lossless", header.quantization.lossless.to_string()));
    if header.quantization.delta_q_y_dc != 0 {
        quant_node.add_child(SyntaxNode::field("delta_q_y_dc", header.quantization.delta_q_y_dc.to_string()));
    }
    node.add_child(quant_node);

    // Loop filter
    let mut lf_node = SyntaxNode::new("loop_filter", SyntaxNodeType::Structure);
    lf_node.add_child(SyntaxNode::field("level", header.loop_filter.level.to_string()));
    lf_node.add_child(SyntaxNode::field("sharpness", header.loop_filter.sharpness.to_string()));
    node.add_child(lf_node);

    // Segmentation
    if header.segmentation.enabled {
        let mut seg_node = SyntaxNode::new("segmentation", SyntaxNodeType::Structure);
        seg_node.add_child(SyntaxNode::field("enabled", "true".to_string()));
        seg_node.add_child(SyntaxNode::field("update_map", header.segmentation.update_map.to_string()));
        seg_node.add_child(SyntaxNode::field("update_data", header.segmentation.update_data.to_string()));
        node.add_child(seg_node);
    }

    // Tiles
    if header.num_tiles() > 1 {
        let mut tile_node = SyntaxNode::new("tiles", SyntaxNodeType::Structure);
        tile_node.add_child(SyntaxNode::field(
            "count",
            format!("{}x{} = {}", header.num_tile_cols(), header.num_tile_rows(), header.num_tiles()),
        ));
        node.add_child(tile_node);
    }

    // Color info
    let mut color_node = SyntaxNode::new("color", SyntaxNodeType::Structure);
    color_node.add_child(SyntaxNode::field("bit_depth", header.bit_depth.to_string()));
    color_node.add_child(SyntaxNode::field("color_space", format!("{:?}", header.color_space)));
    color_node.add_child(SyntaxNode::field(
        "subsampling",
        format!(
            "{}:{}:{}",
            4,
            if header.subsampling_x { 2 } else { 4 },
            if header.subsampling_x && header.subsampling_y { 0 } else if header.subsampling_x { 2 } else { 4 }
        ),
    ));
    node.add_child(color_node);

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
