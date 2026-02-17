#![allow(dead_code)]
//! Minimal tests for VVC syntax module

use bitvue_vvc::syntax::{SyntaxNode, SyntaxNodeType};

#[test]
fn test_syntax_node_new_basic() {
    let node = SyntaxNode::new("Test", SyntaxNodeType::Root);
    assert_eq!(node.name, "Test");
    assert_eq!(node.node_type, SyntaxNodeType::Root);
    assert!(node.value.is_none());
    assert!(node.children.is_empty());
}

#[test]
fn test_syntax_node_field() {
    let node = SyntaxNode::field("test", "value");
    assert_eq!(node.name, "test");
    assert_eq!(node.node_type, SyntaxNodeType::Field);
    assert_eq!(node.value, Some("value".to_string()));
}

#[test]
fn test_syntax_node_add_child() {
    let mut parent = SyntaxNode::new("Parent", SyntaxNodeType::Structure);
    let child = SyntaxNode::field("Child", "child");

    parent.add_child(child);

    assert_eq!(parent.children.len(), 1);
    assert_eq!(parent.children[0].name, "Child");
}

#[test]
fn test_build_syntax_tree_empty() {
    use bitvue_vvc::VvcStream;
    use std::collections::HashMap;

    let stream = VvcStream {
        nal_units: vec![],
        sps_map: HashMap::new(),
        pps_map: HashMap::new(),
    };

    let tree = bitvue_vvc::syntax::build_syntax_tree(&stream);

    assert_eq!(tree.name, "VVC Bitstream");
    assert_eq!(tree.node_type, SyntaxNodeType::Root);
    // build_syntax_tree adds NAL Units array node even when empty
    assert!(!tree.children.is_empty());
}
