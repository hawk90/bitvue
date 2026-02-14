//! Comprehensive tests for VVC syntax module
//!
//! Tests SyntaxNode, SyntaxNodeType, and build_syntax_tree function

use bitvue_vvc::{syntax::{build_syntax_tree, SyntaxNode, SyntaxNodeType}, VvcStream, NalUnit, NalUnitHeader, NalUnitType};

// ============================================================================
// SyntaxNodeType Tests
// ============================================================================

#[test]
fn test_node_type_all_variants_exist() {
    let _root = SyntaxNodeType::Root;
    let _nal = SyntaxNodeType::NalUnit;
    let _param = SyntaxNodeType::ParameterSet;
    let _slice = SyntaxNodeType::SliceHeader;
    let _field = SyntaxNodeType::Field;
    let _array = SyntaxNodeType::Array;
    let _structure = SyntaxNodeType::Structure;
}

#[test]
fn test_node_type_copy_trait() {
    let node_type = SyntaxNodeType::Field;
    let copied = node_type;
    assert_eq!(node_type, copied);
}

#[test]
fn test_node_type_eq() {
    assert_eq!(SyntaxNodeType::Field, SyntaxNodeType::Field);
    assert_ne!(SyntaxNodeType::Field, SyntaxNodeType::Array);
}

#[test]
fn test_node_type_all_distinct() {
    let types = [
        SyntaxNodeType::Root,
        SyntaxNodeType::NalUnit,
        SyntaxNodeType::ParameterSet,
        SyntaxNodeType::SliceHeader,
        SyntaxNodeType::Field,
        SyntaxNodeType::Array,
        SyntaxNodeType::Structure,
    ];

    for i in 0..types.len() {
        for j in (i + 1)..types.len() {
            assert_ne!(types[i], types[j]);
        }
    }
}

// ============================================================================
// SyntaxNode::new() Tests
// ============================================================================

#[test]
fn test_syntax_node_new_basic() {
    let node = SyntaxNode::new("Test Node", SyntaxNodeType::Root);
    assert_eq!(node.name, "Test Node");
    assert_eq!(node.node_type, SyntaxNodeType::Root);
    assert!(node.value.is_none());
    assert!(node.bit_offset.is_none());
    assert!(node.bit_length.is_none());
    assert!(node.children.is_empty());
    assert!(node.children.capacity() > 0);
}

#[test]
fn test_syntax_node_new_all_types() {
    let types = vec![
        (SyntaxNodeType::Root, "Root"),
        (SyntaxNodeType::NalUnit, "NAL"),
        (SyntaxNodeType::ParameterSet, "Param"),
        (SyntaxNodeType::SliceHeader, "Slice"),
        (SyntaxNodeType::Field, "Field"),
        (SyntaxNodeType::Array, "Array"),
        (SyntaxNodeType::Structure, "Struct"),
    ];

    for (node_type, _name) in types {
        let node = SyntaxNode::new(_name, node_type);
        assert_eq!(node.name, _name);
        assert_eq!(node.node_type, node_type);
    }
}

#[test]
fn test_syntax_node_new_with_static_str() {
    let node = SyntaxNode::new("Static String", SyntaxNodeType::Field);
    assert_eq!(node.name, "Static String");
}

#[test]
fn test_syntax_node_new_with_string() {
    let owned = String::from("Owned String");
    let node = SyntaxNode::new(owned.clone(), SyntaxNodeType::Field);
    assert_eq!(node.name, "Owned String");
}

#[test]
fn test_syntax_node_new_empty_name() {
    let node = SyntaxNode::new("", SyntaxNodeType::Field);
    assert_eq!(node.name, "");
}

// ============================================================================
// SyntaxNode::field() Tests
// ============================================================================

#[test]
fn test_syntax_node_field_basic() {
    let node = SyntaxNode::field("test_field", "test_value");
    assert_eq!(node.name, "test_field");
    assert_eq!(node.node_type, SyntaxNodeType::Field);
    assert_eq!(node.value, Some("test_value".to_string()));
    assert!(node.children.is_empty());
    assert!(node.bit_offset.is_none());
    assert!(node.bit_length.is_none());
}

#[test]
fn test_syntax_node_field_with_int() {
    let node = SyntaxNode::field("count", "42");
    assert_eq!(node.value, Some("42".to_string()));
}

#[test]
fn test_syntax_node_field_with_format() {
    let node = SyntaxNode::field("resolution", "1920x1080");
    assert_eq!(node.value, Some("1920x1080".to_string()));
}

#[test]
fn test_syntax_node_field_empty_value() {
    let node = SyntaxNode::field("empty", "");
    assert_eq!(node.value, Some("".to_string()));
}

// ============================================================================
// SyntaxNode::add_child() Tests
// ============================================================================

#[test]
fn test_syntax_node_add_child_single() {
    let mut parent = SyntaxNode::new("Parent", SyntaxNodeType::Structure);
    let child = SyntaxNode::field("Child", "value");

    parent.add_child(child);

    assert_eq!(parent.children.len(), 1);
    assert_eq!(parent.children[0].name, "Child");
    assert_eq!(parent.children[0].value, Some("value".to_string()));
    assert_eq!(parent.children[0].node_type, SyntaxNodeType::Field);
}

#[test]
fn test_syntax_node_add_child_multiple() {
    let mut parent = SyntaxNode::new("Parent", SyntaxNodeType::Array);

    let items = [("item0", "0"), ("item1", "1"), ("item2", "2"), ("item3", "3"), ("item4", "4")];
    for (name, value) in items {
        parent.add_child(SyntaxNode::field(name, value));
    }

    assert_eq!(parent.children.len(), 5);
    for i in 0..5 {
        assert_eq!(parent.children[i].name, format!("item{}", i));
    }
}

#[test]
fn test_syntax_node_add_child_nested() {
    let mut root = SyntaxNode::new("Root", SyntaxNodeType::Root);
    let mut level1 = SyntaxNode::new("Level1", SyntaxNodeType::Structure);
    let level2 = SyntaxNode::field("Level2", "data");

    level1.add_child(level2.clone());
    root.add_child(level1);

    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].name, "Level1");
    assert_eq!(root.children[0].node_type, SyntaxNodeType::Structure);
    assert_eq!(root.children[0].children[0].value, Some("data".to_string()));
}

#[test]
fn test_syntax_node_add_child_preserves_parent_fields() {
    let mut parent = SyntaxNode::new("Parent", SyntaxNodeType::Structure);
    parent.bit_offset = Some(100);
    parent.bit_length = Some(32);

    let child = SyntaxNode::field("child", "value");
    parent.add_child(child);

    assert_eq!(parent.bit_offset, Some(100));
    assert_eq!(parent.bit_length, Some(32));
    assert_eq!(parent.children.len(), 1);
}

// ============================================================================
// build_syntax_tree() Tests
// ============================================================================

#[test]
fn test_build_syntax_tree_empty_stream() {
    use std::collections::HashMap;

    let stream = VvcStream {
        nal_units: vec![],
        sps_map: HashMap::new(),
        pps_map: HashMap::new(),
    };

    let tree = build_syntax_tree(&stream);

    assert_eq!(tree.name, "VVC Bitstream");
    assert_eq!(tree.node_type, SyntaxNodeType::Root);
    // Empty stream still has NAL Units array node, just empty
    assert!(!tree.children.is_empty());
}

#[test]
fn test_build_syntax_tree_with_nal_unit() {
    use std::collections::HashMap;

    let stream = VvcStream {
        nal_units: vec![
            NalUnit {
                header: NalUnitHeader {
                    nal_unit_type: NalUnitType::SpsNut,
                    nuh_layer_id: 0,
                    nuh_temporal_id_plus1: 1,
                },
                offset: 0,
                size: 100,
                payload: vec![],
                raw_payload: vec![],
            },
        ],
        sps_map: HashMap::new(),
        pps_map: HashMap::new(),
    };

    let tree = build_syntax_tree(&stream);

    assert_eq!(tree.name, "VVC Bitstream");
    assert!(tree.children.len() > 0);
}

// ============================================================================
// Clone and Debug Tests
// ============================================================================

#[test]
fn test_syntax_node_clone_basic() {
    let node = SyntaxNode::new("Clone Test", SyntaxNodeType::Field);
    assert_eq!(node.value, None);

    let cloned = node.clone();

    assert_eq!(cloned.name, "Clone Test");
    assert_eq!(cloned.value, node.value);
    assert_eq!(cloned.node_type, node.node_type);
    assert_eq!(cloned.bit_offset, node.bit_offset);
    assert_eq!(cloned.bit_length, node.bit_length);
    assert_eq!(cloned.children.len(), node.children.len());
}

#[test]
fn test_syntax_node_clone_with_children() {
    let mut parent = SyntaxNode::new("Parent", SyntaxNodeType::Structure);
    parent.add_child(SyntaxNode::field("child", "value"));

    let cloned = parent.clone();

    assert_eq!(cloned.children.len(), 1);
    assert_eq!(cloned.children[0].name, "child");
}

#[test]
fn test_syntax_node_debug() {
    let mut node = SyntaxNode::new("Debug", SyntaxNodeType::Structure);
    node.add_child(SyntaxNode::field("field1", "value1"));
    node.add_child(SyntaxNode::field("field2", "value2"));

    let debug_str = format!("{:?}", node);
    assert!(debug_str.contains("Debug"));
    assert!(debug_str.contains("field1"));
    assert!(debug_str.contains("field2"));
}

#[test]
fn test_node_type_debug() {
    let debug_str = format!("{:?}", SyntaxNodeType::SliceHeader);
    assert!(debug_str.contains("SliceHeader"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_syntax_node_empty_name() {
    let node = SyntaxNode::new("", SyntaxNodeType::Field);
    assert_eq!(node.name, "");
}

#[test]
fn test_syntax_node_long_name() {
    let long_name = "Test node with a very long name that exceeds normal bounds";
    let node = SyntaxNode::new(long_name, SyntaxNodeType::Field);
    assert_eq!(node.name, long_name);
}

#[test]
fn test_syntax_node_many_children() {
    let mut parent = SyntaxNode::new("Parent", SyntaxNodeType::Array);

    // Test with a smaller set for compilation
    let items = [
        ("child0", "0"), ("child1", "1"), ("child2", "2"), ("child3", "3"), ("child4", "4"),
        ("child5", "5"), ("child6", "6"), ("child7", "7"), ("child8", "8"), ("child9", "9"),
    ];
    for (name, value) in items {
        parent.add_child(SyntaxNode::field(name, value));
    }

    assert_eq!(parent.children.len(), 10);
    assert!(parent.children.capacity() >= 10);
}

#[test]
fn test_syntax_node_unicode_name() {
    let unicode_name = "VVC 한글 테스트";
    let node = SyntaxNode::new(unicode_name, SyntaxNodeType::Field);
    assert_eq!(node.name, unicode_name);
}

#[test]
fn test_syntax_node_nesting_deep() {
    let mut root = SyntaxNode::new("Root", SyntaxNodeType::Root);
    let mut level1 = SyntaxNode::new("L1", SyntaxNodeType::Structure);
    let mut level2 = SyntaxNode::new("L2", SyntaxNodeType::Field);

    // Proper nesting: root -> level1 -> level2 -> level3
    let level3 = SyntaxNode::field("L3", "deepest");
    level2.add_child(level3);
    level1.add_child(level2);
    root.add_child(level1);

    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].name, "L1");
    assert_eq!(root.children[0].node_type, SyntaxNodeType::Structure);
    assert_eq!(root.children[0].children[0].name, "L2");
    assert_eq!(root.children[0].children[0].children[0].name, "L3");
}

#[test]
fn test_syntax_node_capacity_growth() {
    let mut node = SyntaxNode::new("Capacity", SyntaxNodeType::Array);

    let initial_capacity = node.children.capacity();

    let items = [("item0", "0"), ("item1", "1"), ("item2", "2"), ("item3", "3"),
                 ("item4", "4"), ("item5", "5"), ("item6", "6"), ("item7", "7")];
    for (name, value) in items {
        node.add_child(SyntaxNode::field(name, value));
    }

    let new_capacity = node.children.capacity();
    assert!(new_capacity >= initial_capacity);
}

#[test]
fn test_syntax_node_all_node_types() {
    let types = [
        SyntaxNodeType::Root,
        SyntaxNodeType::NalUnit,
        SyntaxNodeType::ParameterSet,
        SyntaxNodeType::SliceHeader,
        SyntaxNodeType::Field,
        SyntaxNodeType::Array,
        SyntaxNodeType::Structure,
    ];

    for node_type in types {
        let node = SyntaxNode::new("Type Test", node_type);
        assert_eq!(node.node_type, node_type);
    }
}
