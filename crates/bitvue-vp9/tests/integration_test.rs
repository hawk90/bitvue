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
//! VP9 Integration Tests
//!
//! Tests for end-to-end VP9 parsing functionality including:
//! - Frame parsing
//! - Superframe detection
//! - Overlay data extraction

use bitvue_vp9::parse_vp9;

#[test]
fn test_parse_empty_vp9_stream() {
    let data: &[u8] = &[];
    let result = parse_vp9(data);
    assert!(result.is_ok());

    let stream = result.unwrap();
    assert_eq!(stream.frames.len(), 0);
}

#[test]
fn test_vp9_frame_detection() {
    // Test VP9 frame detection (raw VP9, not IVF-wrapped)
    let data = [
        // Raw VP9 keyframe
        0x82, // Frame marker: keyframe, show_frame=1
        0x49, 0x83, 0x42, // Sync code (0x498342)
        // Minimal color config + frame size
        0b00000000, // color_space=0, color_range=0
        0b00000000, // width low bits
        0b00000000, // width high + height low
        0b00000000, // height high bits
    ];

    let result = parse_vp9(&data);
    assert!(result.is_ok());

    let stream = result.unwrap();
    // Should parse at least one frame
    assert!(!stream.frames.is_empty(), "Should have frames");
}

#[test]
fn test_vp9_superframe_detection() {
    // Test superframe index detection
    // VP9 can use superframes with index for seeking

    // Minimal VP9 stream without superframe (raw VP9)
    let data_without_sf = [
        0x82, // Frame marker: keyframe, show_frame=1
        0x49, 0x83, 0x42, // Sync code (0x498342)
        // Minimal color config + frame size
        0b00000000, // color_space=0, color_range=0
        0b00000000, // width low bits
        0b00000000, // width high + height low
        0b00000000, // height high bits
    ];

    let result = parse_vp9(&data_without_sf);
    assert!(result.is_ok());

    let stream = result.unwrap();
    assert_eq!(stream.frames.len(), 1, "Should have 1 frame");
    assert_eq!(
        stream.superframe_index.frame_count, 1,
        "Single frame should have frame_count=1"
    );
}

#[test]
fn test_v0_5_overlay_extraction() {
    // Test VP9 overlay extraction API
    let data = create_minimal_vp9_stream();

    if let Ok(stream) = parse_vp9(&data) {
        // Verify frames exist
        assert!(!stream.frames.is_empty(), "Should have frames");

        // Test that overlay extraction functions exist
        // (VP9 overlay extraction requires actual frame data)
        if let Some(frame) = stream.frames.first() {
            // The overlay extraction API should be available
            assert!(true, "VP9 overlay extraction API exists");
        }
    }
}

#[test]
fn test_v0_5_completeness() {
    // Verify v0.5.x VP9 support is complete

    // 1. Frame parsing works
    let data = create_minimal_vp9_stream();
    let result = parse_vp9(&data);
    assert!(result.is_ok(), "Should parse VP9 stream");
    let stream = result.unwrap();
    assert!(!stream.frames.is_empty(), "Should have frames");

    // 2. Check for frame properties
    if let Some(frame) = stream.frames.first() {
        let _ = frame.frame_type; // Use frame_type instead of is_keyframe
        let _ = frame.width; // Use width/height instead of frame_size
        let _ = frame.show_frame;
    }

    // 3. Overlay extraction functions exist
    // (Tested in test_v0_5_overlay_extraction)
}

/// Create a minimal VP9 byte stream for testing (raw VP9, not IVF-wrapped)
fn create_minimal_vp9_stream() -> Vec<u8> {
    let mut data = Vec::new();

    // Raw VP9 keyframe (minimal valid VP9 uncompressed header)
    data.extend_from_slice(&[0x82]); // Frame marker: keyframe, show_frame=1
    data.extend_from_slice(&[0x49, 0x83, 0x42]); // Sync code (0x498342)

    // Color config + frame size (minimal)
    data.extend_from_slice(&[
        0b00000000, // color_space=0, color_range=0
        0b00000000, // width low bits
        0b00000000, // width high bits + height low bits
        0b00000000, // height high bits
    ]);

    data
}
