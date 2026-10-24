//! Decode state carried from one frame to the next (the reference frame slots), and the step that
//! decodes one frame with it.
//!
//! A frame of a real stream cannot be decoded alone: it starts from the CDFs its primary reference
//! frame saved, its temporal motion-vector candidates come from the motion fields of earlier
//! frames, and its header depends on the order hints of the reference slots. [`StreamDecodeState`]
//! holds those per slot and is advanced by [`StreamDecodeState::decode_next`], once per frame in
//! decode order. Cloning it is cheap (the per-slot data is shared), which is what makes
//! checkpoints affordable.

use super::cu_parser::{decode_frame, frame_within_decode_limit, FrameDecodeInputs};
use super::parser::ParsedFrame;
use super::{DecodeOutcome, ParsedCodingUnits};
use crate::sequence::SequenceHeader;
use crate::symbol::CdfContext;
use crate::tile::{CodingUnit, MotionFieldState, ProjectedMotionField};
use bitvue_engine::BitvueError;
use std::sync::Arc;

/// The decodable frames of one temporal unit, each as the bytes of its OBUs: a `Frame` OBU on its
/// own, or a `FrameHeader` OBU followed by its `TileGroup` OBUs. Redundant frame headers are
/// dropped. A `show_existing_frame` header holds no frame of its own and is left out (feed such a
/// unit to [`StreamDecodeState::skip_unit`] instead).
pub(crate) fn frame_units(packet: &[u8], reduced_still_picture_header: bool) -> Vec<Vec<u8>> {
    use crate::obu::{ObuIterator, ObuType};
    let mut units: Vec<Vec<u8>> = Vec::new();
    let mut open = false;
    let mut iter = ObuIterator::new(packet);
    while let Some(Ok(found)) = iter.next_obu_with_offset() {
        let raw = &packet[found.offset..found.offset + found.consumed];
        match found.obu.header.obu_type {
            ObuType::Frame => {
                units.push(raw.to_vec());
                open = false;
            }
            ObuType::FrameHeader => {
                let shows_existing = !reduced_still_picture_header
                    && found.obu.payload.first().is_some_and(|b| b & 0x80 != 0);
                open = !shows_existing;
                if open {
                    units.push(raw.to_vec());
                }
            }
            ObuType::TileGroup if open => {
                if let Some(unit) = units.last_mut() {
                    unit.extend_from_slice(raw);
                }
            }
            _ => {}
        }
    }
    units
}

/// The state of one stream after the frames decoded so far.
#[derive(Debug, Clone, Default)]
pub struct StreamDecodeState {
    motion: MotionFieldState,
    /// The frame each reference slot holds (what a later `show_existing_frame` displays).
    slots: [Option<Arc<ParsedFrame>>; 8],
    /// `(rng, cnt, dif)` after every symbol of the frame decoded last, for comparison with an
    /// instrumented dav1d.
    #[cfg(test)]
    pub(crate) last_trace: Vec<(u32, i32, usize)>,
}

impl StreamDecodeState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a unit that holds no decodable frame (for example a `show_existing_frame` header),
    /// keeping the reference slots' header state current. `obu_data`: the sequence header OBU
    /// followed by the unit's OBUs.
    ///
    /// A `show_existing_frame` unit yields the frame its slot holds, with that frame's analysis; if
    /// that frame is a key frame, every reference slot then holds it.
    pub fn skip_unit(&mut self, obu_data: &[u8]) -> Result<Arc<ParsedFrame>, BitvueError> {
        let parsed = ParsedFrame::parse_with_ref_state(obu_data, &mut self.motion.ref_state)?;
        if let Some((slot, shown)) = parsed.show_existing_slot.and_then(|slot| {
            self.slots[usize::from(slot) & 7]
                .clone()
                .map(|shown| (slot, shown))
        }) {
            // Showing a key frame again refreshes every reference slot with it.
            if shown.key_frame {
                self.motion.refresh_all_from(slot);
                self.slots = std::array::from_fn(|_| Some(Arc::clone(&shown)));
            }
            return Ok(shown);
        }
        Ok(Arc::new(parsed))
    }

    /// Decodes one frame: `obu_data` is the sequence header OBU followed by one Frame OBU. The
    /// returned frame carries its decoded coding units; the reference slots it refreshes now hold
    /// this frame's CDFs and motion field.
    pub fn decode_next(
        &mut self,
        obu_data: &[u8],
        seq: &SequenceHeader,
    ) -> Result<Arc<ParsedFrame>, BitvueError> {
        let prev_ref_order_hint = *self.motion.ref_state.ref_order_hint();
        let mut parsed = ParsedFrame::parse_with_ref_state(obu_data, &mut self.motion.ref_state)?;
        if !parsed.has_tile_data() {
            return Ok(Arc::new(parsed));
        }

        let order_hints = OrderHints {
            enabled: seq.enable_order_hint,
            bits: seq.order_hint_bits_minus_1.map_or(0, |v| u32::from(v) + 1),
            slots: prev_ref_order_hint,
        };
        let initial_cdf = self.initial_cdf(&parsed);
        let temporal = self.temporal_candidates(&parsed, &order_hints);
        let (units, outcome, final_cdf) = self.decode_tile(&parsed, temporal, &initial_cdf);
        self.record_frame(&parsed, &order_hints, &units, &initial_cdf, &final_cdf);

        parsed.decoded = Some(ParsedCodingUnits {
            units: Arc::new(units),
            outcome,
        });
        let parsed = Arc::new(parsed);
        self.refresh_slots(&parsed);
        Ok(parsed)
    }

    /// The CDFs the frame's tile starts from: its primary reference frame's saved CDFs, or the
    /// defaults for its quantizer when it has none (or that slot saved none).
    fn initial_cdf(&self, parsed: &ParsedFrame) -> CdfContext {
        let qcat = quantizer_category(parsed.frame_type.base_qp.unwrap_or(128) as i16);
        match (parsed.primary_ref_frame, parsed.ref_frame_idx) {
            (slot @ 0..=6, Some(idx)) => self
                .motion
                .saved_cdf(idx[slot as usize])
                .cloned()
                .unwrap_or_else(|| CdfContext::new_with_qcat(qcat)),
            _ => CdfContext::new_with_qcat(qcat),
        }
    }

    /// Temporal motion-vector candidates, projected from the motion fields of the references,
    /// with the frame's `pocdiff`s. `None` for a frame that does not use them.
    fn temporal_candidates(
        &self,
        parsed: &ParsedFrame,
        order_hints: &OrderHints,
    ) -> Option<(ProjectedMotionField, [i32; 7])> {
        let (true, Some(ref_frame_idx)) = (parsed.use_ref_frame_mvs, parsed.ref_frame_idx) else {
            return None;
        };
        let dims = parsed.dimensions;
        if !frame_within_decode_limit(&dims) {
            return None; // not decoded, so nothing to project into
        }
        let sources = crate::tile::select_motion_field_sources(
            &self.motion,
            &order_hints.slots,
            &ref_frame_idx,
            parsed.order_hint,
            order_hints.enabled,
            order_hints.bits,
            (
                dims.width.div_ceil(8).max(1),
                dims.height.div_ceil(8).max(1),
            ),
        );
        let projected = crate::tile::project_motion_field(
            &sources,
            &self.motion,
            dims.width.div_ceil(8).max(1),
            dims.height.div_ceil(8).max(1),
        );
        let pocdiff: [i32; 7] = std::array::from_fn(|i| {
            order_hints
                .dist(parsed.order_hint, ref_frame_idx[i])
                .clamp(-31, 31) as i32
        });
        Some((projected, pocdiff))
    }

    /// Decodes the frame's tile into coding units, its outcome and the CDFs it ended with.
    fn decode_tile(
        &mut self,
        parsed: &ParsedFrame,
        temporal: Option<(ProjectedMotionField, [i32; 7])>,
        initial_cdf: &CdfContext,
    ) -> (Vec<CodingUnit>, DecodeOutcome, CdfContext) {
        let decode = decode_frame(
            parsed,
            FrameDecodeInputs {
                temporal: temporal.as_ref().map(|(field, pocdiff)| (field, *pocdiff)),
                initial_cdf: Some(initial_cdf.clone()),
            },
        );
        #[cfg(test)]
        {
            self.last_trace = decode.as_ref().map(|d| d.trace.clone()).unwrap_or_default();
        }
        match decode {
            Ok(d) => (d.units, d.outcome, d.final_cdf),
            // The tile could not even be started (too small to hold a symbol): the slots are still
            // refreshed, with nothing to inherit.
            Err(_) => (Vec::new(), DecodeOutcome::default(), initial_cdf.clone()),
        }
    }

    /// What the frame leaves behind for later frames: its saved CDFs and motion field, and the
    /// order hints of the slots it refreshes.
    fn record_frame(
        &mut self,
        parsed: &ParsedFrame,
        order_hints: &OrderHints,
        units: &[CodingUnit],
        initial_cdf: &CdfContext,
        final_cdf: &CdfContext,
    ) {
        let saved_cdf = if parsed.disable_frame_end_update_cdf {
            initial_cdf.clone()
        } else {
            initial_cdf.saved_at_frame_end(final_cdf, parsed.frame_type.is_intra_only)
        };
        let mfmv_sign: [bool; 7] = match parsed.ref_frame_idx {
            Some(ref_frame_idx) => std::array::from_fn(|i| {
                order_hints.dist_to(ref_frame_idx[i], parsed.order_hint) < 0
            }),
            None => [false; 7],
        };
        let dims = parsed.dimensions;
        // A frame past the decode limit was not decoded (no units): it saves no motion field, and
        // does not build one the size of its declared extent either.
        let decoded = frame_within_decode_limit(&dims);
        let grid = if decoded {
            crate::tile::store_motion_field(units, dims.width, dims.height, &mfmv_sign)
        } else {
            crate::tile::store_motion_field(&[], 1, 1, &mfmv_sign)
        };
        self.motion
            .store_cdf(parsed.refresh_frame_flags, &saved_cdf);
        self.motion.update(
            &order_hints.slots,
            parsed.refresh_frame_flags,
            parsed.ref_frame_idx.as_ref(),
            grid,
            decoded && (!parsed.frame_type.is_intra_only || parsed.allow_intrabc),
        );
    }

    /// Makes every reference slot the frame refreshes hold it.
    fn refresh_slots(&mut self, parsed: &Arc<ParsedFrame>) {
        for slot in 0..8 {
            if parsed.refresh_frame_flags & (1 << slot) != 0 {
                self.slots[slot] = Some(Arc::clone(parsed));
            }
        }
    }
}

/// The order-hint parameters of the sequence and the order hints the reference slots held before
/// the frame being decoded.
struct OrderHints {
    enabled: bool,
    bits: u32,
    slots: [u32; 8],
}

impl OrderHints {
    /// Distance from the reference in `slot_idx` to the frame with `order_hint` (spec 7.12.3's
    /// `get_relative_dist(order_hint, ref)`).
    fn dist(&self, order_hint: u32, slot_idx: u8) -> i64 {
        crate::frame_header_full::relative_dist(
            order_hint,
            self.slots[slot_idx as usize],
            self.enabled,
            self.bits,
        )
    }

    /// The same distance with the arguments the other way round.
    fn dist_to(&self, slot_idx: u8, order_hint: u32) -> i64 {
        crate::frame_header_full::relative_dist(
            self.slots[slot_idx as usize],
            order_hint,
            self.enabled,
            self.bits,
        )
    }
}

/// The CDF quantizer category of a base quantizer index.
pub(super) fn quantizer_category(base_qp: i16) -> u8 {
    (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obu::{ObuIterator, ObuType};

    const FIXTURE: &[u8] = include_bytes!("../../../../test_data/av1_test.ivf");

    /// The fixture's sequence header and its first two Frame OBUs (the key frame, which refreshes
    /// every slot, then a hidden frame that refreshes slot 6 only).
    fn key_then_hidden_frame() -> (SequenceHeader, Vec<Vec<u8>>) {
        let (_hdr, packets) = crate::ivf::parse_ivf_frames(FIXTURE).unwrap();
        let mut seq_bytes = Vec::new();
        let mut frames = Vec::new();
        for packet in packets.iter().take(2) {
            let mut iter = ObuIterator::new(&packet.data);
            while let Some(Ok(found)) = iter.next_obu_with_offset() {
                let raw = packet.data[found.offset..found.offset + found.consumed].to_vec();
                match found.obu.header.obu_type {
                    ObuType::SequenceHeader if seq_bytes.is_empty() => seq_bytes = raw,
                    ObuType::Frame => frames.push([seq_bytes.clone(), raw].concat()),
                    _ => {}
                }
            }
        }
        let seq_obu = ObuIterator::new(&seq_bytes)
            .next_obu_with_offset()
            .unwrap()
            .unwrap();
        let seq = crate::parse_sequence_header(&seq_obu.obu.payload).unwrap();
        (seq, frames)
    }

    /// A temporal delimiter and a `show_existing_frame` header showing `slot`.
    fn show_existing(slot: u8, frames: &[Vec<u8>]) -> Vec<u8> {
        let mut unit = Vec::new();
        // The sequence header OBU that `frames[0]` starts with.
        let mut iter = ObuIterator::new(&frames[0]);
        let first = iter.next_obu_with_offset().unwrap().unwrap();
        unit.extend_from_slice(&frames[0][first.offset..first.offset + first.consumed]);
        unit.extend_from_slice(&[0x12, 0x00, 0x1A, 0x01, 0x80 | (slot << 4) | 0x08]);
        unit
    }

    /// Showing a key frame again refreshes every reference slot with it (spec 7.21): its header
    /// state and CDFs, and no motion field (dav1d drops `refmvs` for those slots).
    #[test]
    fn showing_an_existing_key_frame_refreshes_every_slot() {
        let (seq, frames) = key_then_hidden_frame();
        let mut state = StreamDecodeState::new();
        let key = state.decode_next(&frames[0], &seq).unwrap();
        assert!(key.key_frame);
        let hidden = state.decode_next(&frames[1], &seq).unwrap();
        assert!(!hidden.key_frame);
        assert!(state.motion.has_motion_field(6));
        assert_eq!(hidden.refresh_frame_flags, 1 << 6, "fixture's hidden frame");
        assert!(Arc::ptr_eq(state.slots[6].as_ref().unwrap(), &hidden));
        assert_ne!(
            state.motion.ref_state.ref_order_hint()[6],
            state.motion.ref_state.ref_order_hint()[0]
        );

        let shown = state.skip_unit(&show_existing(0, &frames)).unwrap();
        assert!(Arc::ptr_eq(&shown, &key));
        for slot in 0..8u8 {
            assert!(
                Arc::ptr_eq(state.slots[usize::from(slot)].as_ref().unwrap(), &key),
                "slot {slot} holds the key frame"
            );
            assert_eq!(
                state.motion.ref_state.ref_order_hint()[usize::from(slot)],
                key.order_hint
            );
            assert!(std::ptr::eq(
                state.motion.saved_cdf(slot).unwrap(),
                state.motion.saved_cdf(0).unwrap()
            ));
            assert!(!state.motion.has_motion_field(slot));
        }
    }

    /// Showing an inter frame again changes nothing.
    #[test]
    fn showing_an_existing_inter_frame_leaves_the_slots_alone() {
        let (seq, frames) = key_then_hidden_frame();
        let mut state = StreamDecodeState::new();
        state.decode_next(&frames[0], &seq).unwrap();
        let hidden = state.decode_next(&frames[1], &seq).unwrap();
        let shown = state.skip_unit(&show_existing(6, &frames)).unwrap();
        assert!(Arc::ptr_eq(&shown, &hidden));
        assert!(!Arc::ptr_eq(
            state.slots[0].as_ref().unwrap(),
            state.slots[6].as_ref().unwrap()
        ));
    }

    // ---- frame_units over rebuilt OBU sequences ----

    fn obu(obu_type: ObuType, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![((obu_type as u8) << 3) | 0x02];
        out.extend(crate::encode_uleb128(payload.len() as u64));
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn frame_units_of_nothing_or_of_obus_without_frames_is_empty() {
        assert!(frame_units(&[], false).is_empty());
        let mut packet = obu(ObuType::TemporalDelimiter, &[]);
        packet.extend(obu(ObuType::Padding, &[0; 4]));
        packet.extend(obu(ObuType::Metadata, &[1, 2]));
        packet.extend(obu(ObuType::RedundantFrameHeader, &[0x10]));
        assert!(frame_units(&packet, false).is_empty());
    }

    #[test]
    fn each_frame_obu_and_each_header_with_its_tile_groups_is_one_unit_of_the_original_bytes() {
        let frame_a = obu(ObuType::Frame, &[1, 2, 3]);
        let header = obu(ObuType::FrameHeader, &[0x10, 0x20]);
        let tg1 = obu(ObuType::TileGroup, &[4, 5]);
        let tg2 = obu(ObuType::TileGroup, &[6]);
        let frame_b = obu(ObuType::Frame, &[7, 8]);
        let mut packet = obu(ObuType::TemporalDelimiter, &[]);
        packet.extend(&frame_a);
        packet.extend(&header);
        packet.extend(&tg1);
        packet.extend(obu(ObuType::RedundantFrameHeader, &[0x10, 0x20]));
        packet.extend(&tg2);
        packet.extend(&frame_b);
        let units = frame_units(&packet, false);
        assert_eq!(units, vec![frame_a, [header, tg1, tg2].concat(), frame_b,]);
    }

    #[test]
    fn a_tile_group_with_no_open_header_is_dropped() {
        // Before any header, and after a Frame OBU (which closes the previous header).
        let mut packet = obu(ObuType::TileGroup, &[9, 9]);
        let frame = obu(ObuType::Frame, &[1]);
        packet.extend(&frame);
        packet.extend(obu(ObuType::TileGroup, &[8, 8]));
        assert_eq!(frame_units(&packet, false), vec![frame]);

        // After a header that was closed by a Frame OBU in between.
        let mut packet = obu(ObuType::FrameHeader, &[0x10]);
        let frame = obu(ObuType::Frame, &[2]);
        packet.extend(&frame);
        packet.extend(obu(ObuType::TileGroup, &[7]));
        let units = frame_units(&packet, false);
        assert_eq!(units.len(), 2);
        assert_eq!(
            units[1], frame,
            "the late tile group is not appended to the Frame OBU"
        );
    }

    #[test]
    fn a_show_existing_frame_header_makes_no_unit_and_its_tile_groups_are_ignored() {
        // First payload bit set = show_existing_frame.
        let mut packet = obu(ObuType::FrameHeader, &[0x80, 0x00]);
        packet.extend(obu(ObuType::TileGroup, &[1, 2, 3]));
        assert!(frame_units(&packet, false).is_empty());
        // Bit clear: an ordinary header.
        let mut packet = obu(ObuType::FrameHeader, &[0x7F]);
        packet.extend(obu(ObuType::TileGroup, &[1]));
        assert_eq!(frame_units(&packet, false).len(), 1);
    }

    #[test]
    fn a_reduced_still_picture_header_has_no_show_existing_bit() {
        let mut packet = obu(ObuType::FrameHeader, &[0x80]);
        let tg = obu(ObuType::TileGroup, &[1, 2]);
        packet.extend(&tg);
        let units = frame_units(&packet, true);
        assert_eq!(units.len(), 1, "the high bit is a different element here");
        assert!(units[0].ends_with(&tg));
    }

    #[test]
    fn a_frame_header_without_payload_is_an_ordinary_header() {
        let mut packet = obu(ObuType::FrameHeader, &[]);
        packet.extend(obu(ObuType::TileGroup, &[1]));
        assert_eq!(frame_units(&packet, false).len(), 1);
    }

    #[test]
    fn a_truncated_trailing_obu_ends_the_scan_but_keeps_the_units_before_it() {
        let frame = obu(ObuType::Frame, &[1, 2, 3]);
        let mut packet = frame.clone();
        let mut cut = obu(ObuType::Frame, &[4, 5, 6, 7, 8]);
        cut.truncate(cut.len() - 2);
        packet.extend(cut);
        assert_eq!(frame_units(&packet, false), vec![frame]);
    }
}

#[cfg(test)]
#[path = "stream_state_test.rs"]
mod stream_state_test;
