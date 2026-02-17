#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Phase 0 Integration Test - End-to-End Validation
//!
//! Tests the complete pipeline:
//! 1. Load actual AV1 file
//! 2. Parse with syntax parser
//! 3. Verify SyntaxModel structure
//! 4. Validate bit ranges

use std::fs;

#[test]
fn test_phase0_real_file_parsing() {
    // Load real test file (relative to workspace root)
    let test_file = "../../test_data/av1_test.ivf";

    // Skip test if file doesn't exist
    if !std::path::Path::new(test_file).exists() {
        println!("⚠️  Skipping test: {} not found", test_file);
        return;
    }

    let data = fs::read(test_file).expect("Failed to read test file");

    // Extract OBU data from IVF
    let obu_data = bitvue_av1_codec::extract_obu_data(&data).expect("Failed to extract OBU data");

    // Parse bitstream with syntax parser
    let models = bitvue_av1_codec::parse_bitstream_syntax(&obu_data)
        .expect("Failed to parse bitstream syntax");

    // Verify we got multiple OBUs
    assert!(
        models.len() > 0,
        "Expected at least one OBU, got {}",
        models.len()
    );

    println!("✅ Parsed {} OBUs from test file", models.len());

    // Verify each SyntaxModel
    for (i, model) in models.iter().enumerate() {
        // 1. Verify root node exists
        assert!(
            model.nodes.contains_key(&model.root_id),
            "OBU {} missing root node",
            i
        );

        // 2. Verify root has children
        let root = model.get_node(&model.root_id).unwrap();
        assert!(!root.children.is_empty(), "OBU {} root has no children", i);

        // 3. Verify all children exist
        for child_id in &root.children {
            assert!(
                model.get_node(child_id).is_some(),
                "OBU {} missing child node {}",
                i,
                child_id
            );
        }

        // 4. Verify bit ranges are valid
        for (node_id, node) in &model.nodes {
            assert!(
                node.bit_range.end_bit >= node.bit_range.start_bit,
                "OBU {} node {} has invalid bit range: {}-{}",
                i,
                node_id,
                node.bit_range.start_bit,
                node.bit_range.end_bit
            );
        }

        // 5. Check for expected nodes
        let has_obu_header = root
            .children
            .iter()
            .any(|id| model.get_node(id).unwrap().field_name == "obu_header");

        assert!(has_obu_header, "OBU {} missing obu_header node", i);

        println!(
            "  OBU[{}]: {} nodes, root bit range: {}-{}",
            i,
            model.nodes.len(),
            root.bit_range.start_bit,
            root.bit_range.end_bit
        );
    }

    // Find and verify Sequence Header (if present)
    let seq_header_model = models.iter().find(|m| {
        let root = m.get_node(&m.root_id).unwrap();
        root.children.iter().any(|id| {
            m.get_node(id)
                .map(|n| n.field_name == "sequence_header")
                .unwrap_or(false)
        })
    });

    if let Some(model) = seq_header_model {
        println!("\n✅ Found Sequence Header OBU");

        // Find sequence_header node
        let root = model.get_node(&model.root_id).unwrap();
        let seq_node_id = root
            .children
            .iter()
            .find(|id| {
                model
                    .get_node(*id)
                    .map(|n| n.field_name == "sequence_header")
                    .unwrap_or(false)
            })
            .unwrap();

        let seq_node = model.get_node(seq_node_id).unwrap();

        // Verify it has expected children
        let expected_fields = [
            "seq_profile",
            "still_picture",
            "reduced_still_picture_header",
        ];

        for field in &expected_fields {
            let has_field = seq_node.children.iter().any(|id| {
                model
                    .get_node(id)
                    .map(|n| n.field_name == *field)
                    .unwrap_or(false)
            });

            assert!(has_field, "Sequence header missing field: {}", field);
        }

        println!(
            "  Sequence header has {} child nodes",
            seq_node.children.len()
        );
    }

    // Find and verify Frame Header (if present)
    let frame_header_model = models.iter().find(|m| {
        let root = m.get_node(&m.root_id).unwrap();
        root.children.iter().any(|id| {
            m.get_node(id)
                .map(|n| n.field_name == "frame_header")
                .unwrap_or(false)
        })
    });

    if let Some(model) = frame_header_model {
        println!("\n✅ Found Frame Header OBU");

        let root = model.get_node(&model.root_id).unwrap();
        let frame_node_id = root
            .children
            .iter()
            .find(|id| {
                model
                    .get_node(*id)
                    .map(|n| n.field_name == "frame_header")
                    .unwrap_or(false)
            })
            .unwrap();

        let frame_node = model.get_node(frame_node_id).unwrap();

        // Verify it has expected fields
        let expected_fields = ["show_existing_frame"];

        for field in &expected_fields {
            let has_field = frame_node.children.iter().any(|id| {
                model
                    .get_node(id)
                    .map(|n| n.field_name == *field)
                    .unwrap_or(false)
            });

            assert!(has_field, "Frame header missing field: {}", field);
        }

        println!(
            "  Frame header has {} child nodes",
            frame_node.children.len()
        );
    }

    println!("\n🎉 Phase 0 Integration Test PASSED!");
}

#[test]
fn test_phase0_bit_range_accuracy() {
    // Create minimal test data
    let data = vec![
        0x12, 0x00, // Temporal Delimiter OBU
    ];

    let models =
        bitvue_av1_codec::parse_bitstream_syntax(&data).expect("Failed to parse test data");

    assert_eq!(models.len(), 1, "Expected 1 OBU");

    let model = &models[0];

    // Find obu_header node
    let root = model.get_node(&model.root_id).unwrap();
    let header_id = root
        .children
        .iter()
        .find(|id| {
            model
                .get_node(*id)
                .map(|n| n.field_name == "obu_header")
                .unwrap_or(false)
        })
        .expect("obu_header not found");

    let header_node = model.get_node(header_id).unwrap();

    // Verify header bit range is exactly 8 bits
    assert_eq!(
        header_node.bit_range.size_bits(),
        8,
        "OBU header should be exactly 8 bits"
    );

    // Find obu_type field
    let type_node = header_node
        .children
        .iter()
        .find_map(|id| {
            let node = model.get_node(id)?;
            if node.field_name == "obu_type" {
                Some(node)
            } else {
                None
            }
        })
        .expect("obu_type field not found");

    // Verify obu_type is bits 1-5 (4 bits)
    assert_eq!(
        type_node.bit_range.start_bit, 1,
        "obu_type should start at bit 1"
    );
    assert_eq!(
        type_node.bit_range.end_bit, 5,
        "obu_type should end at bit 5"
    );
    assert_eq!(
        type_node.bit_range.size_bits(),
        4,
        "obu_type should be 4 bits"
    );

    // Verify value
    assert!(
        type_node
            .value
            .as_ref()
            .unwrap()
            .contains("TEMPORAL_DELIMITER"),
        "Expected TEMPORAL_DELIMITER, got {:?}",
        type_node.value
    );

    println!("✅ Bit range accuracy verified");
}

#[test]
fn test_phase0_bidirectional_trisync() {
    // Test bidirectional tri-sync: Bit → Syntax Node → Bit
    let data = vec![
        0x0A, 0x05, // OBU header + size
        0b00001001, 0x00, 0x00, 0x00, 0x00, // Sequence header payload
    ];

    let models =
        bitvue_av1_codec::parse_bitstream_syntax(&data).expect("Failed to parse test data");

    assert_eq!(models.len(), 1, "Expected 1 OBU");
    let model = &models[0];

    // Test 1: Find node containing bits 1-5 (obu_type field)
    let test_range = bitvue_core::BitRange::new(1, 5);
    let found_node = model
        .find_nearest_node(&test_range)
        .expect("Should find node for bits 1-5");

    assert_eq!(
        found_node.field_name, "obu_type",
        "Should find obu_type field"
    );
    assert_eq!(
        found_node.bit_range.start_bit, 1,
        "obu_type starts at bit 1"
    );
    assert_eq!(found_node.bit_range.end_bit, 5, "obu_type ends at bit 5");

    println!(
        "✅ Tri-sync: Bit range [1-5] → Found '{}'",
        found_node.field_name
    );

    // Test 2: Find node containing a single bit (bit 3 is in obu_type)
    let single_bit = bitvue_core::BitRange::new(3, 4);
    let found_node = model
        .find_nearest_node(&single_bit)
        .expect("Should find node for bit 3");

    assert_eq!(
        found_node.field_name, "obu_type",
        "Single bit should find obu_type"
    );

    println!("✅ Tri-sync: Bit [3] → Found '{}'", found_node.field_name);

    // Test 3: Find node for obu_has_size_field (bit 6)
    let has_size_bit = bitvue_core::BitRange::new(6, 7);
    let found_node = model
        .find_nearest_node(&has_size_bit)
        .expect("Should find node for bit 6");

    assert_eq!(
        found_node.field_name, "obu_has_size_field",
        "Bit 6 should find obu_has_size_field"
    );

    println!("✅ Tri-sync: Bit [6] → Found '{}'", found_node.field_name);

    // Test 4: Find container node (obu_header spans bits 0-8)
    let header_range = bitvue_core::BitRange::new(0, 8);
    let found_node = model
        .find_nearest_node(&header_range)
        .expect("Should find obu_header container");

    assert_eq!(
        found_node.field_name, "obu_header",
        "Should find obu_header container"
    );

    println!(
        "✅ Tri-sync: Bit range [0-8] → Found container '{}'",
        found_node.field_name
    );

    // Test 5: Verify tightest node selection
    // Clicking anywhere in bits 1-5 should select obu_type, not obu_header
    let mid_bit = bitvue_core::BitRange::new(3, 4);
    let tightest = model.find_nearest_node(&mid_bit).unwrap();

    assert_eq!(
        tightest.field_name, "obu_type",
        "Should select tightest node (obu_type), not parent (obu_header)"
    );
    assert!(
        tightest.bit_range.size_bits() < 8,
        "Tightest node should be smaller than header"
    );

    println!("✅ Tri-sync: Tightest node selection verified");

    println!("\n🎉 Bidirectional Tri-sync Test PASSED!");
}
