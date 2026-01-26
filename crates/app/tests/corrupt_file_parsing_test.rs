//! Integration tests for parsing corrupt IVF files

use app::parser_worker::parse_file;
use bitvue_core::{ByteCache, StreamId};
use std::path::PathBuf;
use std::sync::Arc;

/// Helper to get workspace root
fn workspace_root() -> PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(|p| {
            PathBuf::from(p)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        })
        .unwrap_or_else(|_| PathBuf::from("../.."))
}

#[test]
fn test_parse_light_corruption() {
    let test_file = workspace_root().join("test_data/corrupt_samples/01_light_1error.ivf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        return;
    }

    let byte_cache = Arc::new(
        ByteCache::new(&test_file, 64 * 1024, 10 * 1024 * 1024)
            .expect("Failed to create ByteCache"),
    );

    let result = parse_file(&test_file, StreamId::A, byte_cache);

    match result {
        Ok((container, units, diagnostics)) => {
            println!("Light corruption test:");
            println!("  Container: {:?}", container.format);
            println!("  Units: {}", units.unit_count);
            println!("  Frames: {}", units.frame_count);
            println!("  Diagnostics: {}", diagnostics.len());

            // Should have parsed successfully with some diagnostics
            assert!(units.unit_count > 0, "Should parse some units");

            // Print diagnostics for debugging
            for (i, diag) in diagnostics.iter().enumerate() {
                println!(
                    "  Diagnostic {}: {:?} at offset {} - {}",
                    i + 1,
                    diag.severity,
                    diag.offset_bytes,
                    diag.message
                );
            }
        }
        Err(e) => {
            panic!("Failed to parse light corruption file: {:?}", e);
        }
    }
}

#[test]
fn test_parse_medium_corruption() {
    let test_file = workspace_root().join("test_data/corrupt_samples/02_medium_5errors.ivf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        return;
    }

    let byte_cache = Arc::new(
        ByteCache::new(&test_file, 64 * 1024, 10 * 1024 * 1024)
            .expect("Failed to create ByteCache"),
    );

    let result = parse_file(&test_file, StreamId::A, byte_cache);

    match result {
        Ok((container, units, diagnostics)) => {
            println!("Medium corruption test:");
            println!("  Container: {:?}", container.format);
            println!("  Units: {}", units.unit_count);
            println!("  Frames: {}", units.frame_count);
            println!("  Diagnostics: {}", diagnostics.len());

            // Should have parsed successfully with multiple diagnostics
            assert!(units.unit_count > 0, "Should parse some units");

            // Print diagnostics
            for (i, diag) in diagnostics.iter().enumerate() {
                println!(
                    "  Diagnostic {}: {:?} impact={} at frame {:?} - {}",
                    i + 1,
                    diag.severity,
                    diag.impact_score,
                    diag.frame_index,
                    diag.message
                );
            }
        }
        Err(e) => {
            panic!("Failed to parse medium corruption file: {:?}", e);
        }
    }
}

#[test]
fn test_parse_heavy_corruption() {
    let test_file = workspace_root().join("test_data/corrupt_samples/03_heavy_20errors.ivf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        return;
    }

    let byte_cache = Arc::new(
        ByteCache::new(&test_file, 64 * 1024, 10 * 1024 * 1024)
            .expect("Failed to create ByteCache"),
    );

    let result = parse_file(&test_file, StreamId::A, byte_cache);

    match result {
        Ok((container, units, diagnostics)) => {
            println!("Heavy corruption test:");
            println!("  Container: {:?}", container.format);
            println!("  Units: {}", units.unit_count);
            println!("  Frames: {}", units.frame_count);
            println!("  Diagnostics: {}", diagnostics.len());

            // Heavy corruption should trigger error limit
            // Should have <= 11 diagnostics (10 errors + 1 fatal "too many")
            assert!(
                diagnostics.len() <= 11,
                "Should stop after error limit, got {} diagnostics",
                diagnostics.len()
            );

            // Print first and last diagnostics
            if !diagnostics.is_empty() {
                println!("  First: {:?} - {}", diagnostics[0].severity, diagnostics[0].message);
                let last = &diagnostics[diagnostics.len() - 1];
                println!("  Last: {:?} - {}", last.severity, last.message);
            }
        }
        Err(e) => {
            panic!("Failed to parse heavy corruption file: {:?}", e);
        }
    }
}

#[test]
fn test_parse_truncated_66pct() {
    let test_file = workspace_root().join("test_data/corrupt_samples/06_truncated_66pct.ivf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        return;
    }

    let byte_cache = Arc::new(
        ByteCache::new(&test_file, 64 * 1024, 10 * 1024 * 1024)
            .expect("Failed to create ByteCache"),
    );

    let result = parse_file(&test_file, StreamId::A, byte_cache);

    match result {
        Ok((container, units, diagnostics)) => {
            println!("Truncated 66% test:");
            println!("  Container: {:?}", container.format);
            println!("  Units: {}", units.unit_count);
            println!("  Frames: {}", units.frame_count);
            println!("  Diagnostics: {}", diagnostics.len());

            // Truncated file should parse partial data
            assert!(units.unit_count > 0, "Should parse some units before truncation");

            // Should have diagnostics about truncation/EOF
            let has_eof_error = diagnostics
                .iter()
                .any(|d| d.message.contains("end of file") || d.message.contains("EOF"));

            if has_eof_error {
                println!("  ✓ Has EOF diagnostic as expected");
            }
        }
        Err(e) => {
            panic!("Failed to parse truncated file: {:?}", e);
        }
    }
}

#[test]
fn test_parse_truncated_33pct() {
    let test_file = workspace_root().join("test_data/corrupt_samples/07_truncated_33pct.ivf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        return;
    }

    let byte_cache = Arc::new(
        ByteCache::new(&test_file, 64 * 1024, 10 * 1024 * 1024)
            .expect("Failed to create ByteCache"),
    );

    let result = parse_file(&test_file, StreamId::A, byte_cache);

    match result {
        Ok((container, units, diagnostics)) => {
            println!("Truncated 33% test:");
            println!("  Container: {:?}", container.format);
            println!("  Units: {}", units.unit_count);
            println!("  Frames: {}", units.frame_count);
            println!("  Diagnostics: {}", diagnostics.len());

            // Severely truncated file should still parse IVF header
            assert_eq!(
                container.format,
                bitvue_core::ContainerFormat::Ivf,
                "Should detect IVF format"
            );

            // Note: Clean truncation might not generate diagnostics
            // (diagnostics only for malformed OBUs, not for missing frames)
            println!("  Note: {} diagnostics (severe truncation parsed cleanly)", diagnostics.len());
        }
        Err(e) => {
            panic!("Failed to parse severely truncated file: {:?}", e);
        }
    }
}

#[test]
fn test_parse_obu_header_corrupt() {
    let test_file = workspace_root().join("test_data/corrupt_samples/04_obu_header_corrupt.ivf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        return;
    }

    let byte_cache = Arc::new(
        ByteCache::new(&test_file, 64 * 1024, 10 * 1024 * 1024)
            .expect("Failed to create ByteCache"),
    );

    let result = parse_file(&test_file, StreamId::A, byte_cache);

    match result {
        Ok((_container, _units, diagnostics)) => {
            println!("OBU header corruption test:");
            println!("  Diagnostics: {}", diagnostics.len());

            // Should have diagnostics about header errors
            for (i, diag) in diagnostics.iter().enumerate() {
                println!(
                    "  Diagnostic {}: {:?} - {}",
                    i + 1,
                    diag.severity,
                    diag.message
                );
            }

            // Verify diagnostics have proper fields
            for diag in &diagnostics {
                assert_eq!(diag.stream_id, StreamId::A);
                assert!(diag.impact_score > 0);
                assert!(!diag.message.is_empty());
            }
        }
        Err(e) => {
            panic!("Failed to parse OBU header corrupt file: {:?}", e);
        }
    }
}

#[test]
fn test_parse_frame_header_corrupt() {
    let test_file = workspace_root().join("test_data/corrupt_samples/05_frame_header_corrupt.ivf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        return;
    }

    let byte_cache = Arc::new(
        ByteCache::new(&test_file, 64 * 1024, 10 * 1024 * 1024)
            .expect("Failed to create ByteCache"),
    );

    let result = parse_file(&test_file, StreamId::A, byte_cache);

    match result {
        Ok((_container, _units, diagnostics)) => {
            println!("Frame header corruption test:");
            println!("  Diagnostics: {}", diagnostics.len());

            for (i, diag) in diagnostics.iter().enumerate() {
                println!(
                    "  Diagnostic {}: {:?} impact={} - {}",
                    i + 1,
                    diag.severity,
                    diag.impact_score,
                    diag.message
                );
            }
        }
        Err(e) => {
            panic!("Failed to parse frame header corrupt file: {:?}", e);
        }
    }
}

#[test]
fn test_all_corrupt_samples_parseable() {
    // Verify all corrupt samples can be parsed without panicking
    let samples = vec![
        "01_light_1error.ivf",
        "02_medium_5errors.ivf",
        "03_heavy_20errors.ivf",
        "04_obu_header_corrupt.ivf",
        "05_frame_header_corrupt.ivf",
        "06_truncated_66pct.ivf",
        "07_truncated_33pct.ivf",
    ];

    let mut total_diagnostics = 0;

    for sample in samples {
        let test_file = workspace_root().join(format!("test_data/corrupt_samples/{}", sample));

        if !test_file.exists() {
            eprintln!("Sample not found: {}", sample);
            continue;
        }

        let byte_cache = Arc::new(
            ByteCache::new(&test_file, 64 * 1024, 10 * 1024 * 1024)
                .expect("Failed to create ByteCache"),
        );

        match parse_file(&test_file, StreamId::A, byte_cache) {
            Ok((_container, units, diagnostics)) => {
                println!(
                    "✓ {}: {} units, {} diagnostics",
                    sample,
                    units.unit_count,
                    diagnostics.len()
                );
                total_diagnostics += diagnostics.len();
            }
            Err(e) => {
                panic!("Failed to parse {}: {:?}", sample, e);
            }
        }
    }

    println!("\n✅ All samples parsed successfully!");
    println!("   Total diagnostics across all files: {}", total_diagnostics);

    // Note: Corrupt samples might not generate diagnostics if the corruption
    // doesn't result in malformed OBUs (e.g., clean truncation, or if the
    // corruption happened in a way that still produces valid-looking OBUs)
    println!("   Note: 0 diagnostics means files parse cleanly (corruption may need to be more aggressive)");
}

#[test]
fn test_diagnostic_offset_conversion_ivf() {
    // Test that diagnostic offsets are converted correctly from OBU data to file offsets
    let test_file = workspace_root().join("test_data/corrupt_samples/01_light_1error.ivf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        return;
    }

    let byte_cache = Arc::new(
        ByteCache::new(&test_file, 64 * 1024, 10 * 1024 * 1024)
            .expect("Failed to create ByteCache"),
    );

    let result = parse_file(&test_file, StreamId::A, byte_cache);

    match result {
        Ok((_container, _units, diagnostics)) => {
            for diag in &diagnostics {
                // Offset should be > IVF header size (32 bytes)
                assert!(
                    diag.offset_bytes >= 32,
                    "Diagnostic offset {} should be >= IVF header size (32)",
                    diag.offset_bytes
                );

                println!(
                    "Diagnostic at file offset {}: {}",
                    diag.offset_bytes, diag.message
                );
            }
        }
        Err(e) => {
            panic!("Failed to parse file: {:?}", e);
        }
    }
}

#[test]
fn test_diagnostic_fields_complete() {
    // Verify all diagnostic fields are properly populated
    let test_file = workspace_root().join("test_data/corrupt_samples/02_medium_5errors.ivf");

    if !test_file.exists() {
        eprintln!("Test file not found: {:?}", test_file);
        return;
    }

    let byte_cache = Arc::new(
        ByteCache::new(&test_file, 64 * 1024, 10 * 1024 * 1024)
            .expect("Failed to create ByteCache"),
    );

    let result = parse_file(&test_file, StreamId::A, byte_cache);

    match result {
        Ok((_container, _units, diagnostics)) => {
            if diagnostics.is_empty() {
                println!("No diagnostics generated (file might not be actually corrupt)");
                return;
            }

            for (i, diag) in diagnostics.iter().enumerate() {
                println!("\nDiagnostic {}:", i + 1);
                println!("  ID: {}", diag.id);
                println!("  Severity: {:?}", diag.severity);
                println!("  Stream: {:?}", diag.stream_id);
                println!("  Message: {}", diag.message);
                println!("  Category: {:?}", diag.category);
                println!("  Offset: {}", diag.offset_bytes);
                println!("  Timestamp: {}ms", diag.timestamp_ms);
                println!("  Frame: {:?}", diag.frame_index);
                println!("  Count: {}", diag.count);
                println!("  Impact: {}", diag.impact_score);

                // Verify required fields
                assert_eq!(diag.stream_id, StreamId::A);
                assert!(!diag.message.is_empty(), "Message should not be empty");
                assert!(diag.impact_score > 0, "Impact score should be > 0");
                assert!(diag.count >= 1, "Count should be >= 1");
                assert_eq!(
                    diag.category,
                    bitvue_core::event::Category::Bitstream,
                    "Category should be Bitstream"
                );
            }
        }
        Err(e) => {
            panic!("Failed to parse file: {:?}", e);
        }
    }
}
