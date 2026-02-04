//! Syntax Parser - Detailed AV1 OBU parsing with bit-level tracking
//!
//! This module provides detailed syntax tree generation for AV1 OBUs,
//! tracking exact bit ranges for each field. Used for Tri-sync functionality
//! in the GUI (Tree ↔ Syntax ↔ Hex).
//!
//! # Architecture
//!
//! - `TrackedBitReader`: Wraps `BitReader` with absolute bit position tracking
//! - `SyntaxBuilder`: Builds `SyntaxModel` tree structure during parsing
//! - OBU-specific parsers: Parse different OBU types with bit-level detail
//!
//! # Usage
//!
//! ```ignore
//! use bitvue_av1::syntax_parser::parse_obu_syntax;
//!
//! // Parse a single OBU at byte offset 128
//! let model = parse_obu_syntax(obu_data, 0, 128 * 8)?;
//!
//! // Access syntax nodes
//! for node in model.nodes.values() {
//!     println!("{}: {} (bits {}-{})",
//!         node.field_name,
//!         node.value.as_ref().unwrap_or(&"<container>".to_string()),
//!         node.bit_range.start_bit,
//!         node.bit_range.end_bit
//!     );
//! }
//! ```

mod frame_header;
mod obu;
mod sequence;
mod tracked_bitreader;

pub use frame_header::parse_frame_header_syntax;
pub use sequence::parse_sequence_header_syntax;
pub use tracked_bitreader::TrackedBitReader;

use crate::obu::{ObuIterator, ObuWithOffset};
use bitvue_core::{
    types::{BitRange, SyntaxModel, SyntaxNode, SyntaxNodeId},
    Result,
};
use obu::{parse_leb128_size_syntax, parse_obu_header_syntax};

/// Builder for constructing SyntaxModel trees during parsing
///
/// This builder provides a convenient API for creating syntax trees with
/// parent/child relationships and automatic bit range tracking.
///
/// # Example
///
/// ```ignore
/// let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());
///
/// // Start a container
/// builder.push_container("obu_header", 0);
///
/// // Add leaf fields
/// builder.add_field("obu_type", BitRange::new(1, 5), "1 (SEQUENCE_HEADER)".to_string());
/// builder.add_field("obu_forbidden_bit", BitRange::new(0, 1), "0".to_string());
///
/// // End container
/// builder.pop_container(8);
///
/// let model = builder.build();
/// ```
pub struct SyntaxBuilder {
    /// The syntax model being built
    model: SyntaxModel,

    /// Stack of parent node IDs (for nested structures)
    parent_stack: Vec<SyntaxNodeId>,
}

impl SyntaxBuilder {
    /// Create a new syntax builder with a root node
    ///
    /// # Arguments
    ///
    /// * `root_id` - Unique ID for the root node (e.g., `"obu[0]"`)
    /// * `unit_key` - Unit key this syntax belongs to (e.g., `"obu_0"`)
    pub fn new(root_id: String, unit_key: String) -> Self {
        let root_node = SyntaxNode::new(
            root_id.clone(),
            BitRange::new(0, 0), // Will be updated at end
            unit_key.clone(),
            None,
            None,
            0,
        );

        let mut model = SyntaxModel::new(root_id.clone(), unit_key);
        model.add_node(root_node);

        Self {
            model,
            parent_stack: vec![root_id],
        }
    }

    /// Add a leaf field node
    ///
    /// # Arguments
    ///
    /// * `field_name` - Name of the field
    /// * `bit_range` - Bit range this field occupies
    /// * `value` - String representation of the value
    ///
    /// # Returns
    ///
    /// The node ID of the created field
    pub fn add_field(
        &mut self,
        field_name: &str,
        bit_range: BitRange,
        value: String,
    ) -> SyntaxNodeId {
        let parent = self.current_parent();
        let node_id = format!("{}.{}", parent, field_name);
        let depth = self.parent_stack.len();

        let node = SyntaxNode::new(
            node_id.clone(),
            bit_range,
            field_name.to_string(),
            Some(value),
            Some(parent.clone()),
            depth,
        );

        self.model.add_node(node);
        node_id
    }

    /// Start a container node (e.g., a struct or nested structure)
    ///
    /// Call `pop_container` when done to finalize the bit range.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the container
    /// * `start_bit` - Starting bit position
    ///
    /// # Returns
    ///
    /// The node ID of the container
    pub fn push_container(&mut self, name: &str, start_bit: u64) -> SyntaxNodeId {
        let parent = self.current_parent();
        let node_id = format!("{}.{}", parent, name);
        let depth = self.parent_stack.len();

        let node = SyntaxNode::new(
            node_id.clone(),
            BitRange::new(start_bit, start_bit), // End updated on pop
            name.to_string(),
            None, // Containers have no direct value
            Some(parent.clone()),
            depth,
        );

        self.model.add_node(node);
        self.parent_stack.push(node_id.clone());
        node_id
    }

    /// End a container node and update its bit range
    ///
    /// # Arguments
    ///
    /// * `end_bit` - Ending bit position (exclusive)
    pub fn pop_container(&mut self, end_bit: u64) {
        if let Some(container_id) = self.parent_stack.pop() {
            if let Some(node) = self.model.nodes.get_mut(&container_id) {
                node.bit_range.end_bit = end_bit;
            }
        }
    }

    /// Get the current parent node ID
    fn current_parent(&self) -> &SyntaxNodeId {
        self.parent_stack
            .last()
            .expect("Parent stack should never be empty")
    }

    /// Finalize and return the built syntax model
    ///
    /// This updates the root node's bit range to span the entire parsed content.
    pub fn build(mut self) -> SyntaxModel {
        // Update root bit range to span entire unit
        // First, find the max end bit
        let max_end = self
            .model
            .nodes
            .values()
            .map(|node| node.bit_range.end_bit)
            .max()
            .unwrap_or(0);

        // Then update root
        if let Some(root) = self.model.nodes.get_mut(&self.model.root_id) {
            root.bit_range.end_bit = max_end;
        }

        self.model
    }
}

/// Parse a single OBU and return its detailed syntax tree
///
/// # Arguments
///
/// * `data` - The OBU data (including header and payload)
/// * `obu_index` - Index of this OBU in the bitstream (for node ID generation)
/// * `global_offset` - Absolute bit offset of this OBU from file start
///
/// # Returns
///
/// A `SyntaxModel` containing all parsed fields with exact bit ranges.
///
/// # Example
///
/// ```ignore
/// // Parse the first OBU at file offset 0
/// let model = parse_obu_syntax(obu_data, 0, 0)?;
///
/// // Parse the third OBU at byte offset 512
/// let model = parse_obu_syntax(obu_data, 2, 512 * 8)?;
/// ```
pub fn parse_obu_syntax(data: &[u8], obu_index: usize, global_offset: u64) -> Result<SyntaxModel> {
    let root_id = format!("obu[{}]", obu_index);
    let unit_key = format!("obu_{}", obu_index);
    let mut builder = SyntaxBuilder::new(root_id, unit_key);
    let mut reader = TrackedBitReader::new(data, global_offset);

    // Parse OBU header
    let (obu_type, has_size_field) = parse_obu_header_syntax(&mut reader, &mut builder)?;

    // Parse size field if present (Step 3)
    let _obu_size = if has_size_field {
        Some(parse_leb128_size_syntax(&mut reader, &mut builder)?)
    } else {
        None
    };

    // Parse payload based on OBU type (Steps 4-5)
    use crate::obu::ObuType;
    match obu_type {
        ObuType::SequenceHeader => {
            parse_sequence_header_syntax(&mut reader, &mut builder)?;
        }
        ObuType::Frame | ObuType::FrameHeader => {
            parse_frame_header_syntax(&mut reader, &mut builder)?;
        }
        _ => {
            // Other OBU types - not implemented yet
            // (Temporal Delimiter, Tile Group, Metadata, Padding, etc.)
        }
    }

    Ok(builder.build())
}

/// Parse all OBUs in a bitstream and return syntax models
///
/// # Arguments
///
/// * `data` - The complete bitstream data
///
/// # Returns
///
/// A vector of `SyntaxModel`, one for each OBU.
pub fn parse_bitstream_syntax(data: &[u8]) -> Result<Vec<SyntaxModel>> {
    let mut models = Vec::new();
    let mut obu_index = 0;
    let mut iter = ObuIterator::new(data);

    while let Some(result) = iter.next_obu_with_offset() {
        let obu_with_offset = result?;
        let ObuWithOffset {
            obu: _,
            offset,
            consumed,
        } = obu_with_offset;

        // Parse syntax for this OBU
        let model = parse_obu_syntax(
            &data[offset..offset + consumed],
            obu_index,
            (offset * 8) as u64,
        )?;

        models.push(model);
        obu_index += 1;
    }

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bitstream_with_multiple_obus() {
        // Create a minimal bitstream with 3 OBUs:
        // 1. Temporal Delimiter (OBU type 2)
        // 2. Sequence Header (OBU type 1)
        // 3. Frame Header (OBU type 3)

        let mut bitstream = Vec::new();

        // OBU 1: Temporal Delimiter
        // Header: 0x12 (type=2, has_size=1)
        // Size: 0 (leb128)
        bitstream.push(0x12);
        bitstream.push(0x00);

        // OBU 2: Sequence Header
        // Header: 0x0A (type=1, has_size=1)
        // Size: 10 (leb128)
        // Payload: minimal sequence header (10 bytes)
        bitstream.push(0x0A);
        bitstream.push(0x0A); // size = 10
                              // Profile=0, still=0, reduced=1, level=4
        bitstream.push(0b00001001); // profile=0, still=0, reduced=1, level[4:2]=001
        bitstream.push(0b00000000); // level[1:0]=00 + padding
                                    // Add 8 more bytes for remaining fields
        bitstream.extend_from_slice(&[0u8; 8]);

        // OBU 3: Frame Header
        // Header: 0x32 (type=3, has_size=1)
        // Size: 5 (leb128)
        // Payload: minimal frame header (5 bytes)
        bitstream.push(0x32);
        bitstream.push(0x05); // size = 5
                              // show_existing=0, frame_type=00 (KEY), show_frame=1
        bitstream.push(0b00010000);
        bitstream.extend_from_slice(&[0u8; 4]); // padding

        // Parse the entire bitstream
        let result = parse_bitstream_syntax(&bitstream);
        assert!(
            result.is_ok(),
            "Bitstream parsing should succeed: {:?}",
            result.err()
        );

        let models = result.unwrap();

        // Should have 3 OBUs
        assert_eq!(models.len(), 3, "Expected 3 OBUs");

        // Verify OBU 0: Temporal Delimiter
        let obu0 = &models[0];
        assert_eq!(obu0.root_id, "obu[0]");
        assert!(obu0.get_node("obu[0].obu_header").is_some());
        let obu0_type = obu0.get_node("obu[0].obu_header.obu_type").unwrap();
        assert!(obu0_type
            .value
            .as_ref()
            .unwrap()
            .contains("TEMPORAL_DELIMITER"));

        // Verify OBU 1: Sequence Header
        let obu1 = &models[1];
        assert_eq!(obu1.root_id, "obu[1]");
        assert!(obu1.get_node("obu[1].obu_header").is_some());
        assert!(obu1.get_node("obu[1].sequence_header").is_some());
        let profile = obu1.get_node("obu[1].sequence_header.seq_profile").unwrap();
        assert!(profile.value.as_ref().unwrap().contains("Main"));

        // Verify OBU 2: Frame Header
        let obu2 = &models[2];
        assert_eq!(obu2.root_id, "obu[2]");
        assert!(obu2.get_node("obu[2].obu_header").is_some());
        assert!(obu2.get_node("obu[2].frame_header").is_some());
        let frame_type = obu2.get_node("obu[2].frame_header.frame_type").unwrap();
        assert!(frame_type.value.as_ref().unwrap().contains("KEY"));
    }

    #[test]
    fn test_parse_single_obu_syntax() {
        // Test parse_obu_syntax with a single Temporal Delimiter OBU
        // Header: 0x12 (type=2, has_size=1)
        // Size: 0 (leb128)
        let data = vec![0x12, 0x00];

        let result = parse_obu_syntax(&data, 0, 0);
        assert!(
            result.is_ok(),
            "OBU parsing should succeed: {:?}",
            result.err()
        );

        let model = result.unwrap();

        // Verify structure
        assert_eq!(model.root_id, "obu[0]");
        assert!(model.get_node("obu[0].obu_header").is_some());
        assert!(model.get_node("obu[0].obu_size").is_some());

        let obu_type = model.get_node("obu[0].obu_header.obu_type").unwrap();
        assert!(obu_type
            .value
            .as_ref()
            .unwrap()
            .contains("TEMPORAL_DELIMITER"));

        let size = model.get_node("obu[0].obu_size_value").unwrap();
        assert_eq!(size.value.as_ref().unwrap(), "0 bytes");
    }

    #[test]
    fn test_node_hierarchy_and_bit_ranges() {
        // Test that node hierarchy is correct and bit ranges don't overlap
        let data = vec![0x0A, 0x05, 0b00001001, 0x00, 0x00, 0x00, 0x00];

        let model = parse_obu_syntax(&data, 0, 0).unwrap();

        // Get all nodes
        let header = model.get_node("obu[0].obu_header").unwrap();
        let size = model.get_node("obu[0].obu_size").unwrap();

        // Verify bit ranges don't overlap
        assert!(
            header.bit_range.end_bit <= size.bit_range.start_bit,
            "Header should end before size starts"
        );

        // Verify parent-child relationships
        assert_eq!(header.parent, Some("obu[0]".to_string()));
        assert!(header.children.len() > 0, "Header should have children");
    }
}
