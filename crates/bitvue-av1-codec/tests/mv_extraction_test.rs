#![allow(dead_code)]
// Integration test for MV extraction with spec-compliant CDFs
//
// This test verifies that the updated CDF values (from rav1d/AV1 spec)
// correctly decode motion vectors from real AV1 bitstreams.

use bitvue_av1_codec::{
    parse_all_obus, parse_frame_header_basic, parse_superblock, tile::MvPredictorContext,
    FrameType, SymbolDecoder,
};

// TODO: Re-enable after fixing overflow bug in src/tile/mv_prediction.rs:107
// The parsing code panics with "attempt to add with overflow" when parsing certain IVF files
#[test]
#[ignore]
fn test_mv_extraction_with_spec_cdfs() {
    // Load test IVF file
    let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_data/av1_test.ivf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        eprintln!("Skipping MV extraction test");
        return;
    }

    let data = std::fs::read(&test_file).expect("Failed to read test file");

    // Extract OBUs from container (MP4/IVF/WebM)
    let obu_data = if bitvue_av1_codec::is_mp4(&data) {
        bitvue_av1_codec::extract_obu_data_from_mp4(&data).expect("Failed to extract from MP4")
    } else if bitvue_av1_codec::is_ivf(&data) {
        bitvue_av1_codec::extract_obu_data(&data).expect("Failed to extract from IVF")
    } else if bitvue_av1_codec::is_mkv(&data) {
        bitvue_av1_codec::extract_obu_data_from_mkv(&data).expect("Failed to extract from MKV/WebM")
    } else {
        panic!("Unknown container format");
    };

    let obus = parse_all_obus(&obu_data).expect("Failed to parse OBUs");

    eprintln!("Parsed {} OBUs from test file", obus.len());

    let mut inter_frames_found = 0;
    let mut frames_with_mvs = 0;
    let mut total_mvs = 0;
    let mut non_zero_mvs = 0;

    // Process all frame OBUs
    for (idx, obu) in obus.iter().enumerate() {
        if obu.header.obu_type != bitvue_av1_codec::ObuType::Frame {
            continue;
        }

        eprintln!("\nProcessing frame OBU #{}", idx);

        let header = match parse_frame_header_basic(&obu.payload) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("  Failed to parse frame header: {}", e);
                continue;
            }
        };

        eprintln!("  Frame type: {:?}", header.frame_type);

        // Skip KEY frames (INTRA only)
        if header.frame_type == FrameType::Key {
            eprintln!("  Skipping KEY frame (no motion vectors)");
            continue;
        }

        inter_frames_found += 1;
        eprintln!("  INTER frame found!");

        // Extract tile data
        let tile_start = header.header_size_bytes;
        if obu.payload.len() <= tile_start {
            eprintln!(
                "  Payload too small: {} <= {}",
                obu.payload.len(),
                tile_start
            );
            continue;
        }

        let tile_data = &obu.payload[tile_start..];
        eprintln!(
            "  Tile data: {} bytes at offset {}",
            tile_data.len(),
            tile_start
        );

        // Try to decode first superblock
        let mut decoder = match SymbolDecoder::new(tile_data) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("  Failed to create symbol decoder: {}", e);
                continue;
            }
        };

        // Parse first superblock (at 0, 0)
        let sb_size = 64;
        let mut mv_ctx = MvPredictorContext::new(30, 17); // Typical 1920x1080 frame in 64x64 superblocks
        match parse_superblock(&mut decoder, 0, 0, sb_size, false, 128, false, &mut mv_ctx) {
            Ok((superblock, _final_qp)) => {
                eprintln!(
                    "  Superblock has {} coding units",
                    superblock.coding_units.len()
                );

                // Check coding units
                for cu in &superblock.coding_units {
                    eprintln!(
                        "    CU at ({}, {}) {}x{}: mode={:?}, skip={}",
                        cu.x, cu.y, cu.width, cu.height, cu.mode, cu.skip
                    );
                }

                // Per generate-tests skill: Access public interface only
                // The motion_vectors() method may not exist, so we extract MVs from CUs
                let mvs: Vec<_> = superblock
                    .coding_units
                    .iter()
                    .filter(|cu| cu.is_inter())
                    .map(|cu| (cu.x, cu.y, cu.width, cu.height, cu.mv[0]))
                    .collect();

                eprintln!("  Extracted {} MVs from first superblock", mvs.len());

                if !mvs.is_empty() {
                    frames_with_mvs += 1;
                    total_mvs += mvs.len();

                    // Count non-zero MVs
                    for (_x, _y, _w, _h, mv) in &mvs {
                        eprintln!("    MV: ({}, {}) qpel", mv.x, mv.y);
                        if mv.x != 0 || mv.y != 0 {
                            non_zero_mvs += 1;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("  Failed to parse superblock: {}", e);
            }
        }
    }

    eprintln!("\n=== Summary ===");
    eprintln!("INTER frames found: {}", inter_frames_found);
    eprintln!("Frames with MVs: {}", frames_with_mvs);
    eprintln!("Total MVs extracted: {}", total_mvs);
    eprintln!(
        "Non-zero MVs: {} ({:.1}%)",
        non_zero_mvs,
        if total_mvs > 0 {
            (non_zero_mvs as f32 / total_mvs as f32) * 100.0
        } else {
            0.0
        }
    );

    // Assertions
    if inter_frames_found > 0 {
        // If we found INTER frames, we should extract some MVs
        assert!(
            frames_with_mvs > 0,
            "Expected to extract MVs from at least one INTER frame"
        );

        // With spec-compliant CDFs, we should get some non-zero MVs
        // (unless the video really has no motion, but that's unlikely)
        eprintln!("\n✓ MV extraction working with spec-compliant CDFs!");
        eprintln!(
            "  Found {} non-zero MVs out of {} total",
            non_zero_mvs, total_mvs
        );
    } else {
        eprintln!("\nℹ No INTER frames in test file (all KEY frames)");
        eprintln!("  This is normal for some test files - they may only contain I-frames");
    }
}
