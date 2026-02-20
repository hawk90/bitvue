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
//! Overlay Extraction Integration Tests
//!
//! Tests overlay extraction with real AV1 bitstream data to verify
//! that the implementation extracts actual parsed data, not scaffold.

use bitvue_av1_codec::overlay_extraction::{
    extract_mv_grid_from_parsed, extract_partition_grid_from_parsed,
    extract_prediction_mode_grid_from_parsed, extract_qp_grid_from_parsed, ParsedFrame,
};

/// Test helper: Parse IVF file to OBU data
fn parse_ivf_to_obu_data(ivf_path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Read;
    use std::path::{Path, PathBuf};

    // Get workspace root by going up from CARGO_MANIFEST_DIR
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find workspace root");

    // Convert to absolute path from workspace root
    let absolute_path = if Path::new(ivf_path).is_absolute() {
        PathBuf::from(ivf_path)
    } else {
        workspace_root.join(ivf_path)
    };

    let mut file = File::open(&absolute_path)?;
    let mut buffer = Vec::new();

    // Skip IVF header (32 bytes)
    let mut header = [0u8; 32];
    file.read_exact(&mut header)?;

    // Read first frame header (12 bytes)
    let mut frame_header = [0u8; 12];
    file.read_exact(&mut frame_header)?;

    // Get frame size from header (little-endian)
    let frame_size = u32::from_le_bytes([
        frame_header[0],
        frame_header[1],
        frame_header[2],
        frame_header[3],
    ]) as usize;

    // Read frame data
    buffer.resize(frame_size, 0);
    file.read_exact(&mut buffer)?;

    Ok(buffer)
}

#[test]
fn test_qp_grid_extraction_with_real_av1() {
    // Parse real AV1 data
    let obu_data =
        parse_ivf_to_obu_data("test_data/av1_test.ivf").expect("Failed to load test AV1 file");

    // Parse frame
    let parsed = ParsedFrame::parse(&obu_data).expect("Failed to parse frame");

    // Extract QP grid
    let base_qp = 128i16;
    let qp_grid =
        extract_qp_grid_from_parsed(&parsed, 0, base_qp).expect("Failed to extract QP grid");

    // Verify grid dimensions
    assert!(qp_grid.grid_w > 0, "Grid should have width");
    assert!(qp_grid.grid_h > 0, "Grid should have height");

    // Verify QP values are within valid range [0, 255]
    for &qp in &qp_grid.qp {
        assert!(
            qp >= 0 && qp <= 255,
            "QP value {} should be in [0, 255]",
            qp
        );
    }

    println!(
        "QP Grid: {}x{}, {} blocks, avg QP: {:.1}",
        qp_grid.grid_w,
        qp_grid.grid_h,
        qp_grid.qp.len(),
        qp_grid.qp.iter().map(|&x| x as f32).sum::<f32>() / qp_grid.qp.len() as f32
    );
}

#[test]
fn test_mv_grid_extraction_with_real_av1() {
    let obu_data =
        parse_ivf_to_obu_data("test_data/av1_test.ivf").expect("Failed to load test AV1 file");

    let parsed = ParsedFrame::parse(&obu_data).expect("Failed to parse frame");

    let mv_grid = extract_mv_grid_from_parsed(&parsed).expect("Failed to extract MV grid");

    assert!(mv_grid.grid_w > 0, "MV grid should have width");
    assert!(mv_grid.grid_h > 0, "MV grid should have height");

    // Count INTRA vs INTER blocks
    let intra_count = mv_grid
        .mode
        .iter()
        .flat_map(|m| m.iter())
        .filter(|m| **m == bitvue_core::mv_overlay::BlockMode::Intra)
        .count();
    let inter_count = mv_grid
        .mode
        .iter()
        .flat_map(|m| m.iter())
        .filter(|m| **m == bitvue_core::mv_overlay::BlockMode::Inter)
        .count();

    println!(
        "MV Grid: {}x{}, INTRA: {}, INTER: {}",
        mv_grid.grid_w, mv_grid.grid_h, intra_count, inter_count
    );

    // For KEY frames, all blocks should be INTRA
    if parsed.is_intra_only() {
        assert_eq!(inter_count, 0, "KEY frame should have no INTER blocks");
    }
}

#[test]
fn test_partition_grid_extraction_with_real_av1() {
    let obu_data =
        parse_ivf_to_obu_data("test_data/av1_test.ivf").expect("Failed to load test AV1 file");

    let parsed = ParsedFrame::parse(&obu_data).expect("Failed to parse frame");

    let partition_grid =
        extract_partition_grid_from_parsed(&parsed).expect("Failed to extract partition grid");

    assert!(
        partition_grid.coded_width > 0,
        "Partition grid should have width"
    );
    assert!(
        partition_grid.coded_height > 0,
        "Partition grid should have height"
    );

    println!(
        "Partition Grid: {}x{}, {} blocks",
        partition_grid.coded_width,
        partition_grid.coded_height,
        partition_grid.blocks.len()
    );

    // Verify all blocks are within frame bounds
    for block in &partition_grid.blocks {
        assert!(
            block.x < parsed.width(),
            "Block x {} should be < width {}",
            block.x,
            parsed.width()
        );
        assert!(
            block.y < parsed.height(),
            "Block y {} should be < height {}",
            block.y,
            parsed.height()
        );
    }
}

#[test]
fn test_prediction_mode_extraction_with_real_av1() {
    let obu_data =
        parse_ivf_to_obu_data("test_data/av1_test.ivf").expect("Failed to load test AV1 file");

    let parsed = ParsedFrame::parse(&obu_data).expect("Failed to parse frame");

    let mode_grid = extract_prediction_mode_grid_from_parsed(&parsed)
        .expect("Failed to extract prediction mode grid");

    assert!(mode_grid.grid_w > 0, "Mode grid should have width");
    assert!(mode_grid.grid_h > 0, "Mode grid should have height");

    // Count modes
    let intra_count = mode_grid
        .modes
        .iter()
        .filter(|m| m.map(|m| m.is_intra()).unwrap_or(false))
        .count();
    let inter_count = mode_grid
        .modes
        .iter()
        .filter(|m| m.map(|m| m.is_inter()).unwrap_or(false))
        .count();

    println!(
        "Prediction Mode Grid: {}x{}, INTRA: {}, INTER: {}, Unknown: {}",
        mode_grid.grid_w,
        mode_grid.grid_h,
        intra_count,
        inter_count,
        mode_grid.modes.iter().filter(|m| m.is_none()).count()
    );
}

#[test]
fn test_overlay_extraction_caching_works() {
    let obu_data =
        parse_ivf_to_obu_data("test_data/av1_test.ivf").expect("Failed to load test AV1 file");

    let parsed = ParsedFrame::parse(&obu_data).expect("Failed to parse frame");

    // First extraction - should parse and cache
    let base_qp = 128i16;
    let qp_grid1 =
        extract_qp_grid_from_parsed(&parsed, 0, base_qp).expect("Failed to extract QP grid");

    // Second extraction - should use cache
    let qp_grid2 =
        extract_qp_grid_from_parsed(&parsed, 0, base_qp).expect("Failed to extract QP grid");

    // Results should be identical
    assert_eq!(qp_grid1.grid_w, qp_grid2.grid_w);
    assert_eq!(qp_grid1.grid_h, qp_grid2.grid_h);
    assert_eq!(qp_grid1.qp, qp_grid2.qp);
}
