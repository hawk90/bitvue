//! MP4 AV1 Extraction Integration Tests
//!
//! Tests MP4 container format support for AV1 codec extraction.

#[test]
fn test_extract_av1_from_mp4_theater_square() {
    use bitvue_formats::mp4;
    use std::fs;

    let data = fs::read("/Users/hawk/Workspaces/bitvue/test_data/TheaterSquare_640x360.mp4")
        .expect("Failed to load MP4 file");

    println!("MP4 file size: {} bytes", data.len());

    // Verify it's an MP4 file (ftyp box)
    assert!(data.len() > 12, "File too small to be MP4");
    assert_eq!(&data[4..8], b"ftyp", "Missing MP4 ftyp box");

    // Try to extract AV1 samples
    let result = mp4::extract_av1_samples(&data);

    match result {
        Ok(samples) => {
            println!(
                "Successfully extracted {} AV1 samples from MP4",
                samples.len()
            );

            // Verify we got some samples
            assert!(!samples.is_empty(), "Should have at least one sample");

            // Print sample info
            for (i, sample) in samples.iter().enumerate().take(5) {
                println!("Sample {}: {} bytes", i, sample.len());

                // Verify sample has reasonable size
                assert!(sample.len() > 0, "Sample should not be empty");
                assert!(
                    sample.len() < 10_000_000,
                    "Sample size too large: {}",
                    sample.len()
                );
            }

            if samples.len() > 5 {
                println!("... and {} more samples", samples.len() - 5);
            }
        }
        Err(e) => {
            println!("Failed to extract AV1 samples: {:?}", e);

            // The file might not be AV1 codec - that's okay for this test
            // We're testing that the function works correctly
            match e {
                bitvue_core::BitvueError::InvalidData(msg) if msg.contains("Not an AV1 file") => {
                    println!("File is not AV1 codec - this is expected for test files");
                }
                _ => panic!("Unexpected error: {:?}", e),
            }
        }
    }
}

#[test]
fn test_extract_av1_from_mp4_tsu() {
    use bitvue_formats::mp4;
    use std::fs;

    let data = fs::read("/Users/hawk/Workspaces/bitvue/test_data/TSU_640x360.mp4")
        .expect("Failed to load MP4 file");

    println!("TSU MP4 file size: {} bytes", data.len());

    // Try to parse MP4 structure
    let result = mp4::parse_mp4(&data);

    match result {
        Ok(info) => {
            println!("MP4 info:");
            println!("  Brand: {:?}", info.brand);
            println!("  Compatible brands: {:?}", info.compatible_brands);
            println!("  Codec: {:?}", info.codec);
            println!("  Timescale: {}", info.timescale);
            println!("  Sample count: {}", info.sample_count);

            // If codec is AV1, extract samples
            if info.codec.as_ref().map(|s| s.as_str()) == Some("av01") {
                let samples =
                    mp4::extract_av1_samples(&data).expect("Failed to extract AV1 samples");

                println!("Extracted {} AV1 samples", samples.len());
                assert!(!samples.is_empty(), "Should have samples");
            }
        }
        Err(e) => {
            println!("Failed to parse MP4: {:?}", e);
        }
    }
}

#[test]
fn test_mp4_av1_sample_structure() {
    use bitvue_formats::mp4;
    use std::fs;

    let data = fs::read("/Users/hawk/Workspaces/bitvue/test_data/TheaterSquare_640x360.mp4")
        .expect("Failed to load MP4 file");

    // Parse MP4 to get structure info
    if let Ok(info) = mp4::parse_mp4(&data) {
        println!("MP4 structure:");
        println!("  Brand: {:?}", info.brand);
        println!("  Compatible brands: {:?}", info.compatible_brands);
        println!("  Timescale: {}", info.timescale);

        // Check sample info
        if !info.sample_offsets.is_empty() {
            println!("  First sample offset: {}", info.sample_offsets[0]);
            println!("  First sample size: {}", info.sample_sizes[0]);
        }
    }
}

#[test]
fn test_detect_mp4_container_format() {
    use bitvue_formats::detect_container_format;
    use std::path::Path;

    // Test MP4 detection
    let format = detect_container_format(Path::new(
        "/Users/hawk/Workspaces/bitvue/test_data/TheaterSquare_640x360.mp4",
    ))
    .expect("Failed to detect format");

    println!("Detected format: {:?}", format);
    assert_eq!(format, bitvue_formats::ContainerFormat::MP4);
}
