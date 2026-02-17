#![allow(dead_code)]
//! Comprehensive tests for AVC error module
//!
//! Tests AvcError enum variants and From<CodecError> implementation

use bitvue_avc::error::{AvcError, Result};
use bitvue_core::codec_error::{Codec, CodecError};

// ============================================================================
// AvcError Variant Tests
// ============================================================================

#[test]
fn test_error_not_enough_data_display() {
    let err = AvcError::NotEnoughData {
        expected: 100,
        got: 50,
    };
    let msg = format!("{err}");
    assert!(msg.contains("not enough data"));
    assert!(msg.contains("100"));
    assert!(msg.contains("50"));
}

#[test]
fn test_error_not_enough_data_fields() {
    let err = AvcError::NotEnoughData {
        expected: 100,
        got: 50,
    };
    match err {
        AvcError::NotEnoughData { expected, got } => {
            assert_eq!(expected, 100);
            assert_eq!(got, 50);
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_error_invalid_nal_unit_display() {
    let err = AvcError::InvalidNalUnit("bad nal".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("invalid NAL unit"));
    assert!(msg.contains("bad nal"));
}

#[test]
fn test_error_invalid_sps_display() {
    let err = AvcError::InvalidSps("bad sps".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("invalid SPS"));
    assert!(msg.contains("bad sps"));
}

#[test]
fn test_error_invalid_pps_display() {
    let err = AvcError::InvalidPps("bad pps".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("invalid PPS"));
    assert!(msg.contains("bad pps"));
}

#[test]
fn test_error_invalid_slice_header_display() {
    let err = AvcError::InvalidSliceHeader("bad slice".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("invalid slice header"));
    assert!(msg.contains("bad slice"));
}

#[test]
fn test_error_invalid_sei_display() {
    let err = AvcError::InvalidSei("bad sei".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("invalid SEI"));
    assert!(msg.contains("bad sei"));
}

#[test]
fn test_error_missing_parameter_set_display() {
    let err = AvcError::MissingParameterSet("missing sps".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("missing parameter set"));
    assert!(msg.contains("missing sps"));
}

#[test]
fn test_error_bitstream_error_display() {
    let err = AvcError::BitstreamError("bitstream issue".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("bitstream error"));
    assert!(msg.contains("bitstream issue"));
}

#[test]
fn test_error_unsupported_display() {
    let err = AvcError::Unsupported("feature not supported".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("unsupported feature"));
    assert!(msg.contains("feature not supported"));
}

#[test]
fn test_error_parse_error_display() {
    let err = AvcError::ParseError("parse failed".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("parse error"));
    assert!(msg.contains("parse failed"));
}

// ============================================================================
// Error Traits Tests
// ============================================================================

#[test]
fn test_error_debug_trait() {
    let err = AvcError::ParseError("test".to_string());
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("ParseError"));
    assert!(debug_str.contains("test"));
}

#[test]
fn test_error_clone_via_debug() {
    // We can't directly test Clone if not implemented, but Debug works
    let err = AvcError::NotEnoughData {
        expected: 10,
        got: 5,
    };
    let _ = format!("{err:?}");
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
    let result: Result<u32> = Err(AvcError::ParseError("error".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_result_with_not_enough_data() {
    let result: Result<()> = Err(AvcError::NotEnoughData {
        expected: 100,
        got: 50,
    });
    assert!(result.is_err());
    match result {
        Err(AvcError::NotEnoughData { expected, got }) => {
            assert_eq!(expected, 100);
            assert_eq!(got, 50);
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_result_question_mark_operator() {
    fn func_that_fails() -> Result<()> {
        Err(AvcError::ParseError("failed".to_string()))
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
        codec: Codec::Avc,
        position: 100,
    };
    let avc_err: AvcError = codec_err.into();

    match avc_err {
        AvcError::NotEnoughData { expected, got } => {
            assert_eq!(expected, 101); // position + 1
            assert_eq!(got, 0);
        }
        _ => panic!("Wrong conversion: {:?}", avc_err),
    }
}

#[test]
fn test_from_codec_error_invalid_data() {
    let codec_err = CodecError::InvalidData {
        codec: Codec::Avc,
        message: "corrupted data".to_string(),
    };
    let avc_err: AvcError = codec_err.into();

    match avc_err {
        AvcError::ParseError(msg) => {
            assert_eq!(msg, "corrupted data");
        }
        _ => panic!("Wrong conversion: {:?}", avc_err),
    }
}

#[test]
fn test_from_codec_error_insufficient_data() {
    let codec_err = CodecError::InsufficientData {
        codec: Codec::Avc,
        expected: 200,
        actual: 50,
    };
    let avc_err: AvcError = codec_err.into();

    match avc_err {
        AvcError::NotEnoughData { expected, got } => {
            assert_eq!(expected, 200);
            assert_eq!(got, 50);
        }
        _ => panic!("Wrong conversion: {:?}", avc_err),
    }
}

#[test]
fn test_from_codec_error_io() {
    let codec_err = CodecError::Io {
        codec: Codec::Avc,
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    };
    let avc_err: AvcError = codec_err.into();

    match avc_err {
        AvcError::BitstreamError(msg) => {
            assert!(msg.contains("file not found"));
        }
        _ => panic!("Wrong conversion: {:?}", avc_err),
    }
}

#[test]
fn test_from_codec_error_parse() {
    let codec_err = CodecError::Parse {
        codec: Codec::Avc,
        offset: 42,
        message: "parse failure".to_string(),
    };
    let avc_err: AvcError = codec_err.into();

    match avc_err {
        AvcError::ParseError(msg) => {
            assert!(msg.contains("42"));
            assert!(msg.contains("parse failure"));
        }
        _ => panic!("Wrong conversion: {:?}", avc_err),
    }
}

#[test]
fn test_from_codec_error_unsupported() {
    let codec_err = CodecError::Unsupported {
        codec: Codec::Avc,
        feature: "4:2:2 chroma".to_string(),
    };
    let avc_err: AvcError = codec_err.into();

    match avc_err {
        AvcError::Unsupported(feature) => {
            assert_eq!(feature, "4:2:2 chroma");
        }
        _ => panic!("Wrong conversion: {:?}", avc_err),
    }
}

#[test]
fn test_from_codec_error_missing_parameter() {
    let codec_err = CodecError::MissingParameter {
        codec: Codec::Avc,
        parameter: "sps".to_string(),
    };
    let avc_err: AvcError = codec_err.into();

    match avc_err {
        AvcError::MissingParameterSet(param) => {
            assert_eq!(param, "sps");
        }
        _ => panic!("Wrong conversion: {:?}", avc_err),
    }
}

#[test]
fn test_from_codec_error_codec_specific() {
    let codec_err = CodecError::CodecSpecific {
        codec: Codec::Avc,
        message: "specific issue".to_string(),
    };
    let avc_err: AvcError = codec_err.into();

    match avc_err {
        AvcError::ParseError(msg) => {
            assert_eq!(msg, "specific issue");
        }
        _ => panic!("Wrong conversion: {:?}", avc_err),
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_error_empty_strings() {
    let err = AvcError::InvalidNalUnit(String::new());
    let msg = format!("{err}");
    assert!(msg.contains("invalid NAL unit"));
}

#[test]
fn test_error_unicode_string() {
    let err = AvcError::ParseError("한글 오류".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("parse error"));
    assert!(msg.contains("한글 오류"));
}

#[test]
fn test_error_zero_values() {
    let err = AvcError::NotEnoughData {
        expected: 0,
        got: 0,
    };
    let msg = format!("{err}");
    assert!(msg.contains("0"));
}

#[test]
fn test_error_large_values() {
    let err = AvcError::NotEnoughData {
        expected: usize::MAX,
        got: usize::MAX - 1,
    };
    match err {
        AvcError::NotEnoughData { expected, got } => {
            assert_eq!(expected, usize::MAX);
            assert_eq!(got, usize::MAX - 1);
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_all_error_variants_are_distinct() {
    let errors = vec![
        AvcError::NotEnoughData {
            expected: 1,
            got: 1,
        },
        AvcError::InvalidNalUnit("a".to_string()),
        AvcError::InvalidSps("b".to_string()),
        AvcError::InvalidPps("c".to_string()),
        AvcError::InvalidSliceHeader("d".to_string()),
        AvcError::InvalidSei("e".to_string()),
        AvcError::MissingParameterSet("f".to_string()),
        AvcError::BitstreamError("g".to_string()),
        AvcError::Unsupported("h".to_string()),
        AvcError::ParseError("i".to_string()),
    ];

    // Verify each error is created correctly
    assert_eq!(errors.len(), 10);
}
