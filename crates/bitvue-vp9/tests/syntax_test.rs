//! VP9 Syntax Tests
//!
//! Tests for VP9 syntax structures to improve coverage.

use bitvue_vp9::syntax;

#[test]
fn test_syntax_node_new() {
    let node = syntax::SyntaxNode::new("test_node", syntax::SyntaxNodeType::Field);

    assert_eq!(node.name, "test_node");
    assert_eq!(node.node_type, syntax::SyntaxNodeType::Field);
}

#[test]
fn test_syntax_node_field() {
    let node = syntax::SyntaxNode::field("width", "1920");

    assert_eq!(node.name, "width");
    assert_eq!(node.value, Some("1920".to_string()));
    assert_eq!(node.node_type, syntax::SyntaxNodeType::Field);
}

#[test]
fn test_syntax_node_add_child() {
    let mut parent = syntax::SyntaxNode::new("parent", syntax::SyntaxNodeType::Structure);
    let child = syntax::SyntaxNode::new("child", syntax::SyntaxNodeType::Field);

    parent.add_child(child);

    assert_eq!(parent.children.len(), 1);
}

#[test]
fn test_syntax_node_with_position() {
    let node =
        syntax::SyntaxNode::new("test", syntax::SyntaxNodeType::Field).with_position(100, 16);

    assert_eq!(node.bit_offset, Some(100));
    assert_eq!(node.bit_length, Some(16));
}

#[test]
fn test_syntax_node_type_variants() {
    let types = vec![
        syntax::SyntaxNodeType::Root,
        syntax::SyntaxNodeType::Superframe,
        syntax::SyntaxNodeType::Frame,
        syntax::SyntaxNodeType::Field,
        syntax::SyntaxNodeType::Array,
        syntax::SyntaxNodeType::Structure,
    ];

    for node_type in types {
        let node = syntax::SyntaxNode::new("test", node_type);
        assert_eq!(node.node_type, node_type);
    }
}

#[test]
fn test_syntax_tree_structure() {
    let mut root = syntax::SyntaxNode::new("root", syntax::SyntaxNodeType::Root);
    let mut frame = syntax::SyntaxNode::new("frame", syntax::SyntaxNodeType::Frame);

    frame.add_child(syntax::SyntaxNode::field("width", "1920"));
    frame.add_child(syntax::SyntaxNode::field("height", "1080"));

    root.add_child(frame);

    assert_eq!(root.children.len(), 1);
    let frame_children = &root.children[0].children;
    assert_eq!(frame_children.len(), 2);
}

#[test]
fn test_syntax_node_value() {
    let mut node = syntax::SyntaxNode::new("test", syntax::SyntaxNodeType::Field);

    assert_eq!(node.value, None);

    node.value = Some("42".to_string());
    assert_eq!(node.value, Some("42".to_string()));
}

#[test]
fn test_empty_syntax_node() {
    let node = syntax::SyntaxNode::new("empty", syntax::SyntaxNodeType::Field);

    assert_eq!(node.name, "empty");
    assert_eq!(node.value, None);
    assert_eq!(node.children.len(), 0);
    assert_eq!(node.bit_offset, None);
    assert_eq!(node.bit_length, None);
}

#[test]
fn test_syntax_node_multiple_children() {
    let mut parent = syntax::SyntaxNode::new("parent", syntax::SyntaxNodeType::Array);

    parent.add_child(syntax::SyntaxNode::new(
        "child1",
        syntax::SyntaxNodeType::Field,
    ));
    parent.add_child(syntax::SyntaxNode::new(
        "child2",
        syntax::SyntaxNodeType::Field,
    ));
    parent.add_child(syntax::SyntaxNode::new(
        "child3",
        syntax::SyntaxNodeType::Field,
    ));

    assert_eq!(parent.children.len(), 3);
}

#[test]
fn test_syntax_node_deep_tree() {
    let mut root = syntax::SyntaxNode::new("root", syntax::SyntaxNodeType::Root);
    let mut level1 = syntax::SyntaxNode::new("level1", syntax::SyntaxNodeType::Structure);
    let mut level2 = syntax::SyntaxNode::new("level2", syntax::SyntaxNodeType::Structure);
    let level3 = syntax::SyntaxNode::new("level3", syntax::SyntaxNodeType::Field);

    level2.add_child(level3);
    level1.add_child(level2);
    root.add_child(level1);

    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].children[0].children.len(), 1);
}

#[test]
fn test_syntax_node_field_variants() {
    let fields = vec![
        ("string_value", "test"),
        ("number_value", "42"),
        ("bool_value", "true"),
        ("enum_value", "KEY_FRAME"),
    ];

    for (name, value) in fields {
        let node = syntax::SyntaxNode::field(name, value);
        assert_eq!(node.name, name);
        assert_eq!(node.value, Some(value.to_string()));
    }
}

#[test]
fn test_syntax_node_with_all_fields() {
    let node = syntax::SyntaxNode::new("test", syntax::SyntaxNodeType::Field).with_position(0, 32);

    assert_eq!(node.name, "test");
    assert_eq!(node.node_type, syntax::SyntaxNodeType::Field);
    assert_eq!(node.bit_offset, Some(0));
    assert_eq!(node.bit_length, Some(32));
    assert_eq!(node.value, None);
}

#[test]
fn test_syntax_node_array_type() {
    let mut array = syntax::SyntaxNode::new("array", syntax::SyntaxNodeType::Array);

    array.add_child(syntax::SyntaxNode::field("item0", "value0"));
    array.add_child(syntax::SyntaxNode::field("item1", "value1"));

    assert_eq!(array.node_type, syntax::SyntaxNodeType::Array);
    assert_eq!(array.children.len(), 2);
}

#[test]
fn test_syntax_node_frame() {
    let mut frame = syntax::SyntaxNode::new("frame", syntax::SyntaxNodeType::Frame);

    frame.add_child(syntax::SyntaxNode::field("frame_type", "KEY_FRAME"));
    frame.add_child(syntax::SyntaxNode::field("show_frame", "true"));
    frame.add_child(syntax::SyntaxNode::field("width", "1920"));
    frame.add_child(syntax::SyntaxNode::field("height", "1080"));

    assert_eq!(frame.node_type, syntax::SyntaxNodeType::Frame);
    assert_eq!(frame.children.len(), 4);
}
