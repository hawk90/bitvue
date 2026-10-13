//! `SyntaxBuilder`: assembles the `SyntaxModel` tree while the OBU parsers run.

use bitvue_engine::types::{BitRange, SyntaxModel, SyntaxNode, SyntaxNodeId};

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
        let parent = self.current_parent().unwrap_or(&self.model.root_id);
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
        let parent = self.current_parent().unwrap_or(&self.model.root_id);
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
    ///
    /// Returns None if parent stack is empty (should never happen in normal operation)
    fn current_parent(&self) -> Option<&SyntaxNodeId> {
        self.parent_stack.last()
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
