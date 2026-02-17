#![allow(dead_code)]
//! Unit tests for HEVC syntax module
//!
//! Tests for SyntaxNode and SyntaxNodeType

use bitvue_hevc::syntax::{SyntaxNode, SyntaxNodeType};

// ============================================================================
// SyntaxNodeType Tests
// ============================================================================

#[test]
fn test_syntax_node_type_root() {
    let node_type = SyntaxNodeType::Root;
    assert_eq!(node_type, SyntaxNodeType::Root);
}

#[test]
fn test_syntax_node_type_nal_unit() {
    let node_type = SyntaxNodeType::NalUnit;
    assert_eq!(node_type, SyntaxNodeType::NalUnit);
}

#[test]
fn test_syntax_node_type_parameter_set() {
    let node_type = SyntaxNodeType::ParameterSet;
    assert_eq!(node_type, SyntaxNodeType::ParameterSet);
}

#[test]
fn test_syntax_node_type_slice_header() {
    let node_type = SyntaxNodeType::SliceHeader;
    assert_eq!(node_type, SyntaxNodeType::SliceHeader);
}

#[test]
fn test_syntax_node_type_field() {
    let node_type = SyntaxNodeType::Field;
    assert_eq!(node_type, SyntaxNodeType::Field);
}

#[test]
fn test_syntax_node_type_array() {
    let node_type = SyntaxNodeType::Array;
    assert_eq!(node_type, SyntaxNodeType::Array);
}

#[test]
fn test_syntax_node_type_structure() {
    let node_type = SyntaxNodeType::Structure;
    assert_eq!(node_type, SyntaxNodeType::Structure);
}

// ============================================================================
// SyntaxNode Creation Tests
// ============================================================================

#[test]
fn test_syntax_node_new_root() {
    let node = SyntaxNode::new("Root Node", SyntaxNodeType::Root);
    assert_eq!(node.name, "Root Node");
    assert_eq!(node.node_type, SyntaxNodeType::Root);
    assert!(node.children.is_empty());
}

#[test]
fn test_syntax_node_new_structure() {
    let node = SyntaxNode::new("Structure Node", SyntaxNodeType::Structure);
    assert_eq!(node.name, "Structure Node");
    assert_eq!(node.node_type, SyntaxNodeType::Structure);
}

#[test]
fn test_syntax_node_field_string() {
    let node = SyntaxNode::field("field_name", "field_value");
    assert_eq!(node.name, "field_name");
    assert_eq!(node.value, Some("field_value".to_string()));
    assert_eq!(node.node_type, SyntaxNodeType::Field);
}

#[test]
fn test_syntax_node_field_integer() {
    let node = SyntaxNode::field("count", "42");
    assert_eq!(node.name, "count");
    assert_eq!(node.value, Some("42".to_string()));
}

#[test]
fn test_syntax_node_field_float() {
    let node = SyntaxNode::field("fps", "29.97");
    assert_eq!(node.name, "fps");
    assert_eq!(node.value, Some("29.97".to_string()));
}

#[test]
fn test_syntax_node_clone() {
    let mut node = SyntaxNode::new("Test", SyntaxNodeType::Root);
    node.add_child(SyntaxNode::field("child", "value"));

    let cloned = node.clone();
    assert_eq!(cloned.name, "Test");
    assert_eq!(cloned.children.len(), 1);
}

#[test]
fn test_syntax_node_with_position() {
    let node = SyntaxNode::new("Positioned", SyntaxNodeType::Field).with_position(100, 32);

    assert_eq!(node.bit_offset, Some(100));
    assert_eq!(node.bit_length, Some(32));
}

#[test]
fn test_syntax_node_add_child() {
    let mut parent = SyntaxNode::new("Parent", SyntaxNodeType::Structure);
    let child = SyntaxNode::field("Child", "Value");

    parent.add_child(child);
    assert_eq!(parent.children.len(), 1);
    assert_eq!(parent.children[0].name, "Child");
}

#[test]
fn test_syntax_node_add_multiple_children() {
    let mut node = SyntaxNode::new("Parent", SyntaxNodeType::Array);
    node.add_child(SyntaxNode::field("child1", "value1"));
    node.add_child(SyntaxNode::field("child2", "value2"));
    node.add_child(SyntaxNode::field("child3", "value3"));

    assert_eq!(node.children.len(), 3);
    assert_eq!(node.children[2].name, "child3");
}

#[test]
fn test_syntax_node_none_value() {
    let node = SyntaxNode::new("Test", SyntaxNodeType::Root);
    assert!(node.value.is_none());
}

#[test]
fn test_syntax_node_some_value() {
    let node = SyntaxNode::field("key", "value");
    assert!(node.value.is_some());
    assert_eq!(node.value.unwrap(), "value");
}
