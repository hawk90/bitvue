#![allow(dead_code)]
//! Tests for corrupt test data generation
//!
//! Tests for:
//! - Corrupt sample file generation
//! - File size verification
//! - Corruption ratios
//! - Truncation

use std::fs;
use std::path::PathBuf;

const TEST_DATA_DIR: &str = "test_data/corrupt_samples";

#[test]
fn test_corrupt_samples_exist() {
    let samples_dir = PathBuf::from(TEST_DATA_DIR);

    if !samples_dir.exists() {
        eprintln!(
            "⚠️  Corrupt samples not generated. Run: python3 create_corrupt_from_existing.py"
        );
        return;
    }

    // Check that sample files exist
    let expected_files = vec![
        "01_light_1error.ivf",
        "02_medium_5errors.ivf",
        "03_heavy_20errors.ivf",
        "04_obu_header_corrupt.ivf",
        "05_frame_header_corrupt.ivf",
        "06_truncated_66pct.ivf",
        "07_truncated_33pct.ivf",
    ];

    for filename in expected_files {
        let file_path = samples_dir.join(filename);
        assert!(
            file_path.exists(),
            "Expected corrupt sample file not found: {}",
            filename
        );
    }
}

#[test]
fn test_truncated_file_sizes() {
    let samples_dir = PathBuf::from(TEST_DATA_DIR);

    if !samples_dir.exists() {
        eprintln!("⚠️  Corrupt samples not generated. Skipping test.");
        return;
    }

    // Get reference file size
    let reference_file = PathBuf::from("test_data/av1_test.ivf");
    if !reference_file.exists() {
        eprintln!("⚠️  Reference file not found. Skipping test.");
        return;
    }

    let ref_size = fs::metadata(&reference_file)
        .expect("Failed to read reference file")
        .len();

    // Test 66% truncated file
    let truncated_66 = samples_dir.join("06_truncated_66pct.ivf");
    if truncated_66.exists() {
        let size_66 = fs::metadata(&truncated_66).unwrap().len();
        let ratio_66 = size_66 as f64 / ref_size as f64;

        assert!(
            (ratio_66 - 0.66).abs() < 0.01,
            "66% truncated file should be ~66% of original, got {:.2}%",
            ratio_66 * 100.0
        );
    }

    // Test 33% truncated file
    let truncated_33 = samples_dir.join("07_truncated_33pct.ivf");
    if truncated_33.exists() {
        let size_33 = fs::metadata(&truncated_33).unwrap().len();
        let ratio_33 = size_33 as f64 / ref_size as f64;

        assert!(
            (ratio_33 - 0.33).abs() < 0.01,
            "33% truncated file should be ~33% of original, got {:.2}%",
            ratio_33 * 100.0
        );
    }
}

#[test]
fn test_corrupted_file_sizes_match_original() {
    let samples_dir = PathBuf::from(TEST_DATA_DIR);

    if !samples_dir.exists() {
        eprintln!("⚠️  Corrupt samples not generated. Skipping test.");
        return;
    }

    let reference_file = PathBuf::from("test_data/av1_test.ivf");
    if !reference_file.exists() {
        eprintln!("⚠️  Reference file not found. Skipping test.");
        return;
    }

    let ref_size = fs::metadata(&reference_file).unwrap().len();

    // Non-truncated corrupt files should have same size as original
    let non_truncated = vec![
        "01_light_1error.ivf",
        "02_medium_5errors.ivf",
        "03_heavy_20errors.ivf",
        "04_obu_header_corrupt.ivf",
        "05_frame_header_corrupt.ivf",
    ];

    for filename in non_truncated {
        let file_path = samples_dir.join(filename);
        if file_path.exists() {
            let size = fs::metadata(&file_path).unwrap().len();
            assert_eq!(
                size, ref_size,
                "{} should have same size as original ({} bytes), got {} bytes",
                filename, ref_size, size
            );
        }
    }
}

#[test]
fn test_ivf_header_preservation() {
    let samples_dir = PathBuf::from(TEST_DATA_DIR);

    if !samples_dir.exists() {
        eprintln!("⚠️  Corrupt samples not generated. Skipping test.");
        return;
    }

    // IVF header should be preserved (first 32 bytes)
    // Signature: "DKIF" (0x46, 0x4B, 0x49, 0x44 in little endian)
    let expected_signature = [b'D', b'K', b'I', b'F'];

    let test_files = vec![
        "01_light_1error.ivf",
        "02_medium_5errors.ivf",
        "04_obu_header_corrupt.ivf",
    ];

    for filename in test_files {
        let file_path = samples_dir.join(filename);
        if file_path.exists() {
            let data = fs::read(&file_path).unwrap();

            assert!(
                data.len() >= 4,
                "{}: File too small to contain IVF header",
                filename
            );

            let signature = &data[0..4];
            assert_eq!(
                signature, &expected_signature,
                "{}: IVF header signature mismatch. Expected 'DKIF', got {:?}",
                filename, signature
            );
        }
    }
}

#[test]
fn test_file_readability() {
    let samples_dir = PathBuf::from(TEST_DATA_DIR);

    if !samples_dir.exists() {
        eprintln!("⚠️  Corrupt samples not generated. Skipping test.");
        return;
    }

    // All files should be readable
    for entry in fs::read_dir(&samples_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("ivf") {
            let result = fs::read(&path);
            assert!(
                result.is_ok(),
                "Failed to read file: {:?}",
                path.file_name().unwrap()
            );

            let data = result.unwrap();
            assert!(
                !data.is_empty(),
                "File is empty: {:?}",
                path.file_name().unwrap()
            );
        }
    }
}

#[test]
fn test_reference_file_exists() {
    // Tests run from target/debug/deps, so go up to workspace root
    let mut reference_file = std::env::current_dir().unwrap();

    // Go up from target/debug/deps to workspace root
    while reference_file.file_name() != Some(std::ffi::OsStr::new("bitvue")) {
        if !reference_file.pop() {
            // If we can't find bitvue directory, try relative path from workspace root
            reference_file = PathBuf::from("../../../test_data/av1_test.ivf");
            break;
        }
    }

    if reference_file.file_name() == Some(std::ffi::OsStr::new("bitvue")) {
        reference_file.push("test_data/av1_test.ivf");
    }

    assert!(
        reference_file.exists(),
        "Reference file not found at {:?}. This is required for corrupt sample generation.",
        reference_file
    );

    if reference_file.exists() {
        let metadata = fs::metadata(&reference_file).unwrap();
        assert!(
            metadata.len() > 1000,
            "Reference file is too small ({}). Expected at least 1KB.",
            metadata.len()
        );
    }
}

#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_generate_corrupt_samples() {
    use std::process::Command;

    // This test actually generates the corrupt samples
    let output = Command::new("python3")
        .arg("create_corrupt_from_existing.py")
        .output()
        .expect("Failed to run corruption script");

    assert!(
        output.status.success(),
        "Corruption script failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify files were created
    let samples_dir = PathBuf::from(TEST_DATA_DIR);
    assert!(
        samples_dir.exists(),
        "Corrupt samples directory not created"
    );

    let files: Vec<_> = fs::read_dir(&samples_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("ivf"))
        .collect();

    assert!(
        files.len() >= 7,
        "Expected at least 7 corrupt sample files, found {}",
        files.len()
    );
}
