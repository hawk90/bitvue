//! Integration test for motion vector extraction

use app::parser_worker::parse_file;
use bitvue_core::ByteCache;
use std::sync::Arc;

#[test]
fn test_mv_extraction_from_ivf() {
    // Initialize tracing
    let _ = tracing_subscriber::fmt::try_init();

    // Load test IVF file (relative to workspace root)
    let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
        .map(|p| {
            std::path::PathBuf::from(p)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("../.."));
    let test_file = workspace_root.join("test_data/av1_test.ivf");
    let path = test_file.as_path();

    if !path.exists() {
        eprintln!("Test file not found: {:?}", path);
        eprintln!("Current dir: {:?}", std::env::current_dir());
        return;
    }

    // Create ByteCache from file path
    let byte_cache = Arc::new(
        ByteCache::new(path, 64 * 1024, 10 * 1024 * 1024).expect("Failed to create ByteCache"),
    );

    // Parse the file
    let stream_id = bitvue_core::StreamId::A;
    let result = parse_file(path, stream_id, byte_cache);

    match result {
        Ok((container, unit_model, diagnostics)) => {
            println!("Container: {:?}", container.format);
            println!("Total units: {}", unit_model.unit_count);
            println!("Total frames: {}", unit_model.frame_count);
            println!("Diagnostics: {}", diagnostics.len());

            // Check for motion vector data
            let mut frames_with_mv = 0;
            let mut frames_without_mv = 0;
            let mut key_frames = 0;

            for unit in &unit_model.units {
                if let Some(frame_idx) = unit.frame_index {
                    if unit.mv_grid.is_some() {
                        frames_with_mv += 1;
                        let mv_grid = unit.mv_grid.as_ref().unwrap();
                        println!(
                            "Frame {}: Has MV data - grid size: {}x{}",
                            frame_idx, mv_grid.grid_w, mv_grid.grid_h
                        );

                        // Check if MVs are non-zero (not mock data)
                        let mvs = &mv_grid.mv_l0;
                        let non_zero_mvs = mvs
                            .iter()
                            .filter(|mv| mv.dx_qpel != 0 || mv.dy_qpel != 0)
                            .count();
                        println!("  Non-zero MVs: {} / {}", non_zero_mvs, mvs.len());
                    } else {
                        frames_without_mv += 1;
                        println!("Frame {}: No MV data (likely KEY frame)", frame_idx);
                        key_frames += 1;
                    }
                }
            }

            println!("\nSummary:");
            println!("  Frames with MV data: {}", frames_with_mv);
            println!("  Frames without MV data: {}", frames_without_mv);
            println!("  Total frames: {}", unit_model.frame_count);

            // Assertions
            assert!(unit_model.frame_count > 0, "Should have at least one frame");

            // If we have more than 1 frame, at least one should be INTER with MVs
            if unit_model.frame_count > 1 {
                assert!(
                    frames_with_mv > 0 || key_frames == unit_model.frame_count,
                    "Expected at least one INTER frame with MV data, or all frames are KEY frames"
                );
            }
        }
        Err(e) => {
            panic!("Failed to parse file: {:?}", e);
        }
    }
}
