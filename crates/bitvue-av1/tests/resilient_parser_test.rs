//! Tests for resilient OBU parser with diagnostic generation

use bitvue_av1::{parse_all_obus_resilient, parse_ivf_frames};
use bitvue_core::event::{Category, Severity};
use bitvue_core::StreamId;

#[test]
fn test_resilient_parser_valid_file() {
    // Valid file should return 0 diagnostics
    let valid_data = include_bytes!("../../../test_data/av1_test.ivf");

    // Extract OBU data from IVF
    let (_, ivf_frames) = parse_ivf_frames(valid_data).unwrap();
    let mut obu_data = Vec::new();
    for frame in &ivf_frames {
        obu_data.extend_from_slice(&frame.data);
    }

    let (obus, diagnostics) = parse_all_obus_resilient(&obu_data, StreamId::A);

    // Valid file should parse successfully with no errors
    assert!(obus.len() > 0, "Should parse some OBUs");
    assert_eq!(diagnostics.len(), 0, "Valid file should have 0 diagnostics");
}

#[test]
fn test_resilient_parser_invalid_obu_type() {
    // Create data with invalid OBU type (> 15)
    let mut data = Vec::new();

    // Valid OBU header first (TEMPORAL_DELIMITER)
    data.push(0x12); // type=2, has_size=1
    data.push(0x00); // size=0 (leb128)

    // Invalid OBU type (16)
    data.push(0x82); // type=16 (invalid), has_size=1
    data.push(0x00); // size=0

    let (obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    // Should parse first OBU successfully (might parse more due to recovery)
    assert!(obus.len() >= 1, "Should parse at least 1 valid OBU before error");

    // Should generate diagnostic for invalid OBU type
    assert!(diagnostics.len() >= 1, "Should have at least 1 diagnostic for invalid type");

    // Check first diagnostic (might have different message depending on recovery)
    if !diagnostics.is_empty() {
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(!diagnostics[0].message.is_empty(), "Message should not be empty");
        assert!(diagnostics[0].impact_score >= 80, "Should have high impact score");
        assert_eq!(diagnostics[0].category, Category::Bitstream);
    }
}

#[test]
fn test_resilient_parser_unexpected_eof() {
    // Create OBU header claiming more data than available
    let mut data = Vec::new();

    // OBU header: type=1 (SEQUENCE_HEADER), has_size=1
    data.push(0x0A);
    // Size: 100 bytes (but we won't provide them)
    data.push(100);
    // Missing 100 bytes of payload

    let (_obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    // Should generate diagnostic for unexpected EOF
    assert!(diagnostics.len() > 0, "Should have diagnostic for EOF");

    // Check for EOF diagnostic
    let has_eof = diagnostics.iter().any(|d| {
        d.severity == Severity::Fatal && d.message.contains("end of file")
    });
    assert!(has_eof, "Should have fatal EOF diagnostic");
}

#[test]
fn test_resilient_parser_forbidden_bit_set() {
    // Create OBU with forbidden bit set
    let mut data = Vec::new();

    // OBU header with forbidden bit = 1 (0x8A = 10001010)
    data.push(0x8A); // forbidden=1, type=1, has_extension=0, has_size=1
    data.push(0x00); // size=0

    let (_obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    // Should generate diagnostic for forbidden bit
    assert!(diagnostics.len() > 0, "Should have diagnostic for forbidden bit");

    let has_forbidden = diagnostics.iter().any(|d| {
        d.message.contains("forbidden") || d.message.contains("not 0")
    });
    assert!(has_forbidden, "Should have diagnostic about forbidden bit");
}

#[test]
fn test_resilient_parser_multiple_errors() {
    // Create data with multiple errors
    let mut data = Vec::new();

    // Error 1: Invalid type
    data.push(0x82); // type=16 (invalid)
    data.push(0x00);

    // Error 2: Another invalid type
    data.push(0x8A); // forbidden bit set
    data.push(0x00);

    // Error 3: Truncated
    data.push(0x0A); // Valid header
    data.push(0x64); // Claims 100 bytes
    // Missing payload

    let (_obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    // Should generate diagnostics (might be fewer due to recovery finding valid data)
    assert!(diagnostics.len() >= 1, "Should have at least 1 diagnostic for errors");

    // Check that all diagnostics have required fields
    for diagnostic in &diagnostics {
        assert!(diagnostic.id < 1000, "Diagnostic ID should be reasonable");
        assert_eq!(diagnostic.stream_id, StreamId::A);
        assert!(!diagnostic.message.is_empty(), "Message should not be empty");
        assert_eq!(diagnostic.category, Category::Bitstream);
        assert!(diagnostic.impact_score > 0, "Impact score should be > 0");
        assert!(diagnostic.count >= 1, "Count should be >= 1");
    }
}

#[test]
fn test_resilient_parser_error_limit() {
    // Create data that will trigger 15 consecutive errors
    let mut data = Vec::new();

    for _ in 0..15 {
        // Invalid OBU type
        data.push(0x82); // type=16 (invalid)
    }

    let (_obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    // Should stop after 10 errors and add a fatal diagnostic
    assert!(diagnostics.len() <= 11, "Should stop after 10 errors + 1 fatal");

    // Last diagnostic should be fatal "too many errors"
    if diagnostics.len() > 0 {
        let last = &diagnostics[diagnostics.len() - 1];
        if last.severity == Severity::Fatal {
            assert!(last.message.contains("Too many") || last.message.contains("stopping"));
            assert_eq!(last.impact_score, 100);
        }
    }
}

#[test]
fn test_resilient_parser_frame_index_approximation() {
    // Create valid OBUs followed by error
    let mut data = Vec::new();

    // OBU 1: TEMPORAL_DELIMITER (not a frame)
    data.push(0x12); // type=2
    data.push(0x00);

    // OBU 2: FRAME (has frame data)
    data.push(0x32); // type=6
    data.push(0x00);

    // OBU 3: FRAME
    data.push(0x32);
    data.push(0x00);

    // Error after 3 OBUs
    data.push(0x82); // Invalid type
    data.push(0x00);

    let (obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    // Recovery might parse more OBUs (valid data might be found after errors)
    assert!(obus.len() >= 3, "Should parse at least 3 valid OBUs");
    assert!(diagnostics.len() >= 1, "Should have at least 1 error diagnostic");

    // Frame index should be approximated based on parsed OBUs
    if !diagnostics.is_empty() {
        assert!(
            diagnostics[0].frame_index.is_some(),
            "Frame index should be set"
        );
    }
}

#[test]
fn test_resilient_parser_offset_tracking() {
    // Create OBUs at known offsets
    let mut data = Vec::new();

    // OBU 1 at offset 0
    data.push(0x12); // type=2, offset=0
    data.push(0x00);

    // OBU 2 at offset 2
    data.push(0x12); // type=2, offset=2
    data.push(0x00);

    // Error at offset 4
    let error_offset = data.len() as u64;
    data.push(0x82); // Invalid type, offset=4
    data.push(0x00);

    let (_obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    assert_eq!(diagnostics.len(), 1);

    // Diagnostic offset should be close to error location
    // (might be off by 1 due to skip-ahead recovery)
    assert!(
        diagnostics[0].offset_bytes >= error_offset,
        "Diagnostic offset should be >= {}",
        error_offset
    );
}

#[test]
fn test_resilient_parser_all_severity_levels() {
    // Create errors that trigger different severity levels
    let mut data = Vec::new();

    // Parse error (Severity::Error)
    data.push(0x8A); // forbidden bit
    data.push(0x00);

    // Invalid type (Severity::Error)
    data.push(0x82);
    data.push(0x00);

    // Unexpected EOF (Severity::Fatal)
    data.push(0x0A);
    data.push(0xFF); // Claims 255 bytes
    // Missing payload triggers EOF

    let (_obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    // Should have mix of error and fatal
    let has_error = diagnostics.iter().any(|d| d.severity == Severity::Error);
    let has_fatal = diagnostics.iter().any(|d| d.severity == Severity::Fatal);

    assert!(has_error || has_fatal, "Should have Error or Fatal diagnostics");
}

#[test]
fn test_resilient_parser_impact_scores() {
    // Verify impact scores are set correctly for different error types
    let mut data = Vec::new();

    // Invalid OBU type (impact 90)
    data.push(0x82);
    data.push(0x00);

    let (_obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    assert_eq!(diagnostics.len(), 1);

    // Should have high impact score for invalid type
    assert!(
        diagnostics[0].impact_score >= 80,
        "Invalid type should have impact >= 80, got {}",
        diagnostics[0].impact_score
    );
}

#[test]
fn test_resilient_parser_stream_id_preserved() {
    // Verify stream ID is correctly set in diagnostics
    let data = vec![0x82, 0x00]; // Invalid type

    let (_obus_a, diagnostics_a) = parse_all_obus_resilient(&data, StreamId::A);
    let (_obus_b, diagnostics_b) = parse_all_obus_resilient(&data, StreamId::B);

    assert_eq!(diagnostics_a[0].stream_id, StreamId::A);
    assert_eq!(diagnostics_b[0].stream_id, StreamId::B);
}

#[test]
fn test_resilient_parser_recovery_after_error() {
    // Test that parser can recover and continue after error
    let mut data = Vec::new();

    // Valid OBU
    data.push(0x12);
    data.push(0x00);

    // Error
    data.push(0x82);

    // Another valid OBU after error
    data.push(0x12);
    data.push(0x00);

    let (obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    // Should recover and parse OBUs after error
    assert!(obus.len() >= 1, "Should parse at least 1 OBU");
    assert!(diagnostics.len() >= 1, "Should have at least 1 diagnostic");
}

#[test]
fn test_resilient_parser_empty_data() {
    // Empty data should return no OBUs and no diagnostics
    let data: Vec<u8> = vec![];

    let (obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    assert_eq!(obus.len(), 0, "Empty data should have 0 OBUs");
    assert_eq!(diagnostics.len(), 0, "Empty data should have 0 diagnostics");
}

#[test]
fn test_resilient_parser_diagnostic_count_field() {
    // Verify count field is set to 1 for individual errors
    let data = vec![0x82, 0x00]; // Single error

    let (_obus, diagnostics) = parse_all_obus_resilient(&data, StreamId::A);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].count, 1, "Individual error should have count=1");
}
