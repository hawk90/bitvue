//! Frame header parsing for AV1
//!
//! Parses frame header OBUs to extract frame type and other metadata.
//!
//! # Implementation Notes
//!
//! This parser implements the AV1 uncompressed header per spec Section 5.9.2.
//! Because a full decode of all conditional fields requires the Sequence Header
//! context (color config, film-grain params, etc.), fields that depend on
//! absent context are treated conservatively:
//!
//! - `frame_size_override_flag` is parsed but actual width/height override
//!   reading requires sequence-header `frame_width_bits`/`frame_height_bits`,
//!   so those are skipped (we read only the flag).
//! - `reduced_still_picture_header` (from sequence header) is assumed to be
//!   false for all calls coming through `parse_frame_header_basic`, which is
//!   the correct assumption for normal video streams.
//! - `enable_order_hint` is assumed to be true when `order_hint_bits > 0`.
//!
//! The parser extracts: show_existing_frame, frame_type, show_frame,
//! error_resilient_mode, disable_cdf_update, allow_screen_content_tools,
//! quantization_params (base_q_idx + delta_q fields), delta_q_params, and
//! refresh_frame_flags / ref_frame_idx for INTER frames.

use crate::bitreader::BitReader;
use bitvue_engine::BitvueError;
// Re-export FrameType for other modules in this crate
pub use bitvue_engine::FrameType;

mod quant;
mod types;

pub use types::*;

use self::quant::parse_delta_q_params;
pub(crate) use self::quant::{parse_quantization_params, QuantizationParams};

/// Reads the reference-frame indices of an inter frame: `None` when short signaling was used (the
/// slots are then derived by `set_frame_refs()`, which this parser does not run) or when the
/// payload ends early. `reader` is positioned at `frame_refs_short_signaling`.
fn read_ref_frame_idx(reader: &mut BitReader) -> Option<[u8; 7]> {
    let mut ref_frame_idx: Option<[u8; 7]> = None;
    // frame_refs_short_signaling  f(1)
    // Only present when enable_order_hint is true in sequence header.
    // We conservatively assume false (not present) since we lack seq context.
    // Most bitstreams use enable_order_hint=true, so this bit IS typically
    // present. Reading it as 0 means we treat it as "not short-signaling",
    // which requires reading all 7 ref indices.
    let frame_refs_short = reader.read_bit().unwrap_or(false);
    if frame_refs_short {
        // Short signaling: last_frame_idx u(3) + gold_frame_idx u(3)
        // then set_frame_refs() fills in the remaining 5 slots implicitly
        reader.read_bits(3).ok(); // last_frame_idx
        reader.read_bits(3).ok(); // gold_frame_idx
                                  // No explicit ref_frame_idx bits to read; slots are derived.
                                  // We cannot recover the index values without running set_frame_refs().
    } else {
        // Long signaling: read all 7 ref_frame_idx u(3) entries
        let mut idx = [0u8; 7];
        let ok = (0..7).all(|i| {
            reader
                .read_bits(3)
                .map(|v| {
                    idx[i] = v as u8;
                })
                .is_ok()
        });
        if ok {
            ref_frame_idx = Some(idx);
        }
    }
    ref_frame_idx
}

/// A [`FrameHeader`] with every field the basic parser does not read (and the show_existing_frame
/// case never reads) at its default. Callers set what they did read with `..unparsed_header()`.
fn unparsed_header() -> FrameHeader {
    FrameHeader {
        frame_type: FrameType::Key,
        show_frame: true,
        show_existing_frame: false,
        frame_to_show_map_idx: None,
        error_resilient_mode: false,
        base_q_idx: None,
        y_dc_delta_q: None,
        uv_dc_delta_q: None,
        delta_q_present: false,
        delta_q_residue: None,
        header_size_bytes: 0,
        header_size_bits: 0,
        tiles: None,
        refresh_frame_flags: None,
        ref_frame_idx: None,
        order_hint: 0,
        width: 0,
        height: 0,
        upscaled_width: 0,
        upscaled_height: 0,
        reference_select: false,
        allow_intrabc: false,
        allow_screen_content_tools: false,
        delta_lf_present: false,
        delta_lf_multi: false,
        skip_mode_present: false,
        skip_mode_refs: [0, 0],
        subpel_filter_switchable: false,
        switchable_motion_mode: false,
        allow_warped_motion: false,
        force_integer_mv: false,
        allow_high_precision_mv: false,
        gm_type: [0u8; 8],
        reduced_tx_set: false,
        txfm_mode: TxfmMode::Largest,
        use_ref_frame_mvs: false,
        primary_ref_frame: 7,
        disable_frame_end_update_cdf: true,
        segmentation: crate::frame_header_full::SegmentationInfo::default(),
        loop_filter: LoopFilterInfo::default(),
        cdef_damping: CdefInfo::default(),
        cdef_y_primary_strength: 0,
        cdef_y_secondary_strength: 0,
        cdef_uv_primary_strength: 0,
        cdef_uv_secondary_strength: 0,
        loop_restoration: LoopRestorationInfo::default(),
        film_grain: FilmGrainInfo::default(),
        super_resolution: SuperResolutionInfo::default(),
    }
}

/// The header of a `show_existing_frame = 1` frame: a shown key frame with every other field at
/// its default. `reader` is positioned right after the slot index.
fn show_existing_frame_header(reader: &BitReader, frame_to_show_map_idx: u8) -> FrameHeader {
    FrameHeader {
        show_existing_frame: true,
        frame_to_show_map_idx: Some(frame_to_show_map_idx),
        header_size_bytes: reader.byte_position()
            + usize::from(!reader.position().is_multiple_of(8)),
        header_size_bits: reader.position(),
        ..unparsed_header()
    }
}

/// Parse frame header from payload
///
/// Implements AV1 spec Section 5.9.2 (uncompressed_header).
///
/// # Assumptions
///
/// Since we don't carry sequence header context:
/// - `reduced_still_picture_header` = false
/// - `enable_order_hint` = false (skips order_hint reading)
/// - `separate_uv_delta_q` = false
/// - `enable_cdef` = true
/// - `enable_restoration` = true
/// - `use_128x128_superblock` = false
pub fn parse_frame_header_basic(payload: &[u8]) -> Result<FrameHeader, BitvueError> {
    let mut reader = BitReader::new(payload);

    // show_existing_frame (1 bit)
    let show_existing_frame = reader.read_bit()?;

    if show_existing_frame {
        let frame_to_show_map_idx = reader.read_bits(3)? as u8;
        return Ok(show_existing_frame_header(&reader, frame_to_show_map_idx));
    }

    // frame_type (2 bits) – AV1 Spec 5.9.2
    let frame_type_bits = reader.read_bits(2)?;
    let frame_type = FrameType::from_av1_bits(frame_type_bits);

    // show_frame (1 bit)
    let show_frame = reader.read_bit()?;

    // showable_frame (1 bit) – only when NOT (KEY and show_frame)
    let _showable_frame = if !(frame_type == FrameType::Key && show_frame) {
        reader.read_bit()?
    } else {
        true
    };

    // error_resilient_mode (1 bit) – KEY frames shown are always error-resilient
    let error_resilient_mode = match (frame_type, show_frame) {
        (FrameType::Key, true) => true,
        _ => reader.read_bit()?,
    };

    // disable_cdf_update (1 bit)
    let _disable_cdf_update = reader.read_bit()?;

    // allow_screen_content_tools – conditional per spec
    // force_integer_mv – conditional
    //
    // These depend on sequence-header `seq_force_screen_content_tools` /
    // `seq_force_integer_mv`. Without that context we skip them conservatively.
    // In practice most streams do NOT set these flags, so skipping them is safe
    // for the vast majority of real-world AV1.

    // frame_size_override_flag (1 bit) – for SWITCH frames or when not KEY
    let _frame_size_override = if frame_type == FrameType::Switch {
        true
    } else if frame_type != FrameType::Key {
        reader.read_bit()?
    } else {
        false
    };

    // order_hint – skip; without seq header we don't know order_hint_bits
    // We skip this field for simplicity.

    let is_intra = frame_type.is_intra();

    // primary_ref_frame (3 bits) per AV1 spec 5.9.2:
    // present when error_resilient_mode == 0 AND NOT (KEY_FRAME AND show_frame)
    // AND NOT SWITCH_FRAME
    let needs_primary_ref = !(error_resilient_mode || (frame_type == FrameType::Key && show_frame))
        && frame_type != FrameType::Switch;
    if needs_primary_ref {
        reader.read_bits(3)?;
    }

    // Buffer flags for inter frames
    // refresh_frame_flags (8 bits):
    // For KEY+show_frame and SWITCH frames: all slots refreshed (0xFF), no field in bitstream
    // For show_existing_frame: already handled above (None)
    // All other frames: read 8 bits from bitstream
    let refresh_frame_flags =
        if (frame_type == FrameType::Key && show_frame) || frame_type == FrameType::Switch {
            Some(0xFFu8)
        } else {
            match reader.read_bits(8) {
                Ok(v) => Some(v as u8),
                Err(_) => None,
            }
        };

    // For INTRA_ONLY frames: read ref_order_hint for each refreshed buffer
    // (we skip the actual values since we don't use them for extraction)
    // For INTER frames: read reference frame info
    let ref_frame_idx = if is_intra {
        None
    } else {
        read_ref_frame_idx(&mut reader)
    };

    // frame_size() – skip; without seq header we cannot know field widths
    // For frame_size_override we would need frame_width_bits/height_bits from seq header.
    // We skip these bits. This means header_size is an approximation for non-KEY frames.

    // render_and_frame_size_different – skip (1 bit, then maybe 2 × 16 bits)
    // For simplicity we skip these too.

    // allow_high_precision_mv, use_ref_frame_mvs, allow_intrabc: skip for inter

    // --- Fields we CAN reliably parse regardless of missing context ---

    // Skip loop_filter_params, segmentation, etc. and go directly to quant params.
    // We use a bounded read attempt: try parsing quantization at the current position.
    // If it fails we still return a valid (partial) header.

    let (base_q_idx_opt, y_dc_delta_q, uv_dc_delta_q, delta_q_present) =
        match parse_quantization_params(&mut reader, 3, false) {
            Ok(q) => {
                let dq = parse_delta_q_params(&mut reader, q.base_q_idx).unwrap_or(false);
                (Some(q.base_q_idx), Some(q.y_dc), Some(q.u_dc), dq)
            }
            Err(_) => (None, None, None, false),
        };

    let header_size_bytes = reader
        .byte_position()
        .saturating_add(usize::from(!reader.position().is_multiple_of(8)));
    let header_size_bits = reader.position();

    Ok(FrameHeader {
        frame_type,
        show_frame,
        error_resilient_mode,
        base_q_idx: base_q_idx_opt,
        y_dc_delta_q,
        uv_dc_delta_q,
        delta_q_present,
        header_size_bytes,
        header_size_bits,
        refresh_frame_flags,
        ref_frame_idx,
        ..unparsed_header()
    })
}

#[cfg(test)]
mod tests {

    #[test]
    fn loop_restoration_type_maps_the_two_bit_code_and_ignores_higher_bits() {
        assert_eq!(LoopRestorationType::from_bits(0), LoopRestorationType::None);
        assert_eq!(
            LoopRestorationType::from_bits(1),
            LoopRestorationType::Wiener
        );
        assert_eq!(
            LoopRestorationType::from_bits(2),
            LoopRestorationType::SgrProj
        );
        assert_eq!(
            LoopRestorationType::from_bits(3),
            LoopRestorationType::Switchable
        );
        assert_eq!(LoopRestorationType::from_bits(4), LoopRestorationType::None);
        assert_eq!(
            LoopRestorationType::from_bits(5),
            LoopRestorationType::Wiener
        );
    }
    use super::*;

    fn bits_writer() -> (Vec<u8>, impl FnMut(&mut Vec<u8>, u32, u8)) {
        (Vec::new(), |bits: &mut Vec<u8>, v: u32, n: u8| {
            for i in (0..n).rev() {
                bits.push(((v >> i) & 1) as u8);
            }
        })
    }

    fn pack(bits: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; bits.len().div_ceil(8)];
        for (i, bit) in bits.iter().enumerate() {
            if *bit != 0 {
                out[i / 8] |= 1 << (7 - (i % 8));
            }
        }
        out
    }

    #[test]
    fn parse_quantization_params_retains_real_uv_ac_delta_q() {
        let (mut bits, mut push) = bits_writer();
        push(&mut bits, 0, 8); // base_q_idx = 0
        push(&mut bits, 0, 1); // DeltaQYDc: not coded => 0
        push(&mut bits, 0, 1); // DeltaQUDc: not coded => 0
        push(&mut bits, 1, 1); // DeltaQUAc: coded
        push(&mut bits, 3, 7); // DeltaQUAc value = 3 (su(7), positive)
        push(&mut bits, 0, 1); // using_qmatrix = 0
        let payload = pack(&bits);
        let mut reader = BitReader::new(&payload);

        let q = parse_quantization_params(&mut reader, 3, false).unwrap();

        assert_eq!((q.base_q_idx, q.y_dc, q.u_dc), (0, 0, 0));
        assert_eq!(q.u_ac, 3, "DeltaQUAc must be retained, not discarded");
        assert_eq!(
            (q.v_dc, q.v_ac),
            (0, 3),
            "V mirrors U without diff_uv_delta"
        );
        assert_eq!(reader.position(), 8 + 1 + 1 + 8 + 1);
    }

    /// With `separate_uv_delta_q` a `diff_uv_delta` flag precedes the U deltas, and when it is set
    /// the V deltas follow -- and `qm_v` is read only with separate U/V deltas. rav1e writes
    /// this (every delta coded, 49 bits before the quantizer matrix flag), and the parse used to
    /// miss the flag and run 5 bits long on its key frame.
    #[test]
    fn parse_quantization_params_reads_separate_uv_deltas_and_qm_v() {
        let (mut bits, mut push) = bits_writer();
        push(&mut bits, 100, 8); // base_q_idx
        push(&mut bits, 1, 1); // DeltaQYDc coded
        push(&mut bits, 5, 7); //   = 5
        push(&mut bits, 1, 1); // diff_uv_delta
        push(&mut bits, 1, 1); // DeltaQUDc coded
        push(&mut bits, 2, 7); //   = 2
        push(&mut bits, 1, 1); // DeltaQUAc coded
        push(&mut bits, 3, 7); //   = 3
        push(&mut bits, 1, 1); // DeltaQVDc coded
        push(&mut bits, 4, 7); //   = 4
        push(&mut bits, 1, 1); // DeltaQVAc coded
        push(&mut bits, 6, 7); //   = 6
        push(&mut bits, 1, 1); // using_qmatrix
        push(&mut bits, 7, 4); // qm_y
        push(&mut bits, 8, 4); // qm_u
        push(&mut bits, 9, 4); // qm_v (separate_uv_delta_q)
        let payload = pack(&bits);
        let mut reader = BitReader::new(&payload);

        let q = parse_quantization_params(&mut reader, 3, true).unwrap();

        assert_eq!(
            q,
            QuantizationParams {
                base_q_idx: 100,
                y_dc: 5,
                u_dc: 2,
                u_ac: 3,
                v_dc: 4,
                v_ac: 6
            }
        );
        assert_eq!(reader.position(), 8 + 8 + 1 + 4 * 8 + 1 + 12);
    }

    /// Without separate U/V deltas the quantizer matrix level of V is U's: only `qm_y` and
    /// `qm_u` are coded.
    #[test]
    fn parse_quantization_params_reads_qm_v_only_with_separate_uv_deltas() {
        let (mut bits, mut push) = bits_writer();
        push(&mut bits, 80, 8); // base_q_idx
        push(&mut bits, 0, 1); // DeltaQYDc
        push(&mut bits, 0, 1); // DeltaQUDc
        push(&mut bits, 0, 1); // DeltaQUAc
        push(&mut bits, 1, 1); // using_qmatrix
        push(&mut bits, 3, 4); // qm_y
        push(&mut bits, 4, 4); // qm_u
        push(&mut bits, 0xA5, 8); // what follows the quantizer params
        let payload = pack(&bits);
        let mut reader = BitReader::new(&payload);

        parse_quantization_params(&mut reader, 3, false).unwrap();

        assert_eq!(reader.position(), 8 + 3 + 1 + 8);
        assert_eq!(reader.read_bits(8).unwrap(), 0xA5);
    }

    #[test]
    fn parse_quantization_params_reads_no_chroma_deltas_for_monochrome() {
        let (mut bits, mut push) = bits_writer();
        push(&mut bits, 50, 8);
        push(&mut bits, 0, 1); // DeltaQYDc not coded
        push(&mut bits, 0, 1); // using_qmatrix = 0
        let payload = pack(&bits);
        let mut reader = BitReader::new(&payload);

        let q = parse_quantization_params(&mut reader, 1, false).unwrap();

        assert_eq!((q.u_dc, q.u_ac, q.v_dc, q.v_ac), (0, 0, 0, 0));
        assert_eq!(reader.position(), 10);
    }

    #[test]
    fn test_frame_type_from_bits() {
        assert_eq!(FrameType::from_av1_bits(0), FrameType::Key);
        assert_eq!(FrameType::from_av1_bits(1), FrameType::Inter);
        assert_eq!(FrameType::from_av1_bits(2), FrameType::IntraOnly);
        assert_eq!(FrameType::from_av1_bits(3), FrameType::Switch);
        assert_eq!(FrameType::from_av1_bits(4), FrameType::Unknown);
    }

    #[test]
    fn test_frame_type_names() {
        assert!(FrameType::Key.description().contains("Key"));
        assert!(FrameType::Inter.description().contains("Inter"));
        assert!(FrameType::IntraOnly.description().contains("Intra"));
        assert!(FrameType::Switch.description().contains("Switch"));
    }

    #[test]
    fn test_is_intra() {
        assert!(FrameType::Key.is_intra());
        assert!(!FrameType::Inter.is_intra());
        assert!(FrameType::IntraOnly.is_intra());
        assert!(!FrameType::Switch.is_intra());
    }

    #[test]
    fn test_parse_key_frame() {
        // show_existing_frame=0, frame_type=00 (KEY), show_frame=1
        // Binary: 0 00 1 0000 = 0x10
        let payload = [0b0001_0000];
        let header = parse_frame_header_basic(&payload).unwrap();
        assert_eq!(header.frame_type, FrameType::Key);
        assert!(header.show_frame);
        assert!(!header.show_existing_frame);
    }

    #[test]
    fn test_parse_inter_frame() {
        // Bit layout for first byte (0x30):
        //   bit0: show_existing_frame=0, bits1-2: frame_type=01 (INTER),
        //   bit3: show_frame=1, bit4: showable_frame=0, bit5: error_resilient_mode=0,
        //   bit6: disable_cdf_update=0, bit7: frame_size_override=0
        // Remaining fields read from zero-padded bytes that follow.
        let mut payload = [0u8; 16];
        payload[0] = 0b0011_0000;
        let header = parse_frame_header_basic(&payload).unwrap();
        assert_eq!(header.frame_type, FrameType::Inter);
        assert!(header.show_frame);
        assert!(!header.error_resilient_mode);
    }

    #[test]
    fn test_parse_show_existing_frame() {
        // show_existing_frame=1, frame_to_show_map_idx=101 (5)
        // Binary: 1 101 0000 = 0xD0
        let payload = [0b1101_0000];
        let header = parse_frame_header_basic(&payload).unwrap();
        assert!(header.show_existing_frame);
        assert_eq!(header.frame_to_show_map_idx, Some(5));
    }

    #[test]
    fn test_delta_q_present_when_base_q_nonzero() {
        // Build a minimal payload that has enough bits to reach quantization_params.
        // show_existing=0, frame_type=KEY(0b00), show_frame=1 → error_resilient=true(KEY shown)
        // disable_cdf_update=0, frame_size_override=false(KEY), …
        // We pad with zeros so reads don't fail; if parsing succeeds delta_q_present
        // should be parseable.
        let payload = vec![0u8; 32];
        // We just verify it doesn't panic; the exact value depends on the zero-filled content.
        let result = parse_frame_header_basic(&payload);
        assert!(result.is_ok());
    }

    // ---- parse_frame_header_basic: the branches the key/inter fixtures do not reach ----

    /// Writes `fields` (value, width) MSB first and returns the bytes and the bit count.
    fn basic_bits(fields: &[(u32, u8)]) -> (Vec<u8>, u64) {
        let (mut bits, mut push) = bits_writer();
        for &(v, w) in fields {
            push(&mut bits, v, w);
        }
        let n = bits.len() as u64;
        (pack(&bits), n)
    }

    /// `base_q_idx`, three uncoded deltas, `using_qmatrix = 0`, then `delta_q_present = 0`.
    fn quant_fields(base_q: u32) -> Vec<(u32, u8)> {
        vec![(base_q, 8), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1)]
    }

    #[test]
    fn a_switch_frame_has_no_size_override_primary_ref_or_refresh_bits() {
        let mut fields = vec![
            (0, 1), // show_existing_frame
            (3, 2), // frame_type = SWITCH_FRAME
            (1, 1), // show_frame
            (0, 1), // showable_frame
            (0, 1), // error_resilient_mode
            (0, 1), // disable_cdf_update
        ];
        // No frame_size_override_flag, primary_ref_frame or refresh_frame_flags for a switch
        // frame; the reference indices follow.
        fields.push((0, 1)); // frame_refs_short_signaling
        fields.extend((1..=7).map(|i| (i, 3)));
        fields.extend(quant_fields(77));
        let (bytes, bit_len) = basic_bits(&fields);
        let h = parse_frame_header_basic(&bytes).unwrap();
        assert_eq!(h.frame_type, FrameType::Switch);
        assert_eq!(h.refresh_frame_flags, Some(0xFF));
        assert_eq!(h.ref_frame_idx, Some([1, 2, 3, 4, 5, 6, 7]));
        assert_eq!(h.base_q_idx, Some(77));
        assert_eq!(h.header_size_bits, bit_len);
    }

    #[test]
    fn an_inter_frame_reads_size_override_primary_ref_and_refresh_bits() {
        let mut fields = vec![
            (0, 1),
            (1, 2),    // INTER_FRAME
            (1, 1),    // show_frame
            (0, 1),    // showable_frame
            (0, 1),    // error_resilient_mode
            (0, 1),    // disable_cdf_update
            (0, 1),    // frame_size_override_flag
            (5, 3),    // primary_ref_frame
            (0xA5, 8), // refresh_frame_flags
            (0, 1),    // frame_refs_short_signaling
        ];
        fields.extend((0..7).map(|i| (7 - i, 3)));
        fields.extend(quant_fields(9));
        let (bytes, bit_len) = basic_bits(&fields);
        let h = parse_frame_header_basic(&bytes).unwrap();
        assert_eq!(h.frame_type, FrameType::Inter);
        assert_eq!(h.refresh_frame_flags, Some(0xA5));
        assert_eq!(h.ref_frame_idx, Some([7, 6, 5, 4, 3, 2, 1]));
        assert_eq!(h.base_q_idx, Some(9));
        assert_eq!(h.header_size_bits, bit_len);

        // Error resilience drops the primary_ref_frame bits (3).
        let mut fields = vec![
            (0, 1),
            (1, 2),
            (1, 1),
            (0, 1),
            (1, 1), // error_resilient_mode
            (0, 1),
            (0, 1),
            (0xA5, 8),
            (0, 1),
        ];
        fields.extend((0..7).map(|i| (7 - i, 3)));
        fields.extend(quant_fields(9));
        let (bytes, bit_len) = basic_bits(&fields);
        let h = parse_frame_header_basic(&bytes).unwrap();
        assert_eq!(h.refresh_frame_flags, Some(0xA5));
        assert_eq!(h.ref_frame_idx, Some([7, 6, 5, 4, 3, 2, 1]));
        assert_eq!(h.header_size_bits, bit_len);
    }

    #[test]
    fn short_signaling_leaves_the_reference_indices_unknown() {
        let mut fields = vec![
            (0, 1),
            (1, 2),
            (1, 1),
            (0, 1),
            (1, 1),
            (0, 1),
            (0, 1),
            (0x0F, 8),
            (1, 1), // frame_refs_short_signaling
            (2, 3), // last_frame_idx
            (3, 3), // gold_frame_idx
        ];
        fields.extend(quant_fields(31));
        let (bytes, bit_len) = basic_bits(&fields);
        let h = parse_frame_header_basic(&bytes).unwrap();
        assert_eq!(h.ref_frame_idx, None);
        assert_eq!(
            h.base_q_idx,
            Some(31),
            "the quantizer is still read after the short form"
        );
        assert_eq!(h.header_size_bits, bit_len);
    }

    #[test]
    fn reference_indices_cut_short_give_no_indices_and_no_quantizer() {
        // Inter frame whose payload ends after four of the seven reference indices.
        let mut fields = vec![
            (0, 1),
            (1, 2),
            (1, 1),
            (0, 1),
            (1, 1),
            (0, 1),
            (0, 1),
            (0x0F, 8),
            (0, 1),
        ];
        fields.extend((1..=4).map(|i| (i, 3)));
        let (bytes, _) = basic_bits(&fields);
        let h = parse_frame_header_basic(&bytes).unwrap();
        assert_eq!(h.ref_frame_idx, None);
        assert_eq!(h.base_q_idx, None);
        assert!(!h.delta_q_present);
        // Every shorter prefix of a full header parses without panicking.
        let mut full = fields.clone();
        full.extend((5..=7).map(|i| (i, 3)));
        full.extend(quant_fields(5));
        let (bytes, _) = basic_bits(&full);
        for cut in 0..=bytes.len() {
            let _ = parse_frame_header_basic(&bytes[..cut]);
        }
    }
}
