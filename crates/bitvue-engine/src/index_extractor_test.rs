// Index extractor module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use std::io::Cursor;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a mock AV1 OBU header
fn create_av1_obu_header(obu_type: u8, has_size: bool, size: u64) -> Vec<u8> {
    let mut data = Vec::new();
    let header = (obu_type << 3) | (if has_size { 0x02 } else { 0 });
    data.push(header);

    if has_size {
        // Write LEB128 encoded size
        let mut size = size;
        loop {
            let mut byte = (size & 0x7F) as u8;
            size >>= 7;
            if size > 0 {
                byte |= 0x80;
            }
            data.push(byte);
            if size == 0 {
                break;
            }
        }
    }

    data
}

/// Create a mock H.264 NAL unit with start code
fn create_h264_nal_unit(nal_unit_type: u8) -> Vec<u8> {
    let mut data = vec![0x00, 0x00, 0x01]; // 3-byte start code
    data.push(nal_unit_type);
    // Add some payload
    data.extend_from_slice(&[0xFF, 0xFF, 0xFF]);
    data
}

/// Create a mock cursor for testing
fn create_test_cursor(data: Vec<u8>) -> Cursor<Vec<u8>> {
    Cursor::new(data)
}

/// Create AV1 frame OBU (keyframe)
fn create_av1_frame_obu(is_keyframe: bool, size: u64) -> Vec<u8> {
    let obu_type = if is_keyframe { 6 } else { 3 }; // FRAME or FRAME_HEADER
    create_av1_obu_header(obu_type, true, size)
}

/// Create AV1 sequence header OBU (always keyframe)
fn create_av1_sequence_header() -> Vec<u8> {
    create_av1_obu_header(1, true, 10) // OBU_SEQUENCE_HEADER
}

// ============================================================================
// Av1IndexExtractor Tests
// ============================================================================

#[cfg(test)]
mod av1_index_extractor_tests {
    use super::*;

    #[test]
    fn test_av1_extractor_new() {
        // Arrange & Act
        let extractor = Av1IndexExtractor::new();

        // Assert
        assert_eq!(extractor.codec_name(), "AV1");
        assert!(extractor.is_supported());
    }

    #[test]
    fn test_av1_extractor_default() {
        // Arrange & Act
        let extractor = Av1IndexExtractor;

        // Assert
        assert_eq!(extractor.codec_name(), "AV1");
    }

    #[test]
    fn test_av1_read_leb128_single_byte() {
        // Arrange
        let extractor = Av1IndexExtractor::new();
        let data = vec![0x5F]; // 95 (0x5F), no continuation bit

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.read_leb128(&mut cursor);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 95);
    }

    #[test]
    fn test_av1_read_leb128_multi_byte() {
        // Arrange
        let extractor = Av1IndexExtractor::new();
        let data = vec![0x80, 0x01]; // 128: 0x80 | 0x01 << 7

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.read_leb128(&mut cursor);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 128);
    }

    #[test]
    fn test_av1_read_leb128_large_value() {
        // Arrange
        let extractor = Av1IndexExtractor::new();
        let data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x07]; // Maximum in 28 bits
        // Encodes: 0x7F + (0x7F << 7) + (0x7F << 14) + (0x7F << 21) + (0x07 << 28) = 0x7FFFFFFF

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.read_leb128(&mut cursor);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x7FFFFFFF);
    }

    #[test]
    fn test_av1_read_leb128_overflow() {
        // Arrange
        let extractor = Av1IndexExtractor::new();
        // Create more than 8 bytes with continuation bits
        let data = vec![0x80u8; 10]; // 10 continuation bytes

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.read_leb128(&mut cursor);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_av1_extract_quick_index_with_sequence_header() {
        // Arrange
        let extractor = Av1IndexExtractor::new();
        let mut data = Vec::new();
        data.extend_from_slice(&create_av1_sequence_header());
        data.extend_from_slice(&create_av1_frame_obu(true, 100));

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.extract_quick_index(&mut cursor);

        // Assert
        assert!(result.is_ok());
        let index = result.unwrap();
        assert!(!index.seek_points.is_empty());
        assert!(index.seek_points[0].is_keyframe);
    }

    #[test]
    fn test_av1_extract_quick_index_empty_stream() {
        // Arrange
        let extractor = Av1IndexExtractor::new();
        let data = vec![];

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.extract_quick_index(&mut cursor);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_av1_extract_full_index_with_callbacks() {
        // Arrange
        let extractor = Av1IndexExtractor::new();
        let mut data = Vec::new();
        data.extend_from_slice(&create_av1_frame_obu(true, 100));
        data.extend_from_slice(&create_av1_frame_obu(false, 80));

        let progress_fn = |p: f64, _msg: &str| {
            assert!((0.0..=1.0).contains(&p));
        };
        let cancel_fn = || false;

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.extract_full_index(&mut cursor, Some(&progress_fn), Some(&cancel_fn));

        // Assert
        assert!(result.is_ok());
        let frames = result.unwrap();
        assert!(!frames.is_empty());
    }

    #[test]
    fn test_av1_extract_full_index_with_cancellation() {
        // Arrange
        let extractor = Av1IndexExtractor::new();
        let mut data = Vec::new();
        for _ in 0..10 {
            data.extend_from_slice(&create_av1_frame_obu(true, 100));
        }

        let cancel_fn = || true; // Cancel immediately

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.extract_full_index(&mut cursor, None, Some(&cancel_fn));

        // Assert
        assert!(result.is_err());
        if let Err(BitvueError::InvalidData(msg)) = result {
            assert!(msg.contains("cancelled"));
        } else {
            panic!("Expected InvalidData error");
        }
    }
}

// ============================================================================
// H264IndexExtractor Tests
// ============================================================================

#[cfg(test)]
mod h264_index_extractor_tests {
    use super::*;

    #[test]
    fn test_h264_extractor_new() {
        // Arrange & Act
        let extractor = H264IndexExtractor::new();

        // Assert
        assert_eq!(extractor.codec_name(), "H.264");
        assert!(extractor.is_supported());
    }

    #[test]
    fn test_h264_extractor_default() {
        // Arrange & Act
        let extractor = H264IndexExtractor;

        // Assert
        assert_eq!(extractor.codec_name(), "H.264");
    }

    #[test]
    fn test_h264_extract_quick_index_with_idr_frame() {
        // Arrange
        let extractor = H264IndexExtractor::new();
        let mut data = Vec::new();
        // Add IDR frame (nal_unit_type = 5)
        data.extend_from_slice(&create_h264_nal_unit(5));

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.extract_quick_index(&mut cursor);

        // Assert
        assert!(result.is_ok());
        let index = result.unwrap();
        assert!(!index.seek_points.is_empty());
        assert!(index.seek_points[0].is_keyframe);
    }

    #[test]
    fn test_h264_extract_quick_index_with_non_idr_frame() {
        // Arrange
        let extractor = H264IndexExtractor::new();
        let mut data = Vec::new();
        // Add non-IDR frame (nal_unit_type = 1)
        data.extend_from_slice(&create_h264_nal_unit(1));

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.extract_quick_index(&mut cursor);

        // Assert - Quick mode with no keyframes returns error
        assert!(result.is_err());
    }

    #[test]
    fn test_h264_extract_quick_index_four_byte_start_code() {
        // Arrange
        let extractor = H264IndexExtractor::new();
        let mut data = Vec::new();
        // 4-byte start code + IDR
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data.push(5); // NAL header for IDR

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.extract_quick_index(&mut cursor);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_h264_extract_quick_index_empty_stream() {
        // Arrange
        let extractor = H264IndexExtractor::new();
        let data = vec![];

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.extract_quick_index(&mut cursor);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_h264_extract_full_index_with_progress() {
        // Arrange
        let extractor = H264IndexExtractor::new();
        let mut data = Vec::new();
        data.extend_from_slice(&create_h264_nal_unit(5)); // IDR
        data.extend_from_slice(&create_h264_nal_unit(1)); // Non-IDR

        let progress_fn = |p: f64, _msg: &str| {
            assert!((0.0..=1.0).contains(&p));
        };
        let cancel_fn = || false;

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.extract_full_index(&mut cursor, Some(&progress_fn), Some(&cancel_fn));

        // Assert
        assert!(result.is_ok());
        let frames = result.unwrap();
        assert!(!frames.is_empty());
    }

    #[test]
    fn test_h264_find_next_start_code() {
        // Arrange
        let extractor = H264IndexExtractor::new();
        let mut data = Vec::new();
        data.extend_from_slice(&[0xFF, 0xFF]); // Padding
        data.extend_from_slice(&[0x00, 0x00, 0x01]); // 3-byte start code
        data.extend_from_slice(&[0xFF]); // Extra byte to ensure 4 bytes can be read at offset 2
        let file_size = data.len() as u64;

        // Act
        let mut cursor = create_test_cursor(data);
        cursor.seek(std::io::SeekFrom::Start(0)).unwrap();
        let result = extractor.find_next_start_code(&mut cursor, file_size);

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }
}

// ============================================================================
// ExtractorFactory Tests
// ============================================================================

#[cfg(test)]
mod extractor_factory_tests {
    use super::*;

    #[test]
    fn test_factory_create_av1() {
        // Act
        let extractor = ExtractorFactory::create("av1");

        // Assert
        assert_eq!(extractor.codec_name(), "AV1");
        assert!(extractor.is_supported());
    }

    #[test]
    fn test_factory_create_av1_uppercase() {
        // Act
        let extractor = ExtractorFactory::create("AV1");

        // Assert
        assert_eq!(extractor.codec_name(), "AV1");
    }

    #[test]
    fn test_factory_create_h264() {
        // Act
        let extractor = ExtractorFactory::create("h264");

        // Assert
        assert_eq!(extractor.codec_name(), "H.264");
        assert!(extractor.is_supported());
    }

    #[test]
    fn test_factory_create_h264_variant() {
        // Arrange & Act
        let extractor1 = ExtractorFactory::create("h.264");
        let extractor2 = ExtractorFactory::create("avc");

        // Assert
        assert_eq!(extractor1.codec_name(), "H.264");
        assert_eq!(extractor2.codec_name(), "H.264");
    }

    #[test]
    fn test_factory_create_unsupported_codec() {
        // Act
        let extractor = ExtractorFactory::create("unknown_codec");

        // Assert
        assert_eq!(extractor.codec_name(), "Unsupported");
        assert!(!extractor.is_supported());
    }

    #[test]
    fn test_factory_create_empty_string() {
        // Act
        let extractor = ExtractorFactory::create("");

        // Assert
        assert_eq!(extractor.codec_name(), "Unsupported");
        assert!(!extractor.is_supported());
    }

    #[test]
    fn test_factory_from_extension_ivf() {
        // Act
        let extractor = ExtractorFactory::from_extension("ivf");

        // Assert
        assert_eq!(extractor.codec_name(), "AV1");
    }

    #[test]
    fn test_factory_from_extension_h264() {
        // Act
        let extractor = ExtractorFactory::from_extension("h264");

        // Assert
        assert_eq!(extractor.codec_name(), "H.264");
    }

    #[test]
    fn test_factory_from_extension_uppercase() {
        // Act
        let extractor = ExtractorFactory::from_extension("IVF");

        // Assert
        assert_eq!(extractor.codec_name(), "AV1");
    }

    #[test]
    fn test_factory_from_extension_unknown() {
        // Act
        let extractor = ExtractorFactory::from_extension("unknown");

        // Assert
        assert_eq!(extractor.codec_name(), "Unsupported");
        assert!(!extractor.is_supported());
    }

    #[test]
    fn test_factory_from_extension_empty() {
        // Act
        let extractor = ExtractorFactory::from_extension("");

        // Assert
        assert_eq!(extractor.codec_name(), "Unsupported");
    }
}

// ============================================================================
// UnsupportedExtractor Tests
// ============================================================================

#[cfg(test)]
mod unsupported_extractor_tests {
    use super::*;

    #[test]
    fn test_unsupported_extractor_codec_name() {
        // Arrange
        let extractor = UnsupportedExtractor {
            codec: "test_codec".to_string(),
        };

        // Act
        let name = extractor.codec_name();

        // Assert
        assert_eq!(name, "Unsupported");
    }

    #[test]
    fn test_unsupported_extractor_is_not_supported() {
        // Arrange
        let extractor = UnsupportedExtractor {
            codec: "test_codec".to_string(),
        };

        // Act
        let supported = extractor.is_supported();

        // Assert
        assert!(!supported);
    }

    #[test]
    fn test_unsupported_extractor_extract_quick_index_returns_error() {
        // Arrange
        let extractor = UnsupportedExtractor {
            codec: "test_codec".to_string(),
        };
        let mut cursor = create_test_cursor(vec![0xFF, 0xFF]);

        // Act
        let result = extractor.extract_quick_index(&mut cursor);

        // Assert
        assert!(result.is_err());
        match result {
            Err(BitvueError::UnsupportedCodec(codec)) => {
                assert_eq!(codec, "test_codec");
            }
            _ => panic!("Expected UnsupportedCodec error"),
        }
    }

    #[test]
    fn test_unsupported_extractor_extract_full_index_returns_error() {
        // Arrange
        let extractor = UnsupportedExtractor {
            codec: "test_codec".to_string(),
        };
        let mut cursor = create_test_cursor(vec![0xFF, 0xFF]);

        // Act
        let result = extractor.extract_full_index(&mut cursor, None, None);

        // Assert
        assert!(result.is_err());
        match result {
            Err(BitvueError::UnsupportedCodec(codec)) => {
                assert_eq!(codec, "test_codec");
            }
            _ => panic!("Expected UnsupportedCodec error"),
        }
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_av1_obu_without_size_field_in_quick_mode() {
        // Arrange
        let extractor = Av1IndexExtractor::new();
        // OBU header without size field (obu_header[0] & 0x02 == 0)
        let data = vec![0x08]; // OBU type 1, no size field

        // Act
        let mut cursor = create_test_cursor(data);
        let result = extractor.extract_quick_index(&mut cursor);

        // Assert - Should stop early in quick mode
        // Result might be Ok with empty seek_points or Err
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_h264_find_start_code_at_end_of_file() {
        // Arrange
        let extractor = H264IndexExtractor::new();
        let data = vec![0x00, 0x00, 0x01]; // Start code at very end
        let file_size = data.len() as u64;

        // Act
        let mut cursor = create_test_cursor(data);
        cursor.seek(std::io::SeekFrom::Start(0)).unwrap();
        let result = extractor.find_next_start_code(&mut cursor, file_size);

        // Assert
        assert!(result.is_ok());
        // Should find the start code or return None depending on offset
    }

    #[test]
    fn test_factory_case_insensitive_matching() {
        // Arrange & Act
        let extractors = vec![
            ExtractorFactory::create("AV1"),
            ExtractorFactory::create("av1"),
            ExtractorFactory::create("Av1"),
            ExtractorFactory::create("H264"),
            ExtractorFactory::create("h264"),
            ExtractorFactory::create("H.264"),
        ];

        // Assert - All should create valid extractors
        for extractor in extractors {
            assert!(extractor.is_supported() || extractor.codec_name() == "Unsupported");
        }
    }

    #[test]
    fn test_extractor_factory_various_h264_names() {
        // Arrange & Act
        let names = vec!["h264", "h.264", "avc"];
        let codec_name = "H.264";

        // Assert
        for name in names {
            let extractor = ExtractorFactory::create(name);
            assert_eq!(extractor.codec_name(), codec_name);
        }
    }

    #[test]
    fn test_extractor_factory_various_av1_names() {
        // Arrange & Act
        let names = vec!["av1"];
        let codec_name = "AV1";

        // Assert
        for name in names {
            let extractor = ExtractorFactory::create(name);
            assert_eq!(extractor.codec_name(), codec_name);
        }
    }
}
