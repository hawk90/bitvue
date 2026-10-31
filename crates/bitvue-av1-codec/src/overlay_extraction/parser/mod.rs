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
mod parse_test;

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

    // ---- ParsedFrame::parse / parse_with_ref_state over real and rebuilt OBU streams ----

    const CLIP: &[u8] = include_bytes!("../../../../../test_data/av1_aomenc_testsrc2.ivf");

    /// One OBU with a size field, as it sits in an IVF packet.
    fn obu(obu_type: ObuType, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![((obu_type as u8) << 3) | 0x02];
        out.extend(crate::encode_uleb128(payload.len() as u64));
        out.extend_from_slice(payload);
        out
    }

    /// The pieces of one clip packet: sequence header (if present), frame payload, whole packet.
    struct Packet {
        seq: Option<Vec<u8>>,
        /// The last `Frame` OBU payload (empty for a packet with only a `FrameHeader`).
        frame: Vec<u8>,
        /// Every `Frame` / `FrameHeader` OBU of the packet, in order.
        frames: Vec<(ObuType, Vec<u8>)>,
        data: Vec<u8>,
    }

    fn packets() -> Vec<Packet> {
        let (_h, ivf) = crate::parse_ivf_frames(CLIP).unwrap();
        ivf.iter()
            .map(|p| {
                let (mut seq, mut frame, mut frames) = (None, None, Vec::new());
                for o in parse_all_obus(&p.data).unwrap() {
                    match o.header.obu_type {
                        ObuType::SequenceHeader => seq = Some(o.payload.to_vec()),
                        ObuType::Frame => {
                            frame = Some(o.payload.to_vec());
                            frames.push((ObuType::Frame, o.payload.to_vec()));
                        }
                        ObuType::FrameHeader => {
                            frames.push((ObuType::FrameHeader, o.payload.to_vec()));
                        }
                        _ => {}
                    }
                }
                Packet {
                    seq,
                    frame: frame.unwrap_or_default(),
                    frames,
                    data: p.data.clone(),
                }
            })
            .collect()
    }

    fn td() -> Vec<u8> {
        obu(ObuType::TemporalDelimiter, &[])
    }

    fn types(frame: &ParsedFrame) -> Vec<ObuType> {
        frame.obus.iter().map(|o| o.obu_type).collect()
    }

    /// The exact header parse of packet 0's frame, to compute expected values independently.
    fn key_header(packets: &[Packet]) -> (crate::SequenceHeader, crate::frame_header::FrameHeader) {
        let seq = crate::parse_sequence_header(packets[0].seq.as_ref().unwrap()).unwrap();
        let full =
            parse_frame_header_full(&packets[0].frame, &seq, &mut RefFrameState::new()).unwrap();
        (seq, full)
    }

    #[test]
    fn empty_data_parses_to_the_default_scaffold() {
        let frame = ParsedFrame::parse(&[]).unwrap();
        assert!(frame.obus.is_empty());
        assert!(frame.tile_data.is_empty() && frame.tile_groups.is_empty());
        assert_eq!(
            (frame.dimensions.width, frame.dimensions.height),
            (1920, 1080)
        );
        assert!(!frame.key_frame && !frame.frame_type.is_intra_only);
        assert_eq!(frame.primary_ref_frame, 7);
    }

    #[test]
    fn a_real_key_frame_packet_fills_dimensions_header_values_and_tile_data() {
        let packets = packets();
        let (seq, full) = key_header(&packets);
        let frame = ParsedFrame::parse(&packets[0].data).unwrap();

        assert_eq!(
            types(&frame),
            vec![
                ObuType::TemporalDelimiter,
                ObuType::SequenceHeader,
                ObuType::Frame
            ]
        );
        assert_eq!(frame.dimensions.width, seq.max_frame_width);
        assert_eq!(frame.dimensions.height, seq.max_frame_height);
        let sb = if seq.use_128x128_superblock { 128 } else { 64 };
        assert_eq!(frame.dimensions.sb_size, sb);
        // The superblock grid rounds up: 320x240 with 128-pixel superblocks is 3x2, not 2x1.
        assert_eq!(frame.dimensions.sb_cols, seq.max_frame_width.div_ceil(sb));
        assert_eq!(frame.dimensions.sb_rows, seq.max_frame_height.div_ceil(sb));
        assert_ne!(
            seq.max_frame_width % sb,
            0,
            "clip must exercise the rounding"
        );

        assert!(frame.key_frame && frame.frame_type.is_intra_only);
        assert_eq!(frame.frame_type.base_qp, full.base_q_idx);
        assert_eq!(frame.order_hint, full.order_hint);
        assert_eq!(frame.refresh_frame_flags, full.refresh_frame_flags.unwrap());
        assert_eq!(frame.primary_ref_frame, full.primary_ref_frame);
        assert_eq!(frame.cdef_bits, full.cdef_damping.bits);

        let tile = &packets[0].frame[full.header_size_bytes..];
        assert_eq!(&frame.tile_data[..], tile);
        assert_eq!(frame.tile_groups, vec![0..tile.len()]);
    }

    #[test]
    fn parse_is_parse_with_a_fresh_reference_state() {
        let packets = packets();
        let a = ParsedFrame::parse(&packets[0].data).unwrap();
        let b =
            ParsedFrame::parse_with_ref_state(&packets[0].data, &mut RefFrameState::new()).unwrap();
        assert_eq!(a.tile_data, b.tile_data);
        assert_eq!(a.order_hint, b.order_hint);
        assert_eq!(a.tile_groups, b.tile_groups);
        assert_eq!(a.refresh_frame_flags, b.refresh_frame_flags);
    }

    #[test]
    fn every_packet_of_the_clip_matches_an_independent_header_parse_with_threaded_state() {
        let packets = packets();
        let seq = crate::parse_sequence_header(packets[0].seq.as_ref().unwrap()).unwrap();
        let mut state = RefFrameState::new();
        let mut oracle = RefFrameState::new();
        let mut multi_frame_packets = 0;
        for (i, packet) in packets.iter().enumerate() {
            // Only the first packet carries the sequence header; the callers prepend it.
            let mut data = obu(ObuType::SequenceHeader, packets[0].seq.as_ref().unwrap());
            data.extend_from_slice(&packet.data);
            let parsed = ParsedFrame::parse_with_ref_state(&data, &mut state).unwrap();

            // The packet's last frame is the one `parsed` describes.
            let mut last = None;
            for (obu_type, payload) in &packet.frames {
                let before = *oracle.ref_order_hint();
                let header = parse_frame_header_full(payload, &seq, &mut oracle).unwrap();
                last = Some((*obu_type, payload, header, before));
            }
            let (obu_type, payload, header, before) = last.unwrap();
            multi_frame_packets += (packet.frames.len() > 1) as usize;

            assert_eq!(parsed.order_hint, header.order_hint, "packet {i}");
            assert_eq!(parsed.ref_frame_idx, header.ref_frame_idx, "packet {i}");
            assert_eq!(
                parsed.show_existing_slot, header.frame_to_show_map_idx,
                "packet {i}"
            );
            assert_eq!(
                parsed.key_frame,
                !header.show_existing_frame
                    && header.frame_type.is_intra()
                    && !header.frame_type.is_intra_only(),
                "packet {i}"
            );
            let tile: &[u8] =
                if obu_type == ObuType::Frame && header.header_size_bytes < payload.len() {
                    &payload[header.header_size_bytes..]
                } else {
                    &[]
                };
            assert_eq!(&parsed.tile_data[..], tile, "packet {i} tile data");
            assert_eq!(
                state.ref_order_hint(),
                oracle.ref_order_hint(),
                "packet {i} state"
            );

            // Reference-derived values are defined against the slots as they were before the
            // frame's own header. For the later frames of a multi-frame packet the parser uses the
            // slots from the start of the packet instead (#233), so only single-frame packets are
            // checked here.
            if packet.frames.len() == 1 {
                assert_eq!(
                    parsed.ref_frame_sign_bias,
                    crate::frame_header_full::ref_frame_sign_bias(
                        &before,
                        header.ref_frame_idx.as_ref(),
                        header.order_hint,
                        &seq
                    ),
                    "packet {i} sign bias"
                );
                assert_eq!(
                    parsed.ref_order_distance,
                    crate::frame_header_full::ref_order_distance(
                        &before,
                        header.ref_frame_idx.as_ref(),
                        header.order_hint,
                        &seq
                    ),
                    "packet {i} order distance"
                );
            }
        }
        assert!(
            multi_frame_packets > 0,
            "the clip must hold a multi-frame packet"
        );
    }

    #[test]
    fn the_reference_state_is_updated_by_a_key_frame() {
        let packets = packets();
        let mut state = RefFrameState::new();
        let before = *state.ref_order_hint();
        ParsedFrame::parse_with_ref_state(&packets[0].data, &mut state).unwrap();
        let (_, full) = key_header(&packets);
        let flags = full.refresh_frame_flags.unwrap();
        assert_ne!(flags, 0);
        for (slot, (&now, &was)) in state.ref_order_hint().iter().zip(&before).enumerate() {
            if flags & (1 << slot) != 0 {
                assert_eq!(now, full.order_hint, "slot {slot}");
            } else {
                assert_eq!(now, was, "slot {slot}");
            }
        }
    }

    #[test]
    fn without_a_sequence_header_the_tile_bytes_come_from_the_approximate_header() {
        let packets = packets();
        let mut data = td();
        data.extend(obu(ObuType::Frame, &packets[0].frame));
        let basic = parse_frame_header_basic(&packets[0].frame).unwrap();
        let frame = ParsedFrame::parse(&data).unwrap();

        assert_eq!(
            types(&frame),
            vec![ObuType::TemporalDelimiter, ObuType::Frame]
        );
        assert_eq!(
            &frame.tile_data[..],
            &packets[0].frame[basic.header_size_bytes..]
        );
        // No exact header, so no tile group range and none of the exact-header values.
        assert!(frame.tile_groups.is_empty());
        assert!(!frame.key_frame);
        assert!(frame.frame_type.is_intra_only);
        assert_eq!(frame.frame_type.base_qp, basic.base_q_idx);
        // Dimensions stay the default scaffold; the grid is recomputed from them.
        assert_eq!(
            (
                frame.dimensions.width,
                frame.dimensions.sb_cols,
                frame.dimensions.sb_rows
            ),
            (1920, 30, 17)
        );
    }

    #[test]
    fn a_sequence_header_that_does_not_parse_is_ignored_like_a_missing_one() {
        let packets = packets();
        let truncated = &packets[0].seq.as_ref().unwrap()[..3];
        assert!(crate::parse_sequence_header(truncated).is_err());
        let mut data = td();
        data.extend(obu(ObuType::SequenceHeader, truncated));
        data.extend(obu(ObuType::Frame, &packets[0].frame));
        let frame = ParsedFrame::parse(&data).unwrap();
        assert_eq!(frame.dimensions.width, 1920);
        assert!(frame.tile_groups.is_empty());
        assert!(!frame.key_frame);
        assert!(!frame.tile_data.is_empty());
    }

    #[test]
    fn a_frame_header_obu_followed_by_tile_group_obus_matches_the_combined_frame_obu() {
        let packets = packets();
        let (_, full) = key_header(&packets);
        let (header, tiles) = packets[0].frame.split_at(full.header_size_bytes);
        let combined = ParsedFrame::parse(&packets[0].data).unwrap();

        let mut data = td();
        data.extend(obu(
            ObuType::SequenceHeader,
            packets[0].seq.as_ref().unwrap(),
        ));
        data.extend(obu(ObuType::FrameHeader, header));
        data.extend(obu(ObuType::TileGroup, tiles));
        let split = ParsedFrame::parse(&data).unwrap();

        assert_eq!(
            types(&split),
            vec![
                ObuType::TemporalDelimiter,
                ObuType::SequenceHeader,
                ObuType::FrameHeader,
                ObuType::TileGroup
            ]
        );
        assert_eq!(split.tile_data, combined.tile_data);
        assert_eq!(split.tile_groups, combined.tile_groups);
        assert_eq!(split.key_frame, combined.key_frame);
        assert_eq!(split.frame_type.base_qp, combined.frame_type.base_qp);
        assert_eq!(split.refresh_frame_flags, combined.refresh_frame_flags);
    }

    #[test]
    fn several_tile_group_obus_are_concatenated_with_one_range_each() {
        let packets = packets();
        let (_, full) = key_header(&packets);
        let (header, tiles) = packets[0].frame.split_at(full.header_size_bytes);
        let cut = tiles.len() / 3;
        let (a, rest) = tiles.split_at(cut);
        let (b, c) = rest.split_at(rest.len() / 2);

        let mut data = obu(ObuType::SequenceHeader, packets[0].seq.as_ref().unwrap());
        data.extend(obu(ObuType::FrameHeader, header));
        for part in [a, b, c] {
            data.extend(obu(ObuType::TileGroup, part));
        }
        let frame = ParsedFrame::parse(&data).unwrap();

        assert_eq!(&frame.tile_data[..], tiles);
        assert_eq!(
            frame.tile_groups,
            vec![
                0..a.len(),
                a.len()..a.len() + b.len(),
                a.len() + b.len()..tiles.len()
            ]
        );
    }

    #[test]
    fn a_second_frame_in_the_packet_replaces_the_first_ones_tile_data() {
        let packets = packets();
        let (_, full) = key_header(&packets);
        let mut data = td();
        data.extend(obu(
            ObuType::SequenceHeader,
            packets[0].seq.as_ref().unwrap(),
        ));
        data.extend(obu(ObuType::Frame, &packets[0].frame));
        data.extend(obu(ObuType::Frame, &packets[0].frame));
        let frame = ParsedFrame::parse(&data).unwrap();

        let tile_len = packets[0].frame.len() - full.header_size_bytes;
        assert_eq!(frame.tile_data.len(), tile_len);
        assert_eq!(frame.tile_groups, vec![0..tile_len]);
    }

    #[test]
    fn a_frame_header_obu_discards_tile_groups_gathered_before_it() {
        let packets = packets();
        let (_, full) = key_header(&packets);
        let header = &packets[0].frame[..full.header_size_bytes];
        let mut data = obu(ObuType::SequenceHeader, packets[0].seq.as_ref().unwrap());
        data.extend(obu(ObuType::TileGroup, &[1, 2, 3, 4]));
        data.extend(obu(ObuType::FrameHeader, header));
        let frame = ParsedFrame::parse(&data).unwrap();
        assert!(frame.tile_data.is_empty());
        assert!(frame.tile_groups.is_empty());
        assert!(frame.key_frame, "the header itself is still applied");
    }

    #[test]
    fn a_frame_obu_holding_only_the_header_has_no_tile_data() {
        let packets = packets();
        let (_, full) = key_header(&packets);
        let mut data = obu(ObuType::SequenceHeader, packets[0].seq.as_ref().unwrap());
        data.extend(obu(
            ObuType::Frame,
            &packets[0].frame[..full.header_size_bytes],
        ));
        let frame = ParsedFrame::parse(&data).unwrap();
        assert!(frame.tile_data.is_empty() && frame.tile_groups.is_empty());
        assert!(frame.key_frame);
    }

    #[test]
    fn a_frame_obu_too_short_for_any_header_is_recorded_but_contributes_nothing() {
        let packets = packets();
        let mut data = obu(ObuType::SequenceHeader, packets[0].seq.as_ref().unwrap());
        data.extend(obu(ObuType::Frame, &[0x00]));
        let frame = ParsedFrame::parse(&data).unwrap();
        assert_eq!(types(&frame), vec![ObuType::SequenceHeader, ObuType::Frame]);
        assert!(frame.tile_data.is_empty() && frame.tile_groups.is_empty());
        assert!(!frame.key_frame);
    }

    #[test]
    fn obus_of_other_types_are_listed_but_change_nothing() {
        let packets = packets();
        let plain = ParsedFrame::parse(&packets[0].data).unwrap();
        let mut data = td();
        data.extend(obu(ObuType::Metadata, &[1, 2, 3]));
        data.extend(obu(
            ObuType::SequenceHeader,
            packets[0].seq.as_ref().unwrap(),
        ));
        data.extend(obu(ObuType::Padding, &[0; 5]));
        data.extend(obu(ObuType::Frame, &packets[0].frame));
        let frame = ParsedFrame::parse(&data).unwrap();
        assert_eq!(frame.obus.len(), 5);
        assert_eq!(frame.tile_data, plain.tile_data);
        assert_eq!(frame.tile_groups, plain.tile_groups);
        assert_eq!(frame.dimensions.sb_cols, plain.dimensions.sb_cols);
    }

    #[test]
    fn every_prefix_of_a_packet_parses_and_a_longer_prefix_never_has_fewer_obus() {
        let packets = packets();
        let data = &packets[0].data;
        let mut previous = 0;
        for len in 0..=data.len() {
            let frame = ParsedFrame::parse(&data[..len]).unwrap();
            assert!(frame.obus.len() >= previous, "prefix {len}");
            assert!(frame.obus.len() <= 3, "prefix {len}");
            previous = frame.obus.len();
        }
        assert_eq!(previous, 3);
    }

    #[test]
    fn corrupting_bytes_of_a_packet_never_panics() {
        let packets = packets();
        let base = &packets[0].data;
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        for _ in 0..1500 {
            let mut data = base.clone();
            for _ in 0..3 {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let pos = (state % data.len() as u64) as usize;
                data[pos] ^= (state >> 32) as u8 | 1;
            }
            let _ = ParsedFrame::parse(&data);
        }
        // Header bytes in particular: every byte of the first 40 set to each extreme.
        for pos in 0..40 {
            for value in [0x00u8, 0xff] {
                let mut data = base.clone();
                data[pos] = value;
                let _ = ParsedFrame::parse(&data);
            }
        }
    }

    #[test]
    fn apply_full_header_copies_the_loop_filter_delta_motion_and_superres_switches() {
        let (seq, payload) = first_frame();
        let full = parse_frame_header_full(&payload, &seq, &mut RefFrameState::new()).unwrap();
        for flip in [false, true] {
            let mut header = full.clone();
            header.delta_lf_present = flip;
            header.delta_lf_multi = !flip;
            header.super_resolution.enabled = flip;
            header.force_integer_mv = flip;
            header.gm_type = if flip {
                [1, 2, 3, 4, 5, 6, 7, 8]
            } else {
                [0; 8]
            };
            let mut frame = ParsedFrame::scaffold(Arc::from([0u8]));
            frame.apply_full_header(&header, &seq, &[0; 8]);
            assert_eq!(frame.delta_lf_present, flip);
            assert_eq!(frame.delta_lf_multi, !flip);
            assert_eq!(frame.superres, flip);
            assert_eq!(frame.force_integer_mv, flip);
            assert_eq!(frame.gm_type, header.gm_type);
        }
    }

    #[test]
    fn apply_full_header_takes_the_coded_size_and_the_superres_geometry_from_the_header() {
        let (seq, payload) = first_frame();
        let mut header =
            parse_frame_header_full(&payload, &seq, &mut RefFrameState::new()).unwrap();
        header.width = 313;
        header.height = 288;
        header.upscaled_width = 352;
        header.super_resolution.enabled = true;
        header.super_resolution.scale_denominator = 9;
        header.ref_scaled = [false, true, true, true, false, false, false, false];
        let mut frame = ParsedFrame::scaffold(Arc::from([0u8]));
        frame.apply_sequence_header(&seq);
        frame.apply_full_header(&header, &seq, &[0; 8]);
        frame.finish_superblock_grid();
        assert_eq!(
            (frame.dimensions.width, frame.dimensions.height),
            (313, 288)
        );
        assert_eq!(
            frame.dimensions.sb_cols,
            313u32.div_ceil(frame.dimensions.sb_size)
        );
        assert_eq!((frame.superres_denom, frame.upscaled_width), (9, 352));
        assert_eq!(frame.ref_scaled, header.ref_scaled);
        // The loop-restoration grid is the upscaled one.
        let params = frame.coding_params().restoration;
        assert_eq!((params.frame_width, params.superres_denom), (352, 9));

        header.super_resolution.enabled = false;
        header.width = 352;
        frame.apply_full_header(&header, &seq, &[0; 8]);
        assert_eq!(frame.superres_denom, 8);
    }

    #[test]
    fn a_zero_base_index_without_coded_dc_deltas_is_lossless() {
        let (seq, payload) = first_frame();
        let mut header =
            parse_frame_header_full(&payload, &seq, &mut RefFrameState::new()).unwrap();
        header.base_q_idx = Some(0);
        header.y_dc_delta_q = None;
        header.uv_dc_delta_q = None;
        let mut frame = ParsedFrame::scaffold(Arc::from([0u8]));
        frame.apply_full_header(&header, &seq, &[0; 8]);
        assert!(frame.coded_lossless, "an absent delta counts as 0");
        // A missing base index is not lossless.
        header.base_q_idx = None;
        frame.apply_full_header(&header, &seq, &[0; 8]);
        assert!(!frame.coded_lossless);
    }

    #[test]
    fn every_one_of_the_five_delta_q_terms_breaks_lossless() {
        // #245: only the Y and U DC terms used to be looked at
        let (seq, payload) = first_frame();
        let mut header =
            parse_frame_header_full(&payload, &seq, &mut RefFrameState::new()).unwrap();
        header.base_q_idx = Some(0);
        let lossless = |h: &crate::frame_header::FrameHeader| {
            let mut frame = ParsedFrame::scaffold(Arc::from([0u8]));
            frame.apply_full_header(h, &seq, &[0; 8]);
            frame.coded_lossless
        };
        header.y_dc_delta_q = Some(0);
        header.uv_dc_delta_q = Some(0);
        header.uv_ac_delta_q = Some(0);
        header.v_dc_delta_q = Some(0);
        header.v_ac_delta_q = Some(0);
        assert!(lossless(&header));
        type Set = fn(&mut crate::frame_header::FrameHeader, Option<i8>);
        let setters: [(&str, Set); 5] = [
            ("y_dc", |h, v| h.y_dc_delta_q = v),
            ("u_dc", |h, v| h.uv_dc_delta_q = v),
            ("u_ac", |h, v| h.uv_ac_delta_q = v),
            ("v_dc", |h, v| h.v_dc_delta_q = v),
            ("v_ac", |h, v| h.v_ac_delta_q = v),
        ];
        for (name, set) in setters {
            for nonzero in [1, -1] {
                let mut h = header.clone();
                set(&mut h, Some(nonzero));
                assert!(!lossless(&h), "{name} = {nonzero}");
            }
            let mut h = header.clone();
            set(&mut h, None);
            assert!(lossless(&h), "{name} absent counts as 0");
        }
    }

    #[test]
    fn the_scaffold_holds_the_spec_fallbacks_for_values_no_header_supplied() {
        let frame = ParsedFrame::parse(&[]).unwrap();
        assert!(frame.disable_frame_end_update_cdf);
        assert!(frame.mono_chrome);
        assert!(!frame.subsampling_x && !frame.subsampling_y);
        assert!(!frame.delta_q_enabled && !frame.reference_select && !frame.allow_intrabc);
        assert_eq!(frame.refresh_frame_flags, 0);
        assert_eq!(frame.ref_frame_idx, None);
        assert_eq!(frame.ref_frame_sign_bias, [false; 7]);
        assert_eq!(frame.gm_type, [0u8; 8]);
        assert!(frame.tiles.is_none() && frame.decoded.is_none());
    }

    #[test]
    fn without_a_sequence_header_a_frame_obu_holding_only_the_header_has_no_tile_data() {
        let packets = packets();
        let basic = parse_frame_header_basic(&packets[0].frame).unwrap();
        let mut data = td();
        data.extend(obu(
            ObuType::Frame,
            &packets[0].frame[..basic.header_size_bytes],
        ));
        let frame = ParsedFrame::parse(&data).unwrap();
        assert!(frame.tile_data.is_empty() && frame.tile_groups.is_empty());
        assert!(
            frame.frame_type.is_intra_only,
            "the approximate header was still applied"
        );
        assert_eq!(frame.frame_type.base_qp, basic.base_q_idx);
        // One byte more and there is a tile byte.
        let mut data = td();
        data.extend(obu(
            ObuType::Frame,
            &packets[0].frame[..basic.header_size_bytes + 1],
        ));
        let frame = ParsedFrame::parse(&data).unwrap();
        assert_eq!(
            &frame.tile_data[..],
            &[packets[0].frame[basic.header_size_bytes]]
        );
    }

    #[test]
    fn a_frame_header_obu_without_a_sequence_header_applies_only_the_approximate_header() {
        let packets = packets();
        let (_, full) = key_header(&packets);
        let basic = parse_frame_header_basic(&packets[0].frame).unwrap();
        let header = &packets[0].frame[..full.header_size_bytes];
        let frame = ParsedFrame::parse(&obu(ObuType::FrameHeader, header)).unwrap();
        assert!(frame.frame_type.is_intra_only);
        assert_eq!(frame.frame_type.base_qp, basic.base_q_idx);
        assert!(
            !frame.key_frame,
            "key_frame comes from the exact header only"
        );
        assert_eq!(
            frame.dimensions.width, 1920,
            "no sequence header: scaffold size"
        );
        assert!(frame.tile_data.is_empty());
    }

    #[test]
    fn a_frame_header_obu_too_short_to_parse_changes_nothing() {
        let packets = packets();
        let mut data = obu(ObuType::SequenceHeader, packets[0].seq.as_ref().unwrap());
        data.extend(obu(ObuType::FrameHeader, &[]));
        let frame = ParsedFrame::parse(&data).unwrap();
        assert_eq!(
            types(&frame),
            vec![ObuType::SequenceHeader, ObuType::FrameHeader]
        );
        assert!(!frame.key_frame && !frame.frame_type.is_intra_only);
        assert_eq!(frame.frame_type.base_qp, None);
        assert!(frame.tile_data.is_empty());
        // Without a sequence header either.
        let frame = ParsedFrame::parse(&obu(ObuType::FrameHeader, &[])).unwrap();
        assert_eq!(types(&frame), vec![ObuType::FrameHeader]);
        assert!(!frame.frame_type.is_intra_only);
    }

    #[test]
    fn a_zero_base_index_with_a_nonzero_luma_dc_delta_is_not_lossless() {
        let (seq, payload) = first_frame();
        let mut header =
            parse_frame_header_full(&payload, &seq, &mut RefFrameState::new()).unwrap();
        header.base_q_idx = Some(0);
        header.uv_dc_delta_q = Some(0);
        for (y_dc, lossless) in [
            (Some(0), true),
            (None, true),
            (Some(1), false),
            (Some(-1), false),
        ] {
            header.y_dc_delta_q = y_dc;
            let mut frame = ParsedFrame::scaffold(Arc::from([0u8]));
            frame.apply_full_header(&header, &seq, &[0; 8]);
            assert_eq!(frame.coded_lossless, lossless, "y_dc {y_dc:?}");
        }
    }

    /// Every prefix of a real Frame OBU payload, with no sequence header: the tile bytes are
    /// whatever follows the approximate header, and nothing when the payload ends inside it.
    #[test]
    fn without_a_sequence_header_every_prefix_of_a_frame_payload_gives_the_bytes_after_the_header()
    {
        let packets = packets();
        let payload = &packets[0].frame;
        let mut with_tiles = 0;
        for cut in 0..=payload.len().min(60) {
            let frame = ParsedFrame::parse(&obu(ObuType::Frame, &payload[..cut])).unwrap();
            let expected: &[u8] = match parse_frame_header_basic(&payload[..cut]) {
                Ok(h) if h.header_size_bytes < cut => &payload[h.header_size_bytes..cut],
                _ => &[],
            };
            assert_eq!(&frame.tile_data[..], expected, "prefix of {cut} bytes");
            with_tiles += usize::from(!expected.is_empty());
        }
        assert!(with_tiles > 0, "some prefix reaches into the tile");
    }
}
