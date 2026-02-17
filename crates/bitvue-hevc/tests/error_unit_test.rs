#![allow(dead_code)]
//! HEVC Error Unit Tests
//!
//! Tests for error.rs module to improve coverage.

use bitvue_hevc::HevcError;
use std::error::Error;

// ============================================================================
// HevcError Variant Creation Tests
// ============================================================================

#[test]
fn test_hevc_error_unexpected_eof() {
    let err = HevcError::UnexpectedEof(100);
    assert!(matches!(err, HevcError::UnexpectedEof(100)));
}

#[test]
fn test_hevc_error_invalid_data() {
    let err = HevcError::InvalidData("test error".to_string());
    assert!(matches!(err, HevcError::InvalidData(_)));
    if let HevcError::InvalidData(msg) = err {
        assert_eq!(msg, "test error");
    }
}

#[test]
fn test_hevc_error_insufficient_data() {
    let err = HevcError::InsufficientData {
        expected: 50,
        actual: 25,
    };
    assert!(matches!(err, HevcError::InsufficientData { .. }));
}

#[test]
fn test_hevc_error_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = HevcError::Io(io_err);
    assert!(matches!(err, HevcError::Io(_)));
}

#[test]
fn test_hevc_error_parse() {
    let err = HevcError::Parse {
        offset: 42,
        message: "parse error".to_string(),
    };
    assert!(matches!(err, HevcError::Parse { .. }));
    if let HevcError::Parse { offset, message } = err {
        assert_eq!(offset, 42);
        assert_eq!(message, "parse error");
    }
}

// ============================================================================
// Display Tests
// ============================================================================

#[test]
fn test_hevc_error_display_unexpected_eof() {
    let err = HevcError::UnexpectedEof(42);
    let display = format!("{}", err);
    assert!(display.contains("Unexpected") || display.contains("end of data"));
}

#[test]
fn test_hevc_error_display_invalid_data() {
    let err = HevcError::InvalidData("test data error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("test data error"));
}

#[test]
fn test_hevc_error_display_insufficient_data() {
    let err = HevcError::InsufficientData {
        expected: 10,
        actual: 5,
    };
    let display = format!("{}", err);
    assert!(display.contains("Insufficient") || display.contains("data"));
    assert!(display.contains("10") || display.contains("expected"));
}

#[test]
fn test_hevc_error_display_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let err = HevcError::Io(io_err);
    let display = format!("{}", err);
    assert!(display.contains("access denied") || display.contains("IO error"));
}

#[test]
fn test_hevc_error_display_parse() {
    let err = HevcError::Parse {
        offset: 100,
        message: "syntax error".to_string(),
    };
    let display = format!("{}", err);
    assert!(display.contains("syntax error") || display.contains("offset"));
}

// ============================================================================
// Debug Tests
// ============================================================================

#[test]
fn test_hevc_error_debug_unexpected_eof() {
    let err = HevcError::UnexpectedEof(100);
    let debug = format!("{:?}", err);
    assert!(debug.contains("UnexpectedEof"));
}

#[test]
fn test_hevc_error_debug_invalid_data() {
    let err = HevcError::InvalidData("debug test".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("InvalidData"));
}

#[test]
fn test_hevc_error_debug_insufficient_data() {
    let err = HevcError::InsufficientData {
        expected: 25,
        actual: 10,
    };
    let debug = format!("{:?}", err);
    assert!(debug.contains("InsufficientData"));
}

#[test]
fn test_hevc_error_debug_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken pipe");
    let err = HevcError::Io(io_err);
    let debug = format!("{:?}", err);
    assert!(debug.contains("Io"));
}

#[test]
fn test_hevc_error_debug_parse() {
    let err = HevcError::Parse {
        offset: 50,
        message: "debug parse".to_string(),
    };
    let debug = format!("{:?}", err);
    assert!(debug.contains("Parse"));
}

// ============================================================================
// Error Trait Methods
// ============================================================================

#[test]
fn test_hevc_error_source_none() {
    let err = HevcError::UnexpectedEof(10);
    assert!(err.source().is_none());
}

#[test]
fn test_hevc_error_source_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "io error");
    let err = HevcError::Io(io_err);
    assert!(err.source().is_some());
}

#[test]
fn test_hevc_error_source_invalid_data() {
    let err = HevcError::InvalidData("test".to_string());
    assert!(err.source().is_none());
}

#[test]
fn test_hevc_error_source_insufficient_data() {
    let err = HevcError::InsufficientData {
        expected: 10,
        actual: 5,
    };
    assert!(err.source().is_none());
}

#[test]
fn test_hevc_error_source_parse() {
    let err = HevcError::Parse {
        offset: 0,
        message: "test".to_string(),
    };
    assert!(err.source().is_none());
}
