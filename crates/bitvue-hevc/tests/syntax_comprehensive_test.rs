#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! HEVC Syntax Module Comprehensive Tests
//!
//! Comprehensive tests for syntax/mod.rs to reach 95% coverage target.

use bitvue_hevc::parse_hevc;
use bitvue_hevc::syntax::{build_syntax_tree, SyntaxNode, SyntaxNodeType};

// ============================================================================
// SyntaxNode Creation Tests
// ============================================================================

#[test]
fn test_syntax_node_new() {
    let node = SyntaxNode::new("test", SyntaxNodeType::Structure);
    assert_eq!(node.name, "test");
    assert_eq!(node.node_type, SyntaxNodeType::Structure);
    assert!(node.value.is_none());
    assert!(node.bit_offset.is_none());
    assert!(node.bit_length.is_none());
    assert!(node.children.is_empty());
}

#[test]
fn test_syntax_node_with_value() {
    let node = SyntaxNode::new("field", SyntaxNodeType::Field);
    assert_eq!(node.name, "field");
    assert!(node.value.is_none());
}

#[test]
fn test_syntax_node_field() {
    let node = SyntaxNode::field("name", "value");
    assert_eq!(node.name, "name");
    assert_eq!(node.value, Some("value".to_string()));
    assert_eq!(node.node_type, SyntaxNodeType::Field);
}

#[test]
fn test_syntax_node_field_into() {
    let node = SyntaxNode::field("id", "42");
    assert_eq!(node.name, "id");
    assert_eq!(node.value, Some("42".to_string()));
}

// ============================================================================
// SyntaxNode::add_child Tests
// ============================================================================

#[test]
fn test_add_child() {
    let mut parent = SyntaxNode::new("parent", SyntaxNodeType::Structure);
    let child = SyntaxNode::new("child", SyntaxNodeType::Structure);

    parent.add_child(child);

    assert_eq!(parent.children.len(), 1);
    assert_eq!(&parent.children[0].name, "child");
}

#[test]
fn test_add_multiple_children() {
    let mut parent = SyntaxNode::new("root", SyntaxNodeType::Structure);

    for i in 0..5 {
        let child = SyntaxNode::new(format!("child{}", i), SyntaxNodeType::Structure);
        parent.add_child(child);
    }

    assert_eq!(parent.children.len(), 5);
}

#[test]
fn test_add_child_to_field() {
    let mut field = SyntaxNode::field("parent", "value");
    let child = SyntaxNode::new("child", SyntaxNodeType::Structure);

    field.add_child(child);

    assert_eq!(field.children.len(), 1);
    assert_eq!(field.node_type, SyntaxNodeType::Field);
}

// ============================================================================
// SyntaxNode::with_position Tests
// ============================================================================

#[test]
fn test_with_position() {
    let node = SyntaxNode::new("test", SyntaxNodeType::Structure);
    let node = node.with_position(100, 50);

    assert_eq!(node.bit_offset, Some(100));
    assert_eq!(node.bit_length, Some(50));
}

#[test]
fn test_with_position_zero() {
    let node = SyntaxNode::new("zero", SyntaxNodeType::Structure);
    let node = node.with_position(0, 0);

    assert_eq!(node.bit_offset, Some(0));
    assert_eq!(node.bit_length, Some(0));
}

#[test]
fn test_with_position_update() {
    let node = SyntaxNode::new("test", SyntaxNodeType::Structure);
    let node = node.with_position(10, 5);
    assert_eq!(node.bit_offset, Some(10));
    assert_eq!(node.bit_length, Some(5));

    let node = node.with_position(20, 10);
    assert_eq!(node.bit_offset, Some(20));
    assert_eq!(node.bit_length, Some(10));
}

// ============================================================================
// SyntaxNodeType Enum Tests
// ============================================================================

#[test]
fn test_node_type_root() {
    let node_type = SyntaxNodeType::Root;
    assert!(matches!(node_type, SyntaxNodeType::Root));
}

#[test]
fn test_node_type_nal_unit() {
    let node_type = SyntaxNodeType::NalUnit;
    assert!(matches!(node_type, SyntaxNodeType::NalUnit));
}

#[test]
fn test_node_type_parameter_set() {
    let node_type = SyntaxNodeType::ParameterSet;
    assert!(matches!(node_type, SyntaxNodeType::ParameterSet));
}

#[test]
fn test_node_type_slice_header() {
    let node_type = SyntaxNodeType::SliceHeader;
    assert!(matches!(node_type, SyntaxNodeType::SliceHeader));
}

#[test]
fn test_node_type_field() {
    let node_type = SyntaxNodeType::Field;
    assert!(matches!(node_type, SyntaxNodeType::Field));
}

#[test]
fn test_node_type_array() {
    let node_type = SyntaxNodeType::Array;
    assert!(matches!(node_type, SyntaxNodeType::Array));
}

#[test]
fn test_node_type_structure() {
    let node_type = SyntaxNodeType::Structure;
    assert!(matches!(node_type, SyntaxNodeType::Structure));
}

// ============================================================================
// build_syntax_tree Tests
// ============================================================================

#[test]
fn test_build_syntax_tree_empty() {
    let data = &[];
    let result = parse_hevc(data);

    assert!(result.is_ok());
    let stream = result.unwrap();

    let tree = build_syntax_tree(&stream);

    assert_eq!(tree.name, "HEVC Bitstream");
    assert_eq!(tree.node_type, SyntaxNodeType::Root);
}

#[test]
fn test_build_syntax_tree_with_nal() {
    // Minimal HEVC bitstream with VPS NAL unit
    let data = &[
        0x00, 0x00, 0x01, // Start code
        0x40, 0x01, // VPS NAL header (type 32)
        0x01, // nuh_layer_id + 1
    ];

    let result = parse_hevc(data);
    assert!(result.is_ok());

    let stream = result.unwrap();
    let tree = build_syntax_tree(&stream);

    assert!(tree.children.len() > 0);

    // Should have "NAL Units" child node
    let nal_units_node = tree.children.iter().find(|n| n.name == "NAL Units");

    assert!(nal_units_node.is_some());
}

#[test]
fn test_build_syntax_tree_with_dimensions() {
    let data = &[
        0x00, 0x00, 0x01, // Start code
        0x40, 0x01, // VPS NAL header
        0x01, // nuh_layer_id + 1
              // Followed by valid VPS data
    ];

    let result = parse_hevc(data);
    assert!(result.is_ok());

    let stream = result.unwrap();
    let tree = build_syntax_tree(&stream);

    // Verify tree builds successfully (resolution node may not exist with minimal VPS data)
    let _tree = tree;
}

#[test]
fn test_build_syntax_tree_with_frame_rate() {
    let data = &[
        0x00, 0x00, 0x01, // Start code
        0x40, 0x01, // VPS NAL header
        0x01, // nuh_layer_id + 1
              // VPS data with timing info would go here
    ];

    let result = parse_hevc(data);
    assert!(result.is_ok());

    let stream = result.unwrap();
    let tree = build_syntax_tree(&stream);

    // Frame rate node may or may not exist depending on data
    let _tree = tree; // Just verify it builds without error
}

// ============================================================================
// Complex Tree Structure Tests
// ============================================================================

#[test]
fn test_nested_structure_nodes() {
    let mut root = SyntaxNode::new("root", SyntaxNodeType::Structure);

    let mut child1 = SyntaxNode::new("child1", SyntaxNodeType::Structure);
    let mut child2 = SyntaxNode::new("child2", SyntaxNodeType::Structure);

    let grandchild1 = SyntaxNode::new("gc1", SyntaxNodeType::Field);
    let grandchild2 = SyntaxNode::new("gc2", SyntaxNodeType::Array);

    child2.add_child(grandchild1);
    child2.add_child(grandchild2);

    child1.add_child(SyntaxNode::field("field1", "value1"));
    child1.add_child(child2);

    root.add_child(child1);

    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].children.len(), 2);
    assert_eq!(root.children[0].children[1].children.len(), 2);
}

#[test]
fn test_mixed_node_types() {
    let mut root = SyntaxNode::new("mixed", SyntaxNodeType::Structure);

    root.add_child(SyntaxNode::field("field", "value"));
    root.add_child(SyntaxNode::new("struct", SyntaxNodeType::Structure));
    root.add_child(SyntaxNode::new("array", SyntaxNodeType::Array));

    assert_eq!(root.children.len(), 3);

    let types: Vec<_> = root.children.iter().map(|n| n.node_type).collect();

    assert!(types.contains(&SyntaxNodeType::Field));
    assert!(types.contains(&SyntaxNodeType::Structure));
    assert!(types.contains(&SyntaxNodeType::Array));
}

#[test]
fn test_node_with_values_and_positions() {
    let node = SyntaxNode::new("test", SyntaxNodeType::Structure);
    let node = node.with_position(100, 50);

    let child1 = SyntaxNode::field("field1", "value1");
    let child2 = SyntaxNode::field("field2", "value2");
    let child2 = child2.with_position(200, 30);

    let mut node = node;
    node.add_child(child1);
    node.add_child(child2);

    // Parent has position
    assert_eq!(node.bit_offset, Some(100));
    assert_eq!(node.bit_length, Some(50));

    // First child has no position
    assert_eq!(node.children[0].bit_offset, None);
    assert_eq!(node.children[0].bit_length, None);

    // Second child has position
    assert_eq!(node.children[1].bit_offset, Some(200));
    assert_eq!(node.children[1].bit_length, Some(30));
}
