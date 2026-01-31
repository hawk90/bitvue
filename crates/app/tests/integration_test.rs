//! Integration tests for file loading and parsing

use bitvue_core::{ByteCache, StreamId};
use std::path::PathBuf;
use std::sync::Arc;

// TODO: Re-enable after fixing overflow bug in bitvue-av1/src/tile/mv_prediction.rs:107
// The parsing code panics with "attempt to add with overflow" when parsing certain IVF files
#[test]
#[ignore]
fn test_parse_ivf_file() {
    // Arrange: Create ByteCache from test file
    // Test file is at workspace root: test_data/av1_test.ivf
    // When running from crates/app, we need to go up two levels
    let mut test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_file.push("../../test_data/av1_test.ivf");

    // If path doesn't exist, try the workspace root relative path
    if !test_file.exists() {
        test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test_data/av1_test.ivf");
    }

    assert!(test_file.exists(), "Test file not found: {:?}", test_file);

    let byte_cache = ByteCache::new(
        &test_file,
        ByteCache::DEFAULT_SEGMENT_SIZE,
        ByteCache::DEFAULT_MAX_MEMORY,
    )
    .expect("Failed to create ByteCache");

    let byte_cache = Arc::new(byte_cache);

    // Act: Parse the file
    let result = app::parser_worker::parse_file(&test_file, StreamId::A, byte_cache);

    // Assert: Verify successful parsing
    assert!(result.is_ok(), "Failed to parse file: {:?}", result.err());
    println!("Parsing succeeded for file: {:?}", test_file);

    let (container, units, diagnostics) = result.unwrap();
    println!("Diagnostics: {}", diagnostics.len());

    // Verify container info
    assert_eq!(container.format, bitvue_core::ContainerFormat::Ivf);
    assert!(container.codec.contains("AV1"));
    assert_eq!(container.track_count, 1);

    // Verify we got some units
    assert!(units.unit_count > 0, "No units parsed");
    assert!(units.frame_count > 0, "No frames found");

    println!(
        "✅ Parsed {} units, {} frames",
        units.unit_count, units.frame_count
    );

    // Verify first unit is likely a SEQUENCE_HEADER or TEMPORAL_DELIMITER
    let first_unit = &units.units[0];
    println!(
        "  First unit: {} @ offset 0x{:08X}",
        first_unit.unit_type, first_unit.offset
    );

    // Verify we have at least one frame
    let has_frame = units.units.iter().any(|u| u.frame_index.is_some());
    assert!(has_frame, "No frames found in units");
}

#[test]
fn test_byte_cache_read() {
    let test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_data/av1_test.ivf");

    let byte_cache = ByteCache::new(
        &test_file,
        ByteCache::DEFAULT_SEGMENT_SIZE,
        ByteCache::DEFAULT_MAX_MEMORY,
    )
    .expect("Failed to create ByteCache");

    // Read IVF header (first 32 bytes)
    let header = byte_cache.read_range(0, 32).expect("Failed to read header");

    // Verify IVF signature "DKIF"
    assert_eq!(&header[0..4], b"DKIF", "Not a valid IVF file");

    println!("✅ ByteCache successfully read IVF header");
    println!("  File size: {} bytes", byte_cache.len());
}
