// Error module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use std::io;
use std::path::PathBuf;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test path
fn create_test_path() -> PathBuf {
    PathBuf::from("/tmp/test.ivf")
}

/// Create a test IO error
fn create_test_io_error() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "File not found")
}

// ============================================================================
// BitvueError::Io Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_io_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_io_from_std_io() {
        // Arrange
        let io_err = create_test_io_error();

        // Act
        let bitvue_err: BitvueError = io_err.into();

        // Assert
        assert!(matches!(bitvue_err, BitvueError::Io(_)));
        let display = format!("{}", bitvue_err);
        assert!(display.contains("IO error"));
    }

    #[test]
    fn test_bitvue_error_io_display() {
        // Arrange
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        let bitvue_err: BitvueError = io_err.into();

        // Act
        let display = format!("{}", bitvue_err);

        // Assert
        assert!(display.contains("IO error"));
    }
}

// ============================================================================
// BitvueError::IoError Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_io_error_struct_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_io_error_construct() {
        // Arrange
        let path = create_test_path();
        let source = create_test_io_error();

        // Act
        let err = BitvueError::IoError {
            path: path.clone(),
            source,
        };

        // Assert
        assert!(matches!(err, BitvueError::IoError { .. }));
    }

    #[test]
    fn test_bitvue_error_io_error_display() {
        // Arrange
        let path = PathBuf::from("/tmp/test.ivf");
        let source = io::Error::new(io::ErrorKind::NotFound, "Not found");
        let err = BitvueError::IoError { path, source };

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("IO error at"));
        assert!(display.contains("/tmp/test.ivf"));
    }
}

// ============================================================================
// BitvueError::Parse Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_parse_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_parse_construct() {
        // Arrange
        let offset = 1000u64;
        let message = "Invalid OBU header".to_string();

        // Act
        let err = BitvueError::Parse {
            offset,
            message: message.clone(),
        };

        // Assert
        assert!(matches!(err, BitvueError::Parse { .. }));
    }

    #[test]
    fn test_bitvue_error_parse_display() {
        // Arrange
        let err = BitvueError::Parse {
            offset: 5000,
            message: "Invalid syntax".to_string(),
        };

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("Parse error"));
        assert!(display.contains("5000"));
        assert!(display.contains("Invalid syntax"));
    }
}

// ============================================================================
// BitvueError::InvalidObuType Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_invalid_obu_type_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_invalid_obu_type() {
        // Arrange & Act
        let err = BitvueError::InvalidObuType(0xFF);

        // Assert
        assert!(matches!(err, BitvueError::InvalidObuType(_)));
    }

    #[test]
    fn test_bitvue_error_invalid_obu_type_display() {
        // Arrange
        let err = BitvueError::InvalidObuType(0x42);

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("Invalid OBU type"));
        assert!(display.contains("0x42") || display.contains("66"));
    }
}

// ============================================================================
// BitvueError::UnexpectedEof Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_unexpected_eof_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_unexpected_eof() {
        // Arrange & Act
        let err = BitvueError::UnexpectedEof(10000);

        // Assert
        assert!(matches!(err, BitvueError::UnexpectedEof(_)));
    }

    #[test]
    fn test_bitvue_error_unexpected_eof_display() {
        // Arrange
        let err = BitvueError::UnexpectedEof(9999);

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("Unexpected end of data"));
        assert!(display.contains("9999"));
    }
}

// ============================================================================
// BitvueError::UnsupportedCodec Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_unsupported_codec_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_unsupported_codec() {
        // Arrange & Act
        let err = BitvueError::UnsupportedCodec("H.266".to_string());

        // Assert
        assert!(matches!(err, BitvueError::UnsupportedCodec(_)));
    }

    #[test]
    fn test_bitvue_error_unsupported_codec_display() {
        // Arrange
        let err = BitvueError::UnsupportedCodec("AV3".to_string());

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("Unsupported codec"));
        assert!(display.contains("AV3"));
    }
}

// ============================================================================
// BitvueError::Decode Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_decode_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_decode() {
        // Arrange & Act
        let err = BitvueError::Decode("Corrupted bitstream".to_string());

        // Assert
        assert!(matches!(err, BitvueError::Decode(_)));
    }

    #[test]
    fn test_bitvue_error_decode_display() {
        // Arrange
        let err = BitvueError::Decode("Syntax error".to_string());

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("Decode error"));
        assert!(display.contains("Syntax error"));
    }
}

// ============================================================================
// BitvueError::InsufficientData Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_insufficient_data_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_insufficient_data() {
        // Arrange
        let needed = 100usize;
        let available = 50usize;

        // Act
        let err = BitvueError::InsufficientData {
            needed,
            available,
        };

        // Assert
        assert!(matches!(err, BitvueError::InsufficientData { .. }));
    }

    #[test]
    fn test_bitvue_error_insufficient_data_display() {
        // Arrange
        let err = BitvueError::InsufficientData {
            needed: 1000,
            available: 500,
        };

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("Insufficient data"));
        assert!(display.contains("1000"));
        assert!(display.contains("500"));
    }
}

// ============================================================================
// BitvueError::InvalidData Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_invalid_data_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_invalid_data() {
        // Arrange & Act
        let err = BitvueError::InvalidData("Corrupted header".to_string());

        // Assert
        assert!(matches!(err, BitvueError::InvalidData(_)));
    }

    #[test]
    fn test_bitvue_error_invalid_data_display() {
        // Arrange
        let err = BitvueError::InvalidData("Bad checksum".to_string());

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("Invalid data"));
        assert!(display.contains("Bad checksum"));
    }
}

// ============================================================================
// BitvueError::InvalidFile Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_invalid_file_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_invalid_file() {
        // Arrange & Act
        let err = BitvueError::InvalidFile("Not a valid IVF file".to_string());

        // Assert
        assert!(matches!(err, BitvueError::InvalidFile(_)));
    }

    #[test]
    fn test_bitvue_error_invalid_file_display() {
        // Arrange
        let err = BitvueError::InvalidFile("Unknown format".to_string());

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("Invalid file"));
        assert!(display.contains("Unknown format"));
    }
}

// ============================================================================
// BitvueError::InvalidRange Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_invalid_range_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_invalid_range() {
        // Arrange
        let offset = 1000u64;
        let length = 100usize;

        // Act
        let err = BitvueError::InvalidRange { offset, length };

        // Assert
        assert!(matches!(err, BitvueError::InvalidRange { .. }));
    }

    #[test]
    fn test_bitvue_error_invalid_range_display() {
        // Arrange
        let err = BitvueError::InvalidRange {
            offset: 5000,
            length: 1000,
        };

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("Invalid range"));
        assert!(display.contains("5000"));
        assert!(display.contains("1000"));
    }
}

// ============================================================================
// BitvueError::FileModified Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_file_modified_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_file_modified() {
        // Arrange
        let path = create_test_path();
        let old_size = 1000u64;
        let new_size = 2000u64;

        // Act
        let err = BitvueError::FileModified {
            path,
            old_size,
            new_size,
        };

        // Assert
        assert!(matches!(err, BitvueError::FileModified { .. }));
    }

    #[test]
    fn test_bitvue_error_file_modified_display() {
        // Arrange
        let path = PathBuf::from("/tmp/video.ivf");
        let err = BitvueError::FileModified {
            path: path.clone(),
            old_size: 1024,
            new_size: 2048,
        };

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("File modified"));
        assert!(display.contains("/tmp/video.ivf"));
        assert!(display.contains("1024"));
        assert!(display.contains("2048"));
    }
}

// ============================================================================
// BitvueError::FrameNotFound Tests
// ============================================================================

#[cfg(test)]
mod bitvue_error_frame_not_found_tests {
    use super::*;

    #[test]
    fn test_bitvue_error_frame_not_found() {
        // Arrange & Act
        let err = BitvueError::FrameNotFound(999);

        // Assert
        assert!(matches!(err, BitvueError::FrameNotFound(_)));
    }

    #[test]
    fn test_bitvue_error_frame_not_found_display() {
        // Arrange
        let err = BitvueError::FrameNotFound(50);

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("Frame not found"));
        assert!(display.contains("50"));
    }
}

// ============================================================================
// Result Type Tests
// ============================================================================

#[cfg(test)]
mod result_type_tests {
    use super::*;

    #[test]
    fn test_result_ok_value() {
        // Arrange & Act
        let result: Result<u32> = Ok(42);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_error_value() {
        // Arrange & Act
        let result: Result<u32> = Err(BitvueError::InvalidData("Test".to_string()));

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BitvueError::InvalidData(_)));
    }

    #[test]
    fn test_result_with_io_error() {
        // Arrange
        let io_err = io::Error::new(io::ErrorKind::NotFound, "Not found");

        // Act
        let result: Result<()> = Err(BitvueError::Io(io_err));

        // Assert
        assert!(result.is_err());
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_zero_offset() {
        // Arrange & Act
        let err = BitvueError::Parse {
            offset: 0,
            message: "Test".to_string(),
        };

        // Assert
        let display = format!("{}", err);
        assert!(display.contains("0"));
    }

    #[test]
    fn test_zero_size_file_modified() {
        // Arrange
        let path = PathBuf::from("/tmp/empty.ivf");
        let err = BitvueError::FileModified {
            path,
            old_size: 0,
            new_size: 100,
        };

        // Act
        let display = format!("{}", err);

        // Assert
        assert!(display.contains("0"));
        assert!(display.contains("100"));
    }

    #[test]
    fn test_empty_message() {
        // Arrange & Act
        let err = BitvueError::Decode("".to_string());

        // Assert
        let display = format!("{}", err);
        assert!(display.contains("Decode error"));
    }

    #[test]
    fn test_unicode_message() {
        // Arrange
        let korean_message = "파싱 오류".to_string();

        // Act
        let err = BitvueError::Decode(korean_message.clone());

        // Assert
        let display = format!("{}", err);
        assert!(display.contains("파싱 오류"));
    }

    #[test]
    fn test_large_offset() {
        // Arrange & Act
        let err = BitvueError::UnexpectedEof(u64::MAX);

        // Assert
        let display = format!("{}", err);
        assert!(display.contains("18446744073709551615")); // u64::MAX
    }
}
