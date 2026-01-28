//! VVC Syntax Tests
//!
//! Tests for VVC syntax structures to improve coverage.

use bitvue_vvc::syntax;

#[test]
fn test_syntax_node_new() {
    let node = syntax::SyntaxNode::new("ctu", syntax::SyntaxNodeType::Structure);

    assert_eq!(node.name, "ctu");
    assert_eq!(node.node_type, syntax::SyntaxNodeType::Structure);
}

#[test]
fn test_syntax_node_field() {
    let node = syntax::SyntaxNode::field("slice_address", "0");

    assert_eq!(node.name, "slice_address");
    assert_eq!(node.value, Some("0".to_string()));
    assert_eq!(node.node_type, syntax::SyntaxNodeType::Field);
}

#[test]
fn test_syntax_node_add_child() {
    let mut parent = syntax::SyntaxNode::new("picture", syntax::SyntaxNodeType::Root);
    let child = syntax::SyntaxNode::new("ctu", syntax::SyntaxNodeType::Structure);

    parent.add_child(child);

    assert_eq!(parent.children.len(), 1);
}

#[test]
fn test_syntax_node_type_variants() {
    let types = vec![
        syntax::SyntaxNodeType::Root,
        syntax::SyntaxNodeType::NalUnit,
        syntax::SyntaxNodeType::ParameterSet,
        syntax::SyntaxNodeType::PictureHeader,
        syntax::SyntaxNodeType::SliceHeader,
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
    let mut root = syntax::SyntaxNode::new("vvc", syntax::SyntaxNodeType::Root);
    let mut slice = syntax::SyntaxNode::new("slice", syntax::SyntaxNodeType::SliceHeader);
    let mut ctu = syntax::SyntaxNode::new("ctu", syntax::SyntaxNodeType::Structure);

    ctu.add_child(syntax::SyntaxNode::field("x", "0"));
    ctu.add_child(syntax::SyntaxNode::field("y", "0"));
    ctu.add_child(syntax::SyntaxNode::field("size", "64"));

    slice.add_child(ctu);
    root.add_child(slice);

    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].children[0].children.len(), 3);
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
    let mut root = syntax::SyntaxNode::new("video", syntax::SyntaxNodeType::Root);
    let mut seq = syntax::SyntaxNode::new("sequence", syntax::SyntaxNodeType::ParameterSet);
    let mut pic = syntax::SyntaxNode::new("picture", syntax::SyntaxNodeType::PictureHeader);
    let slice = syntax::SyntaxNode::new("slice", syntax::SyntaxNodeType::SliceHeader);

    slice.add_child(syntax::SyntaxNode::field("index", "0"));
    pic.add_child(slice);
    seq.add_child(pic);
    root.add_child(seq);

    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].children[0].children.len(), 1);
    assert_eq!(root.children[0].children[0].children[0].children.len(), 1);
}

#[test]
fn test_syntax_node_field_variants() {
    let fields = vec![
        ("unsigned", "12345"),
        ("signed", "-100"),
        ("bool", "true"),
        ("enum", "TRAIL_R"),
    ];

    for (name, value) in fields {
        let node = syntax::SyntaxNode::field(name, value);
        assert_eq!(node.name, name);
        assert_eq!(node.value, Some(value.to_string()));
    }
}

#[test]
fn test_syntax_node_with_multiple_fields() {
    let mut node = syntax::SyntaxNode::new("sps", syntax::SyntaxNodeType::ParameterSet);

    node.add_child(syntax::SyntaxNode::field("sps_id", "0"));
    node.add_child(syntax::SyntaxNode::field("width", "1920"));
    node.add_child(syntax::SyntaxNode::field("height", "1080"));
    node.add_child(syntax::SyntaxNode::field("chroma_format", "YUV420"));
    node.add_child(syntax::SyntaxNode::field("bit_depth", "8"));

    assert_eq!(node.children.len(), 5);
}

#[test]
fn test_syntax_node_nal_unit_type() {
    let types = vec![
        syntax::SyntaxNodeType::Root,
        syntax::SyntaxNodeType::NalUnit,
        syntax::SyntaxNodeType::ParameterSet,
    ];

    for node_type in types {
        let node = syntax::SyntaxNode::new("test", node_type);
        assert_eq!(node.node_type, node_type);
        assert_eq!(node.name, "test");
        assert_eq!(node.children.len(), 0);
    }
}

#[test]
fn test_syntax_node_array_type() {
    let mut array = syntax::SyntaxNode::new("array", syntax::SyntaxNodeType::Array);

    array.add_child(syntax::SyntaxNode::field("item0", "1"));
    array.add_child(syntax::SyntaxNode::field("item1", "2"));

    assert_eq!(array.node_type, syntax::SyntaxNodeType::Array);
    assert_eq!(array.children.len(), 2);
}

#[test]
fn test_complex_vvc_tree() {
    let mut vvc = syntax::SyntaxNode::new("vvc_bitstream", syntax::SyntaxNodeType::Root);

    // Add VPS
    let mut vps = syntax::SyntaxNode::new("vps", syntax::SyntaxNodeType::ParameterSet);
    vps.add_child(syntax::SyntaxNode::field("vps_id", "0"));
    vps.add_child(syntax::SyntaxNode::field("max_layers", "1"));
    vvc.add_child(vps);

    // Add SPS
    let mut sps = syntax::SyntaxNode::new("sps", syntax::SyntaxNodeType::ParameterSet);
    sps.add_child(syntax::SyntaxNode::field("sps_id", "0"));
    sps.add_child(syntax::SyntaxNode::field("profile_idc", "1"));
    vvc.add_child(sps);

    // Add PPS
    let mut pps = syntax::SyntaxNode::new("pps", syntax::SyntaxNodeType::ParameterSet);
    pps.add_child(syntax::SyntaxNode::field("pps_id", "0"));
    pps.add_child(syntax::SyntaxNode::field("num_ref_idx_l0", "1"));
    vvc.add_child(pps);

    assert_eq!(vvc.children.len(), 3);
    assert_eq!(vvc.children[0].name, "vps");
    assert_eq!(vvc.children[1].name, "sps");
    assert_eq!(vvc.children[2].name, "pps");
}
