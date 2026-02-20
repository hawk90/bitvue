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
//! Tests for index_extractor_evidence module

use crate::{
    indexing::{FrameMetadata, SeekPoint},
    H264IndexExtractor, IndexExtractor, IndexExtractorEvidenceManager,
};
use std::io::Cursor;

fn create_test_seekpoint(display_idx: usize, byte_offset: u64) -> SeekPoint {
    SeekPoint {
        display_idx,
        byte_offset,
        is_keyframe: true,
        pts: Some(display_idx as u64 * 1000),
    }
}

fn create_test_frame_metadata(
    display_idx: usize,
    decode_idx: usize,
    byte_offset: u64,
    size: u64,
    is_keyframe: bool,
) -> FrameMetadata {
    FrameMetadata {
        display_idx,
        decode_idx,
        byte_offset,
        size,
        is_keyframe,
        pts: Some(display_idx as u64 * 1000),
        dts: Some(decode_idx as u64 * 1000),
        frame_type: Some(if is_keyframe {
            "I".to_string()
        } else {
            "P".to_string()
        }),
    }
}

#[test]
fn test_create_seekpoint_evidence() {
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    let seekpoint = create_test_seekpoint(0, 0);

    let evidence = manager.create_seekpoint_evidence(&seekpoint);

    assert_eq!(evidence.display_idx, 0);
    assert!(manager.get_frame_evidence(0).is_some());
    assert!(manager.get_evidence_by_offset(0).is_some());
}

#[test]
fn test_create_frame_metadata_evidence() {
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    let frame = create_test_frame_metadata(0, 0, 0, 1024, true);

    let evidence = manager.create_frame_metadata_evidence(&frame);

    assert_eq!(evidence.display_idx, 0);
    assert!(manager.get_frame_evidence(0).is_some());
    assert!(manager.get_evidence_by_offset(0).is_some());
}

#[test]
fn test_multiple_frames() {
    let mut manager = IndexExtractorEvidenceManager::new_empty();

    for i in 0..10 {
        let frame = create_test_frame_metadata(i, i, i as u64 * 1024, 1024, i % 5 == 0);
        manager.create_frame_metadata_evidence(&frame);
    }

    assert_eq!(manager.frame_count(), 10);
    assert!(manager.get_frame_evidence(0).is_some());
    assert!(manager.get_frame_evidence(9).is_some());
    assert!(manager.get_frame_evidence(10).is_none());
}

#[test]
fn test_trace_to_bit_offset() {
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    let frame = create_test_frame_metadata(5, 5, 5000, 2048, true);

    manager.create_frame_metadata_evidence(&frame);

    let bit_range = manager.trace_to_bit_offset(5).unwrap();
    assert_eq!(bit_range.byte_offset(), 5000);
    assert_eq!(bit_range.size_bits(), 2048 * 8);
}

#[test]
fn test_trace_to_display_idx() {
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    let frame = create_test_frame_metadata(3, 3, 3000, 1024, false);

    manager.create_frame_metadata_evidence(&frame);

    let display_idx = manager.trace_to_display_idx(3000).unwrap();
    assert_eq!(display_idx, 3);
}

#[test]
fn test_update_seekpoint_size() {
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    let seekpoint = create_test_seekpoint(0, 0);

    manager.create_seekpoint_evidence(&seekpoint);

    // Initial size is estimated
    let initial_range = manager.trace_to_bit_offset(0).unwrap();
    assert_eq!(initial_range.size_bits(), 1024 * 8);

    // Update with actual size
    manager.update_seekpoint_size(0, 2048);

    // Size should be updated
    let updated_range = manager.trace_to_bit_offset(0).unwrap();
    assert_eq!(updated_range.size_bits(), 2048 * 8);
}

#[test]
fn test_all_frame_evidence_sorted() {
    let mut manager = IndexExtractorEvidenceManager::new_empty();

    // Add frames out of order
    for i in [5, 2, 8, 1, 9, 0, 3, 7, 4, 6] {
        let frame = create_test_frame_metadata(i, i, i as u64 * 1000, 1024, false);
        manager.create_frame_metadata_evidence(&frame);
    }

    let all = manager.all_frame_evidence();
    assert_eq!(all.len(), 10);

    // Verify sorted order
    for (i, evidence) in all.iter().enumerate() {
        assert_eq!(evidence.display_idx, i);
    }
}

#[test]
fn test_evidence_chain_bidirectional() {
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    let frame = create_test_frame_metadata(0, 0, 0, 1024, true);

    let evidence = manager.create_frame_metadata_evidence(&frame);

    // Forward: display_idx → bit_offset
    let bit_range = manager.trace_to_bit_offset(0).unwrap();
    assert_eq!(bit_range.byte_offset(), 0);

    // Backward: byte_offset → display_idx
    let display_idx = manager.trace_to_display_idx(0).unwrap();
    assert_eq!(display_idx, 0);

    // Verify links
    let chain = manager.evidence_chain();
    let bit_ev = chain
        .bit_offset_index
        .find_by_id(&evidence.bit_offset_id)
        .unwrap();
    assert!(bit_ev.syntax_link.is_some());
    assert_eq!(bit_ev.syntax_link.as_ref().unwrap(), &evidence.syntax_id);
}

#[test]
fn test_metadata_preservation() {
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    let frame = create_test_frame_metadata(0, 0, 0, 1024, true);

    let evidence = manager.create_frame_metadata_evidence(&frame);

    let chain = manager.evidence_chain();
    let syntax_ev = chain.syntax_index.find_by_id(&evidence.syntax_id).unwrap();

    assert_eq!(
        syntax_ev.metadata.get("is_keyframe"),
        Some(&"true".to_string())
    );
    assert_eq!(
        syntax_ev.metadata.get("display_idx"),
        Some(&"0".to_string())
    );
    assert_eq!(syntax_ev.metadata.get("decode_idx"), Some(&"0".to_string()));
    assert_eq!(
        syntax_ev.metadata.get("size_bytes"),
        Some(&"1024".to_string())
    );
    assert_eq!(syntax_ev.metadata.get("pts"), Some(&"0".to_string()));
    assert_eq!(syntax_ev.metadata.get("frame_type"), Some(&"I".to_string()));
}

#[test]
fn test_clear() {
    let mut manager = IndexExtractorEvidenceManager::new_empty();

    for i in 0..5 {
        let frame = create_test_frame_metadata(i, i, i as u64 * 1000, 1024, false);
        manager.create_frame_metadata_evidence(&frame);
    }

    assert_eq!(manager.frame_count(), 5);

    manager.clear();

    assert_eq!(manager.frame_count(), 0);
    assert!(manager.get_frame_evidence(0).is_none());
    assert!(manager.get_evidence_by_offset(0).is_none());
}

// H.264-specific tests
// Deliverable: evidence_chain_01_bit_offset:Indexing:Core:H264:evidence_chain

#[test]
fn test_h264_frame_evidence_integration() {
    // Create H.264 stream with IDR frame
    let data = vec![
        0x00, 0x00, 0x00, 0x01, // Start code
        0x65, // NAL header: type=5 (IDR)
        0xAA, 0xBB, 0xCC, 0xDD, // Dummy payload
    ];

    let extractor = H264IndexExtractor::new();
    let mut cursor = Cursor::new(data);
    let frames = extractor
        .extract_full_index(&mut cursor, None, None)
        .unwrap();

    // Create evidence for H.264 frames
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    for frame in &frames {
        manager.create_frame_metadata_evidence(frame);
    }

    assert_eq!(manager.frame_count(), 1);
    let evidence = manager.get_frame_evidence(0).unwrap();
    assert_eq!(evidence.display_idx, 0);

    // Verify bit range
    let bit_range = manager.trace_to_bit_offset(0).unwrap();
    assert_eq!(bit_range.byte_offset(), 0);
}

#[test]
fn test_h264_multiple_frames_evidence() {
    // Create H.264 stream with multiple frames
    let mut data = Vec::new();

    // IDR frame at offset 0
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
    data.extend_from_slice(&[0xAA; 20]);

    // Non-IDR frame at offset 25
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41]);
    data.extend_from_slice(&[0xBB; 15]);

    // Another Non-IDR frame at offset 45
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41]);
    data.extend_from_slice(&[0xCC; 10]);

    let extractor = H264IndexExtractor::new();
    let mut cursor = Cursor::new(data);
    let frames = extractor
        .extract_full_index(&mut cursor, None, None)
        .unwrap();

    // Create evidence for all H.264 frames
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    for frame in &frames {
        manager.create_frame_metadata_evidence(frame);
    }

    assert_eq!(manager.frame_count(), 3);

    // Verify first frame (keyframe)
    let bit_range_0 = manager.trace_to_bit_offset(0).unwrap();
    assert_eq!(bit_range_0.byte_offset(), 0);

    // Verify second frame
    let bit_range_1 = manager.trace_to_bit_offset(1).unwrap();
    assert_eq!(bit_range_1.byte_offset(), 25);

    // Verify third frame
    let bit_range_2 = manager.trace_to_bit_offset(2).unwrap();
    assert_eq!(bit_range_2.byte_offset(), 45);

    // Verify bidirectional lookup
    let display_idx = manager.trace_to_display_idx(25).unwrap();
    assert_eq!(display_idx, 1);
}

#[test]
fn test_h264_keyframe_metadata_preservation() {
    // Create H.264 stream with IDR frame
    let data = vec![
        0x00, 0x00, 0x00, 0x01, // Start code
        0x65, // NAL header: type=5 (IDR)
        0x00, 0x00, 0x00, 0x00, // Dummy payload
    ];

    let extractor = H264IndexExtractor::new();
    let mut cursor = Cursor::new(data);
    let frames = extractor
        .extract_full_index(&mut cursor, None, None)
        .unwrap();

    // Create evidence
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    let evidence = manager.create_frame_metadata_evidence(&frames[0]);

    // Verify metadata preservation
    let chain = manager.evidence_chain();
    let syntax_ev = chain.syntax_index.find_by_id(&evidence.syntax_id).unwrap();

    assert_eq!(
        syntax_ev.metadata.get("is_keyframe"),
        Some(&"true".to_string())
    );
    assert_eq!(syntax_ev.metadata.get("frame_type"), Some(&"I".to_string()));
    assert!(syntax_ev.node_label.contains("frame_0_I"));
}

#[test]
fn test_h264_non_idr_metadata() {
    // Create H.264 stream with non-IDR frame
    let mut data = Vec::new();

    // IDR first
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
    data.extend_from_slice(&[0x00; 10]);

    // Then non-IDR
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41]);
    data.extend_from_slice(&[0x00; 10]);

    let extractor = H264IndexExtractor::new();
    let mut cursor = Cursor::new(data);
    let frames = extractor
        .extract_full_index(&mut cursor, None, None)
        .unwrap();

    // Create evidence for second frame (non-IDR)
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    let evidence = manager.create_frame_metadata_evidence(&frames[1]);

    // Verify metadata
    let chain = manager.evidence_chain();
    let syntax_ev = chain.syntax_index.find_by_id(&evidence.syntax_id).unwrap();

    assert_eq!(
        syntax_ev.metadata.get("is_keyframe"),
        Some(&"false".to_string())
    );
    assert_eq!(syntax_ev.metadata.get("frame_type"), Some(&"P".to_string()));
    assert!(syntax_ev.node_label.contains("frame_1_P"));
}

#[test]
fn test_h264_evidence_chain_navigation() {
    // Create H.264 stream
    let mut data = Vec::new();
    for i in 0..5 {
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
        data.extend_from_slice(&vec![i as u8; 10]);
    }

    let extractor = H264IndexExtractor::new();
    let mut cursor = Cursor::new(data);
    let frames = extractor
        .extract_full_index(&mut cursor, None, None)
        .unwrap();

    // Build evidence for all frames
    let mut manager = IndexExtractorEvidenceManager::new_empty();
    for frame in &frames {
        manager.create_frame_metadata_evidence(frame);
    }

    assert_eq!(manager.frame_count(), 5);

    // Test navigation for each frame
    for i in 0..5 {
        let evidence = manager.get_frame_evidence(i).unwrap();

        // Forward: display_idx → bit_offset
        let bit_range = manager.trace_to_bit_offset(i).unwrap();
        // Verify we got a valid bit range
        assert!(bit_range.size_bits() > 0);

        // Backward: byte_offset → display_idx
        let display_idx = manager
            .trace_to_display_idx(bit_range.byte_offset())
            .unwrap();
        assert_eq!(display_idx, i);

        // Verify chain linkage
        let chain = manager.evidence_chain();
        let bit_ev = chain
            .bit_offset_index
            .find_by_id(&evidence.bit_offset_id)
            .unwrap();
        assert_eq!(bit_ev.syntax_link.as_ref().unwrap(), &evidence.syntax_id);
    }
}
