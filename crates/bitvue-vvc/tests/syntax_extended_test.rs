#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Extended tests for VVC syntax module
//!
//! Comprehensive tests covering SyntaxNode, SyntaxNodeType,
//! and build_syntax_tree function

use bitvue_vvc::syntax::{build_syntax_tree, SyntaxNode, SyntaxNodeType};
use bitvue_vvc::{NalUnit, NalUnitHeader, NalUnitType, Pps, Sps, VvcStream};
use std::collections::HashMap;

// ============================================================================
// SyntaxNodeType Tests
// ============================================================================

#[test]
fn test_syntax_node_type_all_variants() {
    let root = SyntaxNodeType::Root;
    let nal_unit = SyntaxNodeType::NalUnit;
    let param_set = SyntaxNodeType::ParameterSet;
    let pic_header = SyntaxNodeType::PictureHeader;
    let slice_header = SyntaxNodeType::SliceHeader;
    let field = SyntaxNodeType::Field;
    let array = SyntaxNodeType::Array;
    let structure = SyntaxNodeType::Structure;

    // Verify all variants can be created and compared
    assert_eq!(root, SyntaxNodeType::Root);
    assert_ne!(nal_unit, param_set);
    assert_ne!(slice_header, field);
    assert_ne!(pic_header, array);
}

#[test]
fn test_syntax_node_type_copy() {
    let node_type = SyntaxNodeType::Field;
    let copied = node_type;
    assert_eq!(copied, node_type);
}

#[test]
fn test_syntax_node_type_clone() {
    let node_type = SyntaxNodeType::Structure;
    let cloned = node_type.clone();
    assert_eq!(cloned, node_type);
}

// ============================================================================
// SyntaxNode Tests
// ============================================================================

#[test]
fn test_syntax_node_new_static_str() {
    let node = SyntaxNode::new("Test Node", SyntaxNodeType::Root);
    assert_eq!(node.name, "Test Node");
    assert_eq!(node.node_type, SyntaxNodeType::Root);
    assert!(node.children.is_empty());
    assert!(node.value.is_none());
    assert!(node.bit_offset.is_none());
    assert!(node.bit_length.is_none());
}

#[test]
fn test_syntax_node_new_string() {
    let name = String::from("Dynamic Name");
    let node = SyntaxNode::new(name.clone(), SyntaxNodeType::Structure);
    assert_eq!(node.name, name);
}

#[test]
fn test_syntax_node_field_static() {
    let node = SyntaxNode::field("width", "1920");
    assert_eq!(node.name, "width");
    assert_eq!(node.value, Some("1920".to_string()));
    assert_eq!(node.node_type, SyntaxNodeType::Field);
    assert!(node.children.is_empty());
}

#[test]
fn test_syntax_node_field_string_value() {
    let node = SyntaxNode::field("description", "test description");
    assert_eq!(node.name, "description");
    assert_eq!(node.value, Some("test description".to_string()));
}

#[test]
fn test_syntax_node_field_integer_value() {
    let node = SyntaxNode::field("count", "42");
    assert_eq!(node.name, "count");
    assert_eq!(node.value, Some("42".to_string()));
}

#[test]
fn test_syntax_node_field_float_value() {
    let node = SyntaxNode::field("fps", "29.97");
    assert_eq!(node.name, "fps");
    assert_eq!(node.value, Some("29.97".to_string()));
}

#[test]
fn test_syntax_node_add_child() {
    let mut parent = SyntaxNode::new("Parent", SyntaxNodeType::Structure);
    let child1 = SyntaxNode::field("child1", "value1");
    let child2 = SyntaxNode::field("child2", "value2");

    parent.add_child(child1);
    parent.add_child(child2);

    assert_eq!(parent.children.len(), 2);
    assert_eq!(parent.children[0].name, "child1");
    assert_eq!(parent.children[1].name, "child2");
}

#[test]
fn test_syntax_node_add_multiple_children() {
    let mut node = SyntaxNode::new("Root", SyntaxNodeType::Array);
    for i in 0..10 {
        node.add_child(SyntaxNode::field(format!("item_{}", i), i.to_string()));
    }

    assert_eq!(node.children.len(), 10);
    assert_eq!(node.children[9].name, "item_9");
}

#[test]
fn test_syntax_node_add_nested_children() {
    let mut root = SyntaxNode::new("Root", SyntaxNodeType::Root);
    let mut level1 = SyntaxNode::new("Level1", SyntaxNodeType::Structure);
    let level2 = SyntaxNode::field("level2_field", "value");

    level1.add_child(level2);
    root.add_child(level1);

    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].children[0].name, "level2_field");
}

#[test]
fn test_syntax_node_clone() {
    let mut node = SyntaxNode::new("Original", SyntaxNodeType::Structure);
    node.add_child(SyntaxNode::field("field", "value"));

    let cloned = node.clone();

    assert_eq!(cloned.name, node.name);
    assert_eq!(cloned.node_type, node.node_type);
    assert_eq!(cloned.children.len(), node.children.len());
}

#[test]
fn test_syntax_node_empty_children() {
    let node = SyntaxNode::new("Empty Children", SyntaxNodeType::Array);
    assert!(node.children.is_empty());
    assert_eq!(node.children.len(), 0);
    assert!(node.children.first().is_none());
}

#[test]
fn test_syntax_node_field_no_value() {
    let node = SyntaxNode::field("empty", "");
    assert_eq!(node.name, "empty");
    assert_eq!(node.value, Some("".to_string()));
}

// ============================================================================
// build_syntax_tree Tests
// ============================================================================

#[test]
fn test_build_syntax_tree_empty_stream() {
    let stream = VvcStream {
        nal_units: vec![],
        sps_map: HashMap::new(),
        pps_map: HashMap::new(),
    };

    let tree = build_syntax_tree(&stream);
    assert_eq!(tree.name, "VVC Bitstream");
    assert_eq!(tree.node_type, SyntaxNodeType::Root);
}

#[test]
fn test_build_syntax_tree_with_nal_units() {
    let nal_unit = NalUnit {
        header: NalUnitHeader {
            nal_unit_type: NalUnitType::SpsNut,
            nuh_layer_id: 0,
            nuh_temporal_id_plus1: 1,
        },
        offset: 0,
        size: 100,
        payload: vec![],
        raw_payload: vec![],
    };

    let stream = VvcStream {
        nal_units: vec![nal_unit],
        sps_map: HashMap::new(),
        pps_map: HashMap::new(),
    };

    let tree = build_syntax_tree(&stream);
    let nal_units_node = tree
        .children
        .iter()
        .find(|n| n.name == "NAL Units")
        .expect("Should have NAL Units child");

    assert!(nal_units_node.children.len() > 0);
}

#[test]
fn test_syntax_node_various_node_types() {
    let root_node = SyntaxNode::new("Root", SyntaxNodeType::Root);
    assert_eq!(root_node.node_type, SyntaxNodeType::Root);

    let nal_node = SyntaxNode::new("NAL", SyntaxNodeType::NalUnit);
    assert_eq!(nal_node.node_type, SyntaxNodeType::NalUnit);

    let param_node = SyntaxNode::new("Param", SyntaxNodeType::ParameterSet);
    assert_eq!(param_node.node_type, SyntaxNodeType::ParameterSet);

    let slice_node = SyntaxNode::new("Slice", SyntaxNodeType::SliceHeader);
    assert_eq!(slice_node.node_type, SyntaxNodeType::SliceHeader);

    let pic_node = SyntaxNode::new("Picture", SyntaxNodeType::PictureHeader);
    assert_eq!(pic_node.node_type, SyntaxNodeType::PictureHeader);
}

#[test]
fn test_syntax_node_with_capacity() {
    // Create nodes with different expected capacities
    let field_node = SyntaxNode::field("test", "value");
    assert!(field_node.children.capacity() >= 0);

    let array_node = SyntaxNode::new("Array", SyntaxNodeType::Array);
    assert!(array_node.children.capacity() >= 8); // Pre-allocated capacity
}
