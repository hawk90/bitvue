//! Frame parsing and data structures for overlay extraction
//!
//! Provides ParsedFrame struct and related types for caching parsed OBU data.

mod parse;
mod pixel_info;
mod types;

pub use pixel_info::{extract_pixel_info, PixelInfo};
pub use types::{FrameDimensions, FrameTypeInfo, ObuRef, ParsedFrame};

#[cfg(test)]
use parse::collect_obus_resilient;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame_header_full::{parse_frame_header_full, RefFrameState};
    use crate::{parse_all_obus, parse_frame_header_basic, ObuType};
    use std::sync::Arc;

    #[allow(dead_code)]
    fn create_test_obu_data() -> Vec<u8> {
        // Minimal OBU data with sequence header and frame header
        let mut data = Vec::new();

        // Temporal delimiter OBU (type 2, size 0)
        data.extend_from_slice(&[0x12, 0x00]);

        // Sequence header OBU (type 1, size ~20)
        data.extend_from_slice(&[0x0A, 0x14]); // OBU header
        data.extend_from_slice(&[0x00u8; 20]); // Payload placeholder

        // Frame header OBU (type 3, size ~10)
        data.extend_from_slice(&[0x1A, 0x0A]); // OBU header
        data.extend_from_slice(&[0x00u8; 10]); // Payload placeholder

        data
    }

    /// The first `Frame` OBU of a real clip (a key frame): its sequence header and payload.
    fn first_frame() -> (crate::SequenceHeader, Vec<u8>) {
        const CLIP: &[u8] = include_bytes!("../../../../../test_data/av1_aomenc_testsrc2.ivf");
        let (_h, packets) = crate::parse_ivf_frames(CLIP).unwrap();
        let (mut seq, mut frame) = (None, None);
        for obu in parse_all_obus(&packets[0].data).unwrap() {
            match obu.header.obu_type {
                ObuType::SequenceHeader => seq = crate::parse_sequence_header(&obu.payload).ok(),
                ObuType::Frame => frame = Some(obu.payload.to_vec()),
                _ => {}
            }
        }
        (seq.unwrap(), frame.unwrap())
    }

    #[test]
    fn apply_sequence_header_copies_size_chroma_format_and_tool_switches() {
        let (mut seq, _payload) = first_frame();
        // Alternate the values so that a copied-wrong or inverted field cannot match by chance.
        seq.max_frame_width = 200;
        seq.max_frame_height = 100;
        seq.use_128x128_superblock = true;
        seq.color_config.mono_chrome = true;
        seq.color_config.subsampling_x = true;
        seq.color_config.subsampling_y = false;
        seq.enable_filter_intra = true;
        seq.enable_interintra_compound = false;
        seq.enable_dual_filter = true;
        seq.enable_masked_compound = false;
        seq.enable_jnt_comp = true;
        seq.enable_warped_motion = false;
        let mut frame = ParsedFrame::scaffold(Arc::from([0u8]));

        frame.apply_sequence_header(&seq);

        assert_eq!(
            (frame.dimensions.width, frame.dimensions.height),
            (200, 100)
        );
        assert_eq!(frame.dimensions.sb_size, 128);
        assert!(frame.mono_chrome);
        assert_eq!((frame.subsampling_x, frame.subsampling_y), (true, false));
        assert_eq!(
            (
                frame.enable_filter_intra,
                frame.enable_interintra_compound,
                frame.enable_dual_filter,
                frame.enable_masked_compound,
                frame.enable_jnt_comp,
                frame.enable_warped_motion,
            ),
            (true, false, true, false, true, false)
        );

        seq.use_128x128_superblock = false;
        seq.color_config.mono_chrome = false;
        seq.color_config.subsampling_x = false;
        seq.color_config.subsampling_y = true;
        seq.enable_filter_intra = false;
        seq.enable_interintra_compound = true;
        seq.enable_dual_filter = false;
        seq.enable_masked_compound = true;
        seq.enable_jnt_comp = false;
        seq.enable_warped_motion = true;

        frame.apply_sequence_header(&seq);

        assert_eq!(frame.dimensions.sb_size, 64);
        assert!(!frame.mono_chrome);
        assert_eq!((frame.subsampling_x, frame.subsampling_y), (false, true));
        assert_eq!(
            (
                frame.enable_filter_intra,
                frame.enable_interintra_compound,
                frame.enable_dual_filter,
                frame.enable_masked_compound,
                frame.enable_jnt_comp,
                frame.enable_warped_motion,
            ),
            (false, true, false, true, false, true)
        );
    }

    #[test]
    fn collect_obus_resilient_keeps_the_obus_before_a_truncated_one() {
        // Two temporal delimiters (type 2, size 0), then an OBU that claims 20 payload bytes but
        // has one: `parse_all_obus` fails on the whole packet, the fallback keeps the first two.
        let packet = [0x12, 0x00, 0x12, 0x00, 0x0A, 0x14, 0x00];
        assert!(parse_all_obus(&packet).is_err());

        let obus = collect_obus_resilient(&packet);

        assert_eq!(obus.len(), 2);
        assert!(obus
            .iter()
            .all(|o| o.header.obu_type == ObuType::TemporalDelimiter));
    }

    #[test]
    fn collect_obus_resilient_returns_every_obu_of_a_well_formed_packet() {
        let packet = [0x12, 0x00, 0x12, 0x00, 0x12, 0x00];

        assert_eq!(collect_obus_resilient(&packet).len(), 3);
    }

    #[test]
    fn apply_basic_header_counts_a_key_frame_as_intra() {
        let (_seq, payload) = first_frame();
        let header = parse_frame_header_basic(&payload).unwrap();
        let mut frame = ParsedFrame::scaffold(Arc::from([0u8]));

        frame.apply_basic_header(&header);

        // `FrameIsIntra` covers KEY_FRAME too; `FrameType::is_intra_only` would not.
        assert!(frame.frame_type.is_intra_only);
        assert_eq!(frame.frame_type.base_qp, header.base_q_idx);
        assert_eq!(frame.delta_q_enabled, header.delta_q_present);
    }

    #[test]
    fn apply_full_header_derives_coded_lossless_and_key_frame() {
        let (seq, payload) = first_frame();
        let full = parse_frame_header_full(&payload, &seq, &mut RefFrameState::new()).unwrap();
        let apply = |header: &crate::frame_header::FrameHeader| {
            let mut frame = ParsedFrame::scaffold(Arc::from([0u8]));
            frame.apply_full_header(header, &seq, &[0; 8]);
            frame
        };
        assert!(apply(&full).key_frame, "a shown key frame");

        let mut shown_again = full.clone();
        shown_again.show_existing_frame = true;
        assert!(
            !apply(&shown_again).key_frame,
            "showing an existing frame is not a new one"
        );

        // Lossless needs a zero base index and zero DC deltas, all three.
        let mut lossless = full.clone();
        lossless.base_q_idx = Some(0);
        lossless.y_dc_delta_q = Some(0);
        lossless.uv_dc_delta_q = Some(0);
        assert!(apply(&lossless).coded_lossless);
        lossless.uv_dc_delta_q = Some(1);
        assert!(!apply(&lossless).coded_lossless);
        lossless.uv_dc_delta_q = Some(0);
        lossless.base_q_idx = Some(1);
        assert!(!apply(&lossless).coded_lossless);
    }

    #[test]
    fn apply_full_header_keeps_the_basic_base_qp_when_it_has_none() {
        let (seq, payload) = first_frame();
        let basic = parse_frame_header_basic(&payload).unwrap();
        let mut full = parse_frame_header_full(&payload, &seq, &mut RefFrameState::new()).unwrap();
        full.base_q_idx = None;
        let mut frame = ParsedFrame::scaffold(Arc::from([0u8]));

        frame.apply_basic_header(&basic);
        frame.apply_full_header(&full, &seq, &[0; 8]);

        assert_eq!(frame.frame_type.base_qp, basic.base_q_idx);
        assert!(basic.base_q_idx.is_some());
    }

    #[test]
    fn test_parsed_frame_default_dimensions() {
        // Arrange
        let dims = FrameDimensions::default();

        // Assert
        assert_eq!(dims.width, 1920);
        assert_eq!(dims.height, 1080);
        assert_eq!(dims.sb_size, 64);
    }

    #[test]
    fn test_obu_ref_copy() {
        // Arrange
        let obu_ref = ObuRef {
            obu_type: ObuType::SequenceHeader,
            payload_start: 10,
            payload_end: 20,
        };

        // Act & Assert: Test that ObuRef is Copy (can be used in iterators)
        let _copy = obu_ref;
        let _another_copy = obu_ref;
    }

    #[test]
    fn test_extract_pixel_info_with_empty_data() {
        // Arrange: Empty OBU data
        let obu_data = vec![];

        // Act
        let result = extract_pixel_info(&obu_data, 0, 100, 200);

        // Assert: Should still return PixelInfo with defaults
        assert!(
            result.is_ok(),
            "Pixel info extraction should handle empty data"
        );
        let info = result.unwrap();
        assert_eq!(info.frame_index, 0);
        assert_eq!(info.pixel_x, 100);
        assert_eq!(info.pixel_y, 200);
    }
}
