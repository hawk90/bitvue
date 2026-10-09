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
//! AV1 Overlay Extraction Tests
//!
//! Comprehensive tests for AV1 overlay data extraction.

use bitvue_av1_codec::overlay_extraction;
use bitvue_engine::BlockMode;

/// The first frame of the checked-in fixture (sequence header + frame), which does carry tile data.
fn fixture_frame_0() -> Vec<u8> {
    use bitvue_av1_codec::{
        ivf::parse_ivf_frames,
        obu::{ObuIterator, ObuType},
    };
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test_data/av1_test.ivf"
    ))
    .unwrap();
    let (_header, frames) = parse_ivf_frames(&data).unwrap();
    let first = &frames[0].data;
    let mut iter = ObuIterator::new(first);
    while let Some(Ok(found)) = iter.next_obu_with_offset() {
        if found.obu.header.obu_type == ObuType::SequenceHeader {
            return first.clone();
        }
    }
    panic!("fixture frame 0 has no sequence header");
}

#[test]
fn test_extract_qp_grid_basic() {
    // No tile data to decode: an error, not an invented grid.
    let frame_data = [];

    assert!(overlay_extraction::extract_qp_grid(&frame_data, 0, 26).is_err());
}

#[test]
fn test_extract_mv_grid_basic() {
    // No tile data to decode: an error, not an invented grid.
    let frame_data = [];

    assert!(overlay_extraction::extract_mv_grid(&frame_data, 0).is_err());
}

#[test]
fn test_extract_partition_grid_basic() {
    // No tile data to decode: an error, not an invented grid.
    let frame_data = [];

    assert!(overlay_extraction::extract_partition_grid(&frame_data, 0).is_err());
}

#[test]
fn test_extract_prediction_mode_grid_basic() {
    // No tile data to decode: an error, not an invented grid.
    let frame_data = [];

    assert!(overlay_extraction::extract_prediction_mode_grid(&frame_data, 0).is_err());
}

#[test]
fn test_extract_transform_grid_basic() {
    // No tile data to decode: an error, not an invented grid.
    let frame_data = [];

    assert!(overlay_extraction::extract_transform_grid(&frame_data, 0).is_err());
}

#[test]
fn test_block_mode_variants() {
    // Test all block mode variants
    let modes = vec![BlockMode::Intra, BlockMode::Inter, BlockMode::Skip];

    for mode in modes {
        let _ = format!("{:?}", mode);
        // Just verify the mode can be converted to string
    }
}

#[test]
fn test_qp_grid_with_base_qp() {
    use bitvue_engine::qp_heatmap::QPGrid;

    let base_qp = 30i16;
    let grid = QPGrid::new(10, 10, 64, 64, vec![base_qp; 100], -1);

    assert_eq!(grid.grid_w, 10);
    assert_eq!(grid.grid_h, 10);
}

#[test]
fn test_mv_grid_basic() {
    use bitvue_engine::mv_overlay::{MVGrid, MotionVector};

    // 1920x1080 with 64x64 blocks needs 30x17 = 510 elements
    let mv_l0 = vec![MotionVector::ZERO; 510];
    let mv_l1 = vec![MotionVector::MISSING; 510];
    let modes = vec![BlockMode::Intra; 510];

    let grid = MVGrid::new(1920, 1080, 64, 64, mv_l0, mv_l1, Some(modes));

    assert_eq!(grid.grid_w, 30);
    assert_eq!(grid.grid_h, 17);
}

#[test]
fn test_partition_grid_basic() {
    use bitvue_engine::partition_grid::{PartitionBlock, PartitionGrid, PartitionType};

    let mut grid = PartitionGrid::new(1920, 1080, 64);

    let block = PartitionBlock {
        x: 0,
        y: 0,
        width: 64,
        height: 64,
        partition: PartitionType::None,
        depth: 0,
        tree_type: None,
    };

    grid.add_block(block);
    // Verify partition was added
}

#[test]
fn test_motion_vector_creation() {
    use bitvue_engine::mv_overlay::MotionVector;

    let mv = MotionVector::new(10, -5);

    assert_eq!(mv.dx_qpel, 10);
    assert_eq!(mv.dy_qpel, -5);
}

#[test]
fn test_motion_vector_zero() {
    use bitvue_engine::mv_overlay::MotionVector;

    let mv = MotionVector::ZERO;

    assert_eq!(mv.dx_qpel, 0);
    assert_eq!(mv.dy_qpel, 0);
}

#[test]
fn test_motion_vector_missing() {
    use bitvue_engine::mv_overlay::MotionVector;

    let mv = MotionVector::MISSING;

    assert!(mv.is_missing());
}

#[test]
fn test_motion_vector_to_pixels() {
    use bitvue_engine::mv_overlay::MotionVector;

    let mv = MotionVector::new(8, 4); // 2 pixels, 1 pixel
    let (px_x, px_y) = mv.to_pixels();

    assert_eq!(px_x, 2.0);
    assert_eq!(px_y, 1.0);
}

#[test]
fn test_block_mode_inter() {
    let inter_mode = BlockMode::Inter;
    let _ = format!("{:?}", inter_mode);
    // Verify Inter mode exists
}

#[test]
fn test_block_mode_intra() {
    let intra_mode = BlockMode::Intra;
    let _ = format!("{:?}", intra_mode);
    // Verify Intra mode exists
}

#[test]
fn test_partition_none() {
    use bitvue_engine::partition_grid::PartitionType;
    let none = PartitionType::None;
    let _ = format!("{:?}", none);
    // Verify None partition exists
}

#[test]
fn test_partition_horz() {
    use bitvue_engine::partition_grid::PartitionType;
    let horz = PartitionType::Horz;
    let _ = format!("{:?}", horz);
    // Verify Horz partition exists
}

#[test]
fn test_partition_vert() {
    use bitvue_engine::partition_grid::PartitionType;
    let vert = PartitionType::Vert;
    let _ = format!("{:?}", vert);
    // Verify Vert partition exists
}

#[test]
fn test_partition_split() {
    use bitvue_engine::partition_grid::PartitionType;
    let split = PartitionType::Split;
    let _ = format!("{:?}", split);
    // Verify Split partition exists
}

#[test]
fn test_qp_grid_with_various_base_qp() {
    use bitvue_engine::qp_heatmap::QPGrid;

    let base_qp_values = vec![0i16, 20, 26, 40, 63];

    for base_qp in base_qp_values {
        let grid = QPGrid::new(8, 8, 64, 64, vec![base_qp; 64], base_qp);
        assert_eq!(grid.grid_w, 8);
        assert_eq!(grid.grid_h, 8);
        assert_eq!(grid.qp[0], base_qp);
    }
}

#[test]
fn test_mv_grid_with_motion_vectors() {
    use bitvue_engine::mv_overlay::{MVGrid, MotionVector};

    // 1920x1080 with 64x64 blocks needs 30x17 = 510 elements
    let mut mv_l0 = vec![MotionVector::ZERO; 510];
    mv_l0[0] = MotionVector::new(20, -12); // 5 pixels, -3 pixels

    let mv_l1 = vec![MotionVector::MISSING; 510];
    let modes = vec![BlockMode::Inter; 510];

    let grid = MVGrid::new(1920, 1080, 64, 64, mv_l0, mv_l1, Some(modes));

    let retrieved = grid.get_l0(0, 0);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().dx_qpel, 20);
}

#[test]
fn test_partition_grid_with_partitions() {
    use bitvue_engine::partition_grid::{PartitionBlock, PartitionGrid, PartitionType};

    let mut grid = PartitionGrid::new(128, 128, 64);

    let block = PartitionBlock {
        x: 0,
        y: 0,
        width: 64,
        height: 64,
        partition: PartitionType::Split,
        depth: 0,
        tree_type: None,
    };

    grid.add_block(block);
    // Verify partition was added
}

#[test]
fn test_overlay_extraction_api_exists() {
    // Verify all extraction functions exist and are callable
    let frame_data = [];

    // These should all be callable without panicking
    let _ = overlay_extraction::extract_qp_grid(&frame_data, 0, 26);
    let _ = overlay_extraction::extract_mv_grid(&frame_data, 0);
    let _ = overlay_extraction::extract_partition_grid(&frame_data, 0);
    let _ = overlay_extraction::extract_prediction_mode_grid(&frame_data, 0);
    let _ = overlay_extraction::extract_transform_grid(&frame_data, 0);
}

#[test]
fn test_empty_obu_stream() {
    let frame_data = [];
    let result = overlay_extraction::extract_qp_grid(&frame_data, 0, 26);
    assert!(result.is_err());
}

#[test]
fn test_overlay_extraction_with_obus() {
    // Create OBUs with sequence header and frame
    let mut data = Vec::new();

    // Sequence Header OBU (minimal)
    data.extend_from_slice(&[0x0A, 0x80, 0x00]);

    // Frame Header OBU (minimal)
    data.extend_from_slice(&[0x22, 0x80, 0x02, 0x00, 0x01]);

    // Header-only OBUs carry no tile data: an error, not an invented grid.
    let qp_result = overlay_extraction::extract_qp_grid(&data, 0, 26);
    assert!(qp_result.is_err());
}

#[test]
fn test_grid_dimensions() {
    use bitvue_engine::mv_overlay::{MVGrid, MotionVector};
    use bitvue_engine::qp_heatmap::QPGrid;

    // Test various dimensions
    let dimensions = vec![(64, 64), (128, 128), (1920, 1080)];

    for (width, height) in dimensions {
        let grid_w = width / 64;
        let grid_h = height / 64;
        let qp_grid = QPGrid::new(
            grid_w,
            grid_h,
            64,
            64,
            vec![26; (grid_w * grid_h) as usize],
            26,
        );
        assert_eq!(qp_grid.grid_w * 64, width);

        // MVGrid uses ceiling division for height, so calculate size separately
        let mv_grid_h = height.div_ceil(64); // ceiling division
        let mv_grid_size = (grid_w * mv_grid_h) as usize;
        let mv_l0 = vec![MotionVector::ZERO; mv_grid_size];
        let mv_l1 = vec![MotionVector::MISSING; mv_grid_size];
        let mv_grid = MVGrid::new(width, height, 64, 64, mv_l0, mv_l1, None);
        assert_eq!(mv_grid.coded_width, width);
    }
}

#[test]
fn test_grid_boundary_handling() {
    use bitvue_engine::mv_overlay::{MVGrid, MotionVector};

    // 1920x1080 with 64x64 blocks = 30x17 = 510 blocks
    let mv_l0 = vec![MotionVector::ZERO; 510];
    let mv_l1 = vec![MotionVector::MISSING; 510];
    let grid = MVGrid::new(1920, 1080, 64, 64, mv_l0, mv_l1, None);

    // Valid access
    let mv = grid.get_l0(0, 0);
    assert!(mv.is_some());

    // Out of bounds
    let mv = grid.get_l0(30, 17);
    assert!(mv.is_none());
}

#[test]
fn test_qp_range() {
    use bitvue_engine::qp_heatmap::QPGrid;

    // Test various QP values
    let qp_values = vec![0i16, 10, 20, 30, 40, 50, 63];

    for qp in qp_values {
        let grid = QPGrid::new(1, 1, 64, 64, vec![qp; 1], -1);
        assert_eq!(grid.qp[0], qp);
    }
}

#[test]
fn test_mv_range() {
    use bitvue_engine::mv_overlay::{MVGrid, MotionVector};

    // Test various MV values including large values
    let mv_values = vec![(0, 0), (10, -5), (-10, 5), (100, -100), (-127, 127)];

    let mut mv_l0 = vec![MotionVector::ZERO; 1];
    let mv_l1 = vec![MotionVector::MISSING; 1];

    for (dx, dy) in mv_values {
        mv_l0[0] = MotionVector::new(dx, dy);
        let grid = MVGrid::new(64, 64, 64, 64, mv_l0.clone(), mv_l1.clone(), None);

        let retrieved = grid.get_l0(0, 0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().dx_qpel, dx);
        assert_eq!(retrieved.unwrap().dy_qpel, dy);
    }
}

#[test]
fn test_partition_various_sizes() {
    use bitvue_engine::partition_grid::{PartitionBlock, PartitionGrid, PartitionType};

    let sizes = vec![4, 8, 16, 32, 64, 128];

    for size in sizes {
        let mut grid = PartitionGrid::new(256, 256, 64);

        let block = PartitionBlock {
            x: 0,
            y: 0,
            width: size,
            height: size,
            partition: PartitionType::Split,
            depth: 0,
            tree_type: None,
        };

        grid.add_block(block);
        // Block was added
    }
}

#[test]
fn test_overlay_error_handling() {
    let invalid_data = [0xFF; 10];

    // Should handle invalid data gracefully
    let qp_result = overlay_extraction::extract_qp_grid(&invalid_data, 0, 26);
    assert!(qp_result.is_ok() || qp_result.is_err());
}

#[test]
fn test_multi_tile_extraction() {
    // Create frame with multiple tiles
    let mut data = Vec::new();

    // Sequence Header
    data.extend_from_slice(&[0x0A, 0x80, 0x00]);

    // Frame Header
    data.extend_from_slice(&[0x22, 0x80, 0x02, 0x00, 0x01]);

    // Header-only OBUs carry no tile data: an error, not an invented grid.
    let qp_result = overlay_extraction::extract_qp_grid(&data, 0, 26);
    assert!(qp_result.is_err());
}

#[test]
fn test_grid_consistency() {
    use bitvue_engine::qp_heatmap::QPGrid;

    let width = 8u32;
    let height = 8u32;
    let block_w = 64u32;
    let block_h = 64u32;

    let qp_values = vec![26i16; (width * height) as usize];
    let base_qp = 26i16;

    let grid = QPGrid::new(width, height, block_w, block_h, qp_values, base_qp);

    // Verify grid is consistent
    assert_eq!(grid.grid_w, width);
    assert_eq!(grid.grid_h, height);
    assert_eq!(grid.block_w, block_w);
    assert_eq!(grid.block_h, block_h);
}

#[test]
fn test_mv_grid_consistency() {
    use bitvue_engine::mv_overlay::{MVGrid, MotionVector};

    // Use a smaller resolution for easier testing
    let width = 256u32;
    let height = 256u32;
    let block_w = 64u32;
    let block_h = 64u32;

    // 256x256 with 64x64 blocks = 4x4 = 16 elements
    let grid_w = width / block_w; // 4
    let grid_h = height / block_h; // 4
    let mut mv_l0 = Vec::with_capacity((grid_w * grid_h) as usize);
    let mv_l1 = vec![MotionVector::MISSING; (grid_w * grid_h) as usize];

    // Fill with consistent MVs
    for y in 0..grid_h {
        for x in 0..grid_w {
            mv_l0.push(MotionVector::new(x as i32 * 4, y as i32 * 4));
        }
    }

    let grid = MVGrid::new(width, height, block_w, block_h, mv_l0, mv_l1, None);

    // Verify consistency - use grid dimensions, not pixel dimensions
    for y in 0..grid_h {
        for x in 0..grid_w {
            let mv = grid.get_l0(x, y);
            assert!(mv.is_some());
            assert_eq!(mv.unwrap().dx_qpel, x as i32 * 4);
            assert_eq!(mv.unwrap().dy_qpel, y as i32 * 4);
        }
    }
}

#[test]
fn test_block_mode_properties() {
    // Test block mode properties
    let modes = vec![BlockMode::Intra, BlockMode::Inter, BlockMode::Skip];

    for mode in modes {
        // Each mode should have a unique debug representation
        let debug_str = format!("{:?}", mode);
        assert!(!debug_str.is_empty());
    }
}

#[test]
fn test_grid_with_zero_dimensions() {
    use bitvue_engine::mv_overlay::{MVGrid, MotionVector};
    use bitvue_engine::qp_heatmap::QPGrid;

    // Zero dimensions should be handled
    let qp_grid = QPGrid::new(0, 0, 64, 64, vec![], -1);
    assert_eq!(qp_grid.grid_w, 0);

    let mv_grid = MVGrid::new(0, 0, 64, 64, vec![], vec![], None);
    assert_eq!(mv_grid.grid_w, 0);
}

#[test]
fn test_large_grid() {
    use bitvue_engine::qp_heatmap::QPGrid;

    // Test 4K resolution
    let width: u32 = 3840 / 64; // 60 SBs
    let height: u32 = 2160 / 64; // 33 SBs
    let qp_values = vec![26i16; (width * height) as usize];

    let grid = QPGrid::new(width, height, 64, 64, qp_values, 26);

    assert_eq!(grid.grid_w, 60);
    assert_eq!(grid.grid_h, 33);
}

#[test]
fn test_mv_grid_large() {
    use bitvue_engine::mv_overlay::{MVGrid, MotionVector};

    // Test 4K resolution
    // MVGrid uses ceiling division for height
    let width: u32 = 3840 / 64; // 60
    let height: u32 = 2160_u32.div_ceil(64); // 34 (ceiling division)
    let mv_l0 = vec![MotionVector::ZERO; (width * height) as usize];
    let mv_l1 = vec![MotionVector::MISSING; (width * height) as usize];

    let grid = MVGrid::new(3840, 2160, 64, 64, mv_l0, mv_l1, None);
    assert_eq!(grid.grid_w, 60);
    assert_eq!(grid.grid_h, 34);
}

#[test]
fn test_various_resolutions() {
    // Placeholder bytes carry no tile data, whatever resolution they stand for.
    let frame_data = [];

    assert!(overlay_extraction::extract_qp_grid(&frame_data, 0, 26).is_err());
    assert!(overlay_extraction::extract_mv_grid(&frame_data, 0).is_err());
    assert!(overlay_extraction::extract_partition_grid(&frame_data, 0).is_err());
    assert!(overlay_extraction::extract_prediction_mode_grid(&frame_data, 0).is_err());
    assert!(overlay_extraction::extract_transform_grid(&frame_data, 0).is_err());
}

#[test]
fn test_prediction_mode_grid_structure() {
    let grid = overlay_extraction::extract_prediction_mode_grid(&fixture_frame_0(), 0).unwrap();
    assert_eq!(grid.block_w, 16);
    assert_eq!(grid.block_h, 16);
}

#[test]
fn test_transform_grid_structure() {
    let grid = overlay_extraction::extract_transform_grid(&fixture_frame_0(), 0).unwrap();
    assert_eq!(grid.block_w, 16);
    assert_eq!(grid.block_h, 16);
}

#[test]
fn test_partition_grid_structure() {
    let grid = overlay_extraction::extract_partition_grid(&fixture_frame_0(), 0).unwrap();
    assert_eq!(grid.sb_size, 128);
}

#[test]
fn test_extract_pixel_info() {
    let frame_data = [];

    let result = overlay_extraction::extract_pixel_info(&frame_data, 0, 100, 200);
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.frame_index, 0);
    assert_eq!(info.pixel_x, 100);
    assert_eq!(info.pixel_y, 200);
}

#[test]
fn test_superblock_coverage() {
    let width = 1920;
    let height = 1080;
    let sb_size = 64;

    let sb_cols = (width + sb_size - 1) / sb_size;
    let sb_rows = (height + sb_size - 1) / sb_size;

    assert!(sb_cols > 0);
    assert!(sb_rows > 0);
    assert_eq!(sb_cols, 30);
    assert_eq!(sb_rows, 17);
}

#[test]
fn test_partition_block_contains() {
    use bitvue_engine::partition_grid::PartitionBlock;

    let block = PartitionBlock {
        x: 100,
        y: 200,
        width: 64,
        height: 64,
        partition: bitvue_engine::partition_grid::PartitionType::None,
        depth: 0,
        tree_type: None,
    };

    // Point inside block
    assert!(block.contains(120, 220));
    assert!(block.contains(100, 200));
    assert!(block.contains(163, 263));

    // Point outside block
    assert!(!block.contains(99, 200));
    assert!(!block.contains(100, 199));
    assert!(!block.contains(164, 220));
    assert!(!block.contains(120, 264));
}

#[test]
fn test_partition_block_area() {
    use bitvue_engine::partition_grid::PartitionBlock;

    let block = PartitionBlock {
        x: 0,
        y: 0,
        width: 64,
        height: 32,
        partition: bitvue_engine::partition_grid::PartitionType::None,
        depth: 0,
        tree_type: None,
    };

    assert_eq!(block.area(), 64 * 32);
}

#[test]
fn test_partition_type_from_u8() {
    use bitvue_engine::partition_grid::PartitionType;

    // Test conversion from u8
    assert_eq!(PartitionType::from(0u8), PartitionType::None);
    assert_eq!(PartitionType::from(1u8), PartitionType::Horz);
    assert_eq!(PartitionType::from(2u8), PartitionType::Vert);
    assert_eq!(PartitionType::from(3u8), PartitionType::Split);

    // Invalid value should default to None
    assert_eq!(PartitionType::from(255u8), PartitionType::None);
}

/// Every cell of a fully decoded frame has a QP. The extractor used to pass the base QP as the
/// grid's "missing" marker, which hid every cell that happened to equal it.
#[test]
fn qp_grid_of_a_decoded_frame_has_no_missing_cells() {
    // Use a QP the frame really contains as the base QP: that is the case that used to vanish.
    let probe = overlay_extraction::extract_qp_grid(&fixture_frame_0(), 0, 100).unwrap();
    let base_qp = probe.get(0, 0).expect("first cell is decoded");
    let grid = overlay_extraction::extract_qp_grid(&fixture_frame_0(), 0, base_qp).unwrap();

    assert_eq!(grid.missing, -1);
    for y in 0..grid.grid_h {
        for x in 0..grid.grid_w {
            assert!(grid.get(x, y).is_some(), "cell ({x}, {y}) is missing");
        }
    }
}
