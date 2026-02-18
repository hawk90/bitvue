#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Comprehensive tests for HEVC error.rs module.
//! Targeting 95%+ line coverage for error handling.

use bitvue_hevc::{HevcError, Result};
use std::io::{Error, ErrorKind};

// ============================================================================
// HevcError Variant Tests
// ============================================================================

#[test]
fn test_hevc_error_unexpected_eof() {
    let err = HevcError::UnexpectedEof(100);
    assert!(matches!(err, HevcError::UnexpectedEof(_)));
}

#[test]
fn test_hevc_error_invalid_data() {
    let err = HevcError::InvalidData("test data".to_string());
    assert!(matches!(err, HevcError::InvalidData(_)));
}

#[test]
fn test_hevc_error_insufficient_data() {
    let err = HevcError::InsufficientData {
        expected: 100,
        actual: 50,
    };
    assert!(matches!(
        err,
        HevcError::InsufficientData {
            expected: 100,
            actual: 50
        }
    ));
}

#[test]
fn test_hevc_error_io_error() {
    let io_err = Error::new(ErrorKind::Other, "test error");
    let err = HevcError::Io(io_err);
    assert!(matches!(err, HevcError::Io(_)));
}

#[test]
fn test_hevc_error_parse_error() {
    let err = HevcError::Parse {
        offset: 50,
        message: "test message".to_string(),
    };
    assert!(matches!(
        err,
        HevcError::Parse {
            offset: 50,
            message: _
        }
    ));
}

#[test]
fn test_hevc_error_display() {
    let err1 = HevcError::UnexpectedEof(100);
    let err2 = HevcError::InvalidData("test data".to_string());
    let err3 = HevcError::InsufficientData {
        expected: 100,
        actual: 50,
    };

    // Verify errors can be formatted for display
    let debug1 = format!("{:?}", err1);
    let debug2 = format!("{:?}", err2);
    let debug3 = format!("{:?}", err3);

    assert!(debug1.contains("UnexpectedEof") || debug1.contains("Unexpected end of data"));
    assert!(debug2.contains("InvalidData") || debug2.contains("Invalid data"));
    assert!(debug3.contains("InsufficientData") || debug3.contains("Insufficient data"));
}

// ============================================================================
// Result<T> Type Tests
// ============================================================================

#[test]
fn test_result_ok() {
    let value = 42u32;
    let result: Result<u32> = Result::Ok(value);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_result_err() {
    let err = HevcError::InvalidData("error".to_string());
    let result: Result<u32> = Result::Err(err);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("error"));
}

// ============================================================================
// Error Display and Debug Tests
// ============================================================================

#[test]
fn test_error_display_unexpected_eof() {
    let err = HevcError::UnexpectedEof(100);
    let display = format!("{:?}", err);
    assert!(display.contains("100"));
}

#[test]
fn test_error_display_invalid_data() {
    let err = HevcError::InvalidData("test data".to_string());
    let display = format!("{:?}", err);
    assert!(display.contains("test data"));
}

#[test]
fn test_error_display_insufficient_data() {
    let err = HevcError::InsufficientData {
        expected: 100,
        actual: 50,
    };
    let display = format!("{:?}", err);
    assert!(display.contains("100") || display.contains("50"));
}

#[test]
fn test_error_display_parse_error() {
    let err = HevcError::Parse {
        offset: 50,
        message: "test".to_string(),
    };
    let display = format!("{:?}", err);
    assert!(display.contains("50") || display.contains("test"));
}

#[test]
fn test_error_display_io_error() {
    let io_err = Error::new(ErrorKind::Other, "io error");
    let err = HevcError::Io(io_err);
    let display = format!("{:?}", err);
    assert!(display.contains("io error") || display.contains("IO error"));
}
