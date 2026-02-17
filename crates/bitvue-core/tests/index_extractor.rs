#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for index extractor API

use bitvue_core::{Av1IndexExtractor, ExtractorFactory, H264IndexExtractor, IndexExtractor};
use std::io::Cursor;

#[test]
fn test_av1_extractor_codec_name() {
    let extractor = Av1IndexExtractor::new();
    assert_eq!(extractor.codec_name(), "AV1");
    assert!(extractor.is_supported());
}

#[test]
fn test_h264_extractor_codec_name() {
    let extractor = H264IndexExtractor::new();
    assert_eq!(extractor.codec_name(), "H.264");
    assert!(extractor.is_supported());
}

#[test]
fn test_h264_empty_stream() {
    let extractor = H264IndexExtractor::new();
    let mut cursor = Cursor::new(vec![]);

    let result = extractor.extract_quick_index(&mut cursor);
    assert!(result.is_err());
}

#[test]
fn test_h264_minimal_idr_frame() {
    let extractor = H264IndexExtractor::new();

    // Create minimal H.264 stream with one IDR frame
    // Start code (4-byte): 0x00 0x00 0x00 0x01
    // NAL unit header: type=5 (IDR)
    // Dummy payload
    let data = vec![
        0x00, 0x00, 0x00, 0x01, // Start code
        0x65, // NAL header: type=5 (IDR), nal_ref_idc=3
        0x00, 0x00, 0x00, 0x00, // Dummy payload
    ];

    let mut cursor = Cursor::new(data);
    let result = extractor.extract_quick_index(&mut cursor);

    assert!(result.is_ok());
    let quick_index = result.unwrap();
    assert_eq!(quick_index.seek_points.len(), 1);
    assert!(quick_index.seek_points[0].is_keyframe);
    assert_eq!(quick_index.seek_points[0].display_idx, 0);
}

#[test]
fn test_h264_multiple_frames() {
    let extractor = H264IndexExtractor::new();

    // Create H.264 stream with IDR + non-IDR frames
    let mut data = Vec::new();

    // IDR frame (keyframe)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
    data.extend_from_slice(&[0x00; 10]); // Payload

    // Non-IDR frame
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41]);
    data.extend_from_slice(&[0x00; 10]); // Payload

    // Another Non-IDR frame
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41]);
    data.extend_from_slice(&[0x00; 10]); // Payload

    let mut cursor = Cursor::new(data);
    let result = extractor.extract_full_index(&mut cursor, None, None);

    assert!(result.is_ok());
    let frames = result.unwrap();
    assert_eq!(frames.len(), 3);
    assert!(frames[0].is_keyframe);
    assert!(!frames[1].is_keyframe);
    assert!(!frames[2].is_keyframe);
}

#[test]
fn test_h264_3byte_start_code() {
    let extractor = H264IndexExtractor::new();

    // Create H.264 stream with 3-byte start code
    // Start code (3-byte): 0x00 0x00 0x01
    let data = vec![
        0x00, 0x00, 0x01, // Start code (3-byte)
        0x65, // NAL header: type=5 (IDR)
        0x00, 0x00, 0x00, 0x00, // Dummy payload
    ];

    let mut cursor = Cursor::new(data);
    let result = extractor.extract_quick_index(&mut cursor);

    assert!(result.is_ok());
    let quick_index = result.unwrap();
    assert_eq!(quick_index.seek_points.len(), 1);
    assert!(quick_index.seek_points[0].is_keyframe);
}

#[test]
fn test_h264_frame_size_calculation() {
    let extractor = H264IndexExtractor::new();

    // Create H.264 stream with two frames to test size calculation
    let mut data = Vec::new();

    // First frame at offset 0
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
    data.extend_from_slice(&[0xAA; 20]); // 20-byte payload

    // Second frame at offset 25
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41]);
    data.extend_from_slice(&[0xBB; 15]); // 15-byte payload

    let mut cursor = Cursor::new(data.clone());
    let result = extractor.extract_full_index(&mut cursor, None, None);

    assert!(result.is_ok());
    let frames = result.unwrap();
    assert_eq!(frames.len(), 2);

    // First frame size should be from offset 0 to offset 25 = 25 bytes
    assert_eq!(frames[0].byte_offset, 0);
    assert_eq!(frames[0].size, 25);

    // Second frame size should be from offset 25 to end
    assert_eq!(frames[1].byte_offset, 25);
    assert_eq!(frames[1].size, (data.len() - 25) as u64);
}

#[test]
fn test_h264_cancellation() {
    let extractor = H264IndexExtractor::new();

    // Create H.264 stream
    let mut data = Vec::new();
    for _ in 0..10 {
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
        data.extend_from_slice(&[0x00; 10]);
    }

    let mut cursor = Cursor::new(data);
    let cancel_fn = || true; // Always cancel

    let result = extractor.extract_full_index(&mut cursor, None, Some(&cancel_fn));

    assert!(result.is_err());
}

#[test]
fn test_h264_progress_callback() {
    use std::cell::RefCell;

    let extractor = H264IndexExtractor::new();

    let mut data = Vec::new();
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
    data.extend_from_slice(&[0x00; 100]);

    let mut cursor = Cursor::new(data);
    let progress_updates = RefCell::new(Vec::new());

    let result = extractor.extract_full_index(
        &mut cursor,
        Some(&|progress, msg| {
            progress_updates
                .borrow_mut()
                .push((progress, msg.to_string()));
        }),
        None,
    );

    assert!(result.is_ok());
    let updates = progress_updates.borrow();
    assert!(!updates.is_empty());
    assert!(updates.last().unwrap().0 >= 0.99); // Should report near completion
}

#[test]
fn test_factory_create_av1() {
    let extractor = ExtractorFactory::create("av1");
    assert_eq!(extractor.codec_name(), "AV1");
    assert!(extractor.is_supported());
}

#[test]
fn test_factory_create_h264() {
    let extractor = ExtractorFactory::create("h264");
    assert_eq!(extractor.codec_name(), "H.264");
    assert!(extractor.is_supported());
}

#[test]
fn test_factory_create_unsupported() {
    let extractor = ExtractorFactory::create("unknown");
    assert_eq!(extractor.codec_name(), "Unsupported");
    assert!(!extractor.is_supported());
}

#[test]
fn test_factory_from_extension() {
    let extractor = ExtractorFactory::from_extension("ivf");
    assert_eq!(extractor.codec_name(), "AV1");

    let extractor = ExtractorFactory::from_extension("h264");
    assert_eq!(extractor.codec_name(), "H.264");

    let extractor = ExtractorFactory::from_extension("unknown");
    assert_eq!(extractor.codec_name(), "Unsupported");
}

#[test]
fn test_av1_empty_stream() {
    let extractor = Av1IndexExtractor::new();
    let mut cursor = Cursor::new(vec![]);

    let result = extractor.extract_quick_index(&mut cursor);
    assert!(result.is_err());
}

#[test]
fn test_av1_minimal_stream() {
    let extractor = Av1IndexExtractor::new();

    // Create minimal AV1 stream with one sequence header OBU
    // OBU header: type=1 (sequence header), has_size=1
    // OBU size: 5 bytes (LEB128)
    // Dummy sequence header data
    let data = vec![
        0x0A, // OBU header: type=1, has_size=1
        0x05, // Size = 5 bytes
        0x00, 0x00, 0x00, 0x00, 0x00, // Dummy data
    ];

    let mut cursor = Cursor::new(data);
    let result = extractor.extract_quick_index(&mut cursor);

    assert!(result.is_ok());
    let quick_index = result.unwrap();
    assert_eq!(quick_index.seek_points.len(), 1);
    assert!(quick_index.seek_points[0].is_keyframe);
}
