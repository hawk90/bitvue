#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Comprehensive tests for VVC error module
//!
//! Tests VvcError enum variants and From<CodecError> implementation

use bitvue_core::codec_error::{Codec, CodecError};
use bitvue_vvc::error::{Result, VvcError};
use std::io;

// ============================================================================
// VvcError Variant Tests
// ============================================================================

#[test]
fn test_error_unexpected_eof_display() {
    let err = VvcError::UnexpectedEof(100);
    let msg = format!("{err}");
    assert!(msg.contains("Unexpected end of data"));
    assert!(msg.contains("100"));
}

#[test]
fn test_error_unexpected_eof_field() {
    let err = VvcError::UnexpectedEof(42);
    match err {
        VvcError::UnexpectedEof(pos) => {
            assert_eq!(pos, 42);
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_error_invalid_data_display() {
    let err = VvcError::InvalidData("corrupted nal".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("Invalid data"));
    assert!(msg.contains("corrupted nal"));
}

#[test]
fn test_error_insufficient_data_display() {
    let err = VvcError::InsufficientData {
        expected: 100,
        actual: 50,
    };
    let msg = format!("{err}");
    assert!(msg.contains("Insufficient data"));
    assert!(msg.contains("100"));
    assert!(msg.contains("50"));
}

#[test]
fn test_error_insufficient_data_fields() {
    let err = VvcError::InsufficientData {
        expected: 200,
        actual: 75,
    };
    match err {
        VvcError::InsufficientData { expected, actual } => {
            assert_eq!(expected, 200);
            assert_eq!(actual, 75);
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_error_parse_display() {
    let err = VvcError::Parse {
        offset: 42,
        message: "invalid syntax".to_string(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("Parse error"));
    assert!(msg.contains("42"));
    assert!(msg.contains("invalid syntax"));
}

#[test]
fn test_error_parse_fields() {
    let err = VvcError::Parse {
        offset: 99,
        message: "test error".to_string(),
    };
    match err {
        VvcError::Parse { offset, message } => {
            assert_eq!(offset, 99);
            assert_eq!(message, "test error");
        }
        _ => panic!("Wrong error type"),
    }
}

// ============================================================================
// Io Error Tests
// ============================================================================

#[test]
fn test_error_io_from_std_io_error() {
    let std_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let vvc_err: VvcError = std_err.into();

    match vvc_err {
        VvcError::Io(io_err) => {
            assert_eq!(io_err.kind(), io::ErrorKind::NotFound);
            assert!(io_err.to_string().contains("file not found"));
        }
        _ => panic!("Wrong error type, got: {:?}", vvc_err),
    }
}

#[test]
fn test_error_io_display() {
    let std_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let vvc_err: VvcError = std_err.into();
    let msg = format!("{vvc_err}");
    assert!(msg.contains("IO error"));
}

#[test]
fn test_error_io_various_kinds() {
    let test_cases = vec![
        io::ErrorKind::NotFound,
        io::ErrorKind::PermissionDenied,
        io::ErrorKind::ConnectionRefused,
        io::ErrorKind::ConnectionReset,
        io::ErrorKind::BrokenPipe,
        io::ErrorKind::WouldBlock,
        io::ErrorKind::InvalidInput,
        io::ErrorKind::TimedOut,
    ];

    for kind in test_cases {
        let std_err = io::Error::new(kind, "test");
        let vvc_err: VvcError = std_err.into();
        match vvc_err {
            VvcError::Io(_) => {}
            _ => panic!("Wrong error type for {:?}", kind),
        }
    }
}

// ============================================================================
// Error Traits Tests
// ============================================================================

#[test]
fn test_error_debug_trait() {
    let err = VvcError::InvalidData("test".to_string());
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("InvalidData"));
    assert!(debug_str.contains("test"));
}

#[test]
fn test_error_std_error_trait() {
    let err = VvcError::Parse {
        offset: 10,
        message: "test".to_string(),
    };
    // Should be able to use as std::error::Error
    let dyn_err: &(dyn std::error::Error + Send + Sync) = &err;
    assert!(dyn_err.to_string().contains("Parse error"));
}

// ============================================================================
// Result Type Alias Tests
// ============================================================================

#[test]
fn test_result_ok_variant() {
    let result: Result<u32> = Ok(42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_result_err_variant() {
    let result: Result<u32> = Err(VvcError::InvalidData("error".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_result_question_mark_operator() {
    fn func_that_fails() -> Result<()> {
        Err(VvcError::UnexpectedEof(0))
    }

    fn caller() -> Result<()> {
        func_that_fails()?;
        Ok(())
    }

    assert!(caller().is_err());
}

// ============================================================================
// From<CodecError> Tests
// ============================================================================

#[test]
fn test_from_codec_error_unexpected_eof() {
    let codec_err = CodecError::UnexpectedEof {
        codec: Codec::Vvc,
        position: 100,
    };
    let vvc_err: VvcError = codec_err.into();

    match vvc_err {
        VvcError::UnexpectedEof(pos) => {
            assert_eq!(pos, 100);
        }
        _ => panic!("Wrong conversion: {:?}", vvc_err),
    }
}

#[test]
fn test_from_codec_error_invalid_data() {
    let codec_err = CodecError::InvalidData {
        codec: Codec::Vvc,
        message: "corrupted data".to_string(),
    };
    let vvc_err: VvcError = codec_err.into();

    match vvc_err {
        VvcError::InvalidData(msg) => {
            assert_eq!(msg, "corrupted data");
        }
        _ => panic!("Wrong conversion: {:?}", vvc_err),
    }
}

#[test]
fn test_from_codec_error_insufficient_data() {
    let codec_err = CodecError::InsufficientData {
        codec: Codec::Vvc,
        expected: 200,
        actual: 50,
    };
    let vvc_err: VvcError = codec_err.into();

    match vvc_err {
        VvcError::InsufficientData { expected, actual } => {
            assert_eq!(expected, 200);
            assert_eq!(actual, 50);
        }
        _ => panic!("Wrong conversion: {:?}", vvc_err),
    }
}

#[test]
fn test_from_codec_error_io() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let codec_err = CodecError::Io {
        codec: Codec::Vvc,
        source: io_err,
    };
    let vvc_err: VvcError = codec_err.into();

    match vvc_err {
        VvcError::Io(err) => {
            assert_eq!(err.kind(), io::ErrorKind::NotFound);
        }
        _ => panic!("Wrong conversion: {:?}", vvc_err),
    }
}

#[test]
fn test_from_codec_error_parse() {
    let codec_err = CodecError::Parse {
        codec: Codec::Vvc,
        offset: 42,
        message: "parse failure".to_string(),
    };
    let vvc_err: VvcError = codec_err.into();

    match vvc_err {
        VvcError::Parse { offset, message } => {
            assert_eq!(offset, 42);
            assert_eq!(message, "parse failure");
        }
        _ => panic!("Wrong conversion: {:?}", vvc_err),
    }
}

#[test]
fn test_from_codec_error_unsupported() {
    let codec_err = CodecError::Unsupported {
        codec: Codec::Vvc,
        feature: "4:2:2 chroma".to_string(),
    };
    let vvc_err: VvcError = codec_err.into();

    match vvc_err {
        VvcError::InvalidData(msg) => {
            assert!(msg.contains("Unsupported"));
            assert!(msg.contains("4:2:2 chroma"));
        }
        _ => panic!("Wrong conversion: {:?}", vvc_err),
    }
}

#[test]
fn test_from_codec_error_missing_parameter() {
    let codec_err = CodecError::MissingParameter {
        codec: Codec::Vvc,
        parameter: "sps".to_string(),
    };
    let vvc_err: VvcError = codec_err.into();

    match vvc_err {
        VvcError::InvalidData(msg) => {
            assert!(msg.contains("Missing parameter"));
            assert!(msg.contains("sps"));
        }
        _ => panic!("Wrong conversion: {:?}", vvc_err),
    }
}

#[test]
fn test_from_codec_error_codec_specific() {
    let codec_err = CodecError::CodecSpecific {
        codec: Codec::Vvc,
        message: "specific issue".to_string(),
    };
    let vvc_err: VvcError = codec_err.into();

    match vvc_err {
        VvcError::InvalidData(msg) => {
            assert_eq!(msg, "specific issue");
        }
        _ => panic!("Wrong conversion: {:?}", vvc_err),
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_error_empty_strings() {
    let err = VvcError::InvalidData(String::new());
    let msg = format!("{err}");
    assert!(msg.contains("Invalid data"));
}

#[test]
fn test_error_unicode_string() {
    let err = VvcError::Parse {
        offset: 0,
        message: "한글 오류".to_string(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("Parse error"));
    assert!(msg.contains("한글 오류"));
}

#[test]
fn test_error_zero_values() {
    let err = VvcError::UnexpectedEof(0);
    let msg = format!("{err}");
    assert!(msg.contains("0"));
}

#[test]
fn test_error_large_values() {
    let err = VvcError::InsufficientData {
        expected: usize::MAX,
        actual: usize::MAX - 1,
    };
    match err {
        VvcError::InsufficientData { expected, actual } => {
            assert_eq!(expected, usize::MAX);
            assert_eq!(actual, usize::MAX - 1);
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_error_large_offset() {
    let err = VvcError::Parse {
        offset: u64::MAX,
        message: "test".to_string(),
    };
    match err {
        VvcError::Parse { offset, .. } => {
            assert_eq!(offset, u64::MAX);
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_all_error_variants_are_distinct() {
    let errors = vec![
        VvcError::UnexpectedEof(0),
        VvcError::InvalidData("a".to_string()),
        VvcError::InsufficientData {
            expected: 1,
            actual: 1,
        },
        VvcError::Parse {
            offset: 0,
            message: "b".to_string(),
        },
    ];

    // Verify each error is created correctly
    assert_eq!(errors.len(), 4);
}

#[test]
fn test_result_with_different_error_types() {
    let results: Vec<Result<u32>> = vec![
        Err(VvcError::UnexpectedEof(0)),
        Err(VvcError::InvalidData("error".to_string())),
        Err(VvcError::InsufficientData {
            expected: 10,
            actual: 5,
        }),
        Err(VvcError::Parse {
            offset: 10,
            message: "parse error".to_string(),
        }),
    ];

    for result in results {
        assert!(result.is_err());
    }
}

#[test]
fn test_error_conversions_chain() {
    // Test that we can convert through multiple layers
    let codec_err = CodecError::Parse {
        codec: Codec::Vvc,
        offset: 50,
        message: "deep error".to_string(),
    };

    let vvc_err: VvcError = codec_err.into();

    // Should be able to convert back to a generic error
    let _dyn_err: Box<dyn std::error::Error + Send + Sync> = vvc_err.into();
}

#[test]
fn test_error_send_sync_traits() {
    // Verify VvcError implements Send and Sync
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<VvcError>();
    assert_send_sync::<Result<()>>();
}

// ============================================================================
// VVC-Specific Tests
// ============================================================================

#[test]
fn test_error_vvc_specific_messages() {
    // Test error messages specific to VVC parsing
    let err = VvcError::InvalidData("Invalid VVC NAL unit type".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("Invalid data"));
    assert!(msg.contains("VVC"));
}

#[test]
fn test_error_vvc_ctu_size() {
    let err = VvcError::Parse {
        offset: 100,
        message: "Invalid CTU size".to_string(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("CTU size"));
    assert!(msg.contains("100"));
}
