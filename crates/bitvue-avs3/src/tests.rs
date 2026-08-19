//! Integration tests for the AVS3 parser.

use crate::frames::extract_avs3_frames;
use crate::nal::{scan_nal_units, Sci};

// ── NAL unit scanning ─────────────────────────────────────────────────────────

#[test]
fn scan_empty_buffer() {
    let units = scan_nal_units(&[]);
    assert!(units.is_empty());
}

#[test]
fn scan_no_start_codes() {
    let data = [0xAA, 0xBB, 0xCC, 0xDD];
    let units = scan_nal_units(&data);
    assert!(units.is_empty());
}

#[test]
fn scan_sequence_header_unit() {
    // Minimal synthetic AVS3 buffer: seq header + I-frame start code
    let data = [
        0x00, 0x00, 0x01, 0xB0, // seq header SCI
        0x20, 0x01, // profile=0x20 (Main8), level=1
        0xFF, 0xFF, // padding bytes
        0x00, 0x00, 0x01, 0xB3, // I-frame start
        0x00, 0xFF,
    ];
    let units = scan_nal_units(&data);
    assert_eq!(units.len(), 2);
    assert_eq!(units[0].sci, Sci::SequenceHeader);
    assert_eq!(units[1].sci, Sci::IFrame);
}

// ── Frame extraction ──────────────────────────────────────────────────────────

#[test]
fn extract_empty_buffer_returns_no_frames() {
    let result = extract_avs3_frames(&[], 0).unwrap();
    assert!(result.frames.is_empty());
    assert!(result.sequence_header.is_none());
    assert_eq!(result.parse_errors, 0);
}

#[test]
fn extract_respects_limit() {
    // Three I-frame start codes — limit to 2
    let mut data = Vec::new();
    for _ in 0..3 {
        // Minimal I-frame: SCI byte + 6 more bytes so the parser can read
        data.extend_from_slice(&[0x00, 0x00, 0x01, 0xB3]);
        // Add enough bytes for the bitreader to not hit EOF immediately
        data.extend_from_slice(&[0x00; 10]);
    }
    let result = extract_avs3_frames(&data, 2).unwrap();
    // May succeed or produce parse errors depending on synthetic data validity,
    // but frame count must not exceed limit.
    assert!(result.frames.len() <= 2);
}

// ── Overlay extraction ────────────────────────────────────────────────────────

#[test]
fn qp_grid_none_without_dimensions() {
    use crate::frames::Avs3Frame;
    use crate::overlay_extraction::extract_qp_grid;
    use crate::picture_header::PictureType;
    let frame = Avs3Frame {
        frame_index: 0,
        picture_type: PictureType::I,
        qp: 30,
        display_delay: 0,
        offset: 0,
        size: 100,
        data: vec![],
        esao_enable: false,
        ccsao_enable: false,
    };
    // No sequence header → no dimensions → None
    let grid = extract_qp_grid(&frame, None);
    assert!(grid.is_none());
}

#[test]
fn qp_grid_correct_dimensions() {
    use crate::frames::Avs3Frame;
    use crate::overlay_extraction::extract_qp_grid;
    use crate::picture_header::PictureType;
    use crate::sequence_header::{Avs3Profile, ChromaFormat, SequenceHeader};

    let seq = SequenceHeader {
        profile: Avs3Profile::Main8,
        level: 0x20,
        progressive_sequence: true,
        width: 1920,
        height: 1080,
        chroma_format: ChromaFormat::Yuv420,
        bit_depth_luma: 8,
        bit_depth_chroma: 8,
        sar_width: 1,
        sar_height: 1,
        frame_rate_code: 5,
        bit_rate: 0,
        low_delay: false,
        temporal_id_nesting: false,
        deblocking_filter_flag: true,
        sample_adaptive_offset_enabled: false,
        adaptive_leveling_filter_enabled: false,
        cross_component_prediction_enabled: false,
        adaptive_loop_filter_enabled: false,
    };

    let frame = Avs3Frame {
        frame_index: 0,
        picture_type: PictureType::I,
        qp: 28,
        display_delay: 0,
        offset: 0,
        size: 200,
        data: vec![],
        esao_enable: false,
        ccsao_enable: false,
    };

    let grid = extract_qp_grid(&frame, Some(&seq)).unwrap();
    // 1920 / 128 = 15 CTUs wide, 1080 / 128 = 9 CTUs high (ceil)
    assert_eq!(grid.grid_w, 15);
    assert_eq!(grid.grid_h, 9);
    assert_eq!(grid.qp.len(), 135);
    assert!(grid.qp.iter().all(|&q| q == 28));
}

#[test]
fn esao_map_none_when_disabled() {
    use crate::frames::Avs3Frame;
    use crate::overlay_extraction::extract_esao_map;
    use crate::picture_header::PictureType;

    let frame = Avs3Frame {
        frame_index: 0,
        picture_type: PictureType::I,
        qp: 30,
        display_delay: 0,
        offset: 0,
        size: 100,
        data: vec![],
        esao_enable: false, // disabled
        ccsao_enable: false,
    };
    assert!(extract_esao_map(&frame, None).is_none());
}
