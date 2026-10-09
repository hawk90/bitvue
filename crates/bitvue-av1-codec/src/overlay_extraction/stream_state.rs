//! Decode state carried from one frame to the next (the reference frame slots), and the step that
//! decodes one frame with it.
//!
//! A frame of a real stream cannot be decoded alone: it starts from the CDFs its primary reference
//! frame saved, its temporal motion-vector candidates come from the motion fields of earlier
//! frames, and its header depends on the order hints of the reference slots. [`StreamDecodeState`]
//! holds those per slot and is advanced by [`StreamDecodeState::decode_next`], once per frame in
//! decode order. Cloning it is cheap (the per-slot data is shared), which is what makes
//! checkpoints affordable.

use super::cu_parser::{decode_frame, FrameDecodeInputs};
use super::parser::ParsedFrame;
use super::{DecodeOutcome, ParsedCodingUnits};
use crate::sequence::SequenceHeader;
use crate::symbol::CdfContext;
use crate::tile::MotionFieldState;
use bitvue_engine::BitvueError;
use std::sync::Arc;

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

        let order_hint_bits = seq.order_hint_bits_minus_1.map_or(0, |v| u32::from(v) + 1);
        let base_qp = parsed.frame_type.base_qp.unwrap_or(128) as i16;
        let qcat = (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8;
        let dims = parsed.dimensions;

        // Temporal motion-vector candidates: projected from the motion fields of the references.
        let temporal = match (parsed.use_ref_frame_mvs, parsed.ref_frame_idx) {
            (true, Some(ref_frame_idx)) => {
                let sources = crate::tile::select_motion_field_sources(
                    &self.motion,
                    &prev_ref_order_hint,
                    &ref_frame_idx,
                    parsed.order_hint,
                    seq.enable_order_hint,
                    order_hint_bits,
                );
                let projected = crate::tile::project_motion_field(
                    &sources,
                    &self.motion,
                    dims.width.div_ceil(8).max(1),
                    dims.height.div_ceil(8).max(1),
                );
                let pocdiff: [i32; 7] = std::array::from_fn(|i| {
                    crate::frame_header_full::relative_dist(
                        parsed.order_hint,
                        prev_ref_order_hint[ref_frame_idx[i] as usize],
                        seq.enable_order_hint,
                        order_hint_bits,
                    )
                    .clamp(-31, 31) as i32
                });
                Some((projected, pocdiff))
            }
            _ => None,
        };

        // A frame with a primary reference frame starts from that reference's saved CDFs.
        let initial_cdf = match (parsed.primary_ref_frame, parsed.ref_frame_idx) {
            (slot @ 0..=6, Some(idx)) => self
                .motion
                .saved_cdf(idx[slot as usize])
                .cloned()
                .unwrap_or_else(|| CdfContext::new_with_qcat(qcat)),
            _ => CdfContext::new_with_qcat(qcat),
        };

        let decode = decode_frame(
            &parsed,
            FrameDecodeInputs {
                temporal: temporal.as_ref().map(|(field, pocdiff)| (field, *pocdiff)),
                initial_cdf: Some(initial_cdf.clone()),
            },
        );
        #[cfg(test)]
        {
            self.last_trace = decode.as_ref().map(|d| d.trace.clone()).unwrap_or_default();
        }
        let (units, outcome, final_cdf) = match decode {
            Ok(d) => (d.units, d.outcome, d.final_cdf),
            // The tile could not even be started (too small to hold a symbol): the slots are still
            // refreshed, with nothing to inherit.
            Err(_) => (Vec::new(), DecodeOutcome::default(), initial_cdf.clone()),
        };

        // What the frame leaves behind for later frames.
        let saved_cdf = if parsed.disable_frame_end_update_cdf {
            initial_cdf
        } else {
            initial_cdf.saved_at_frame_end(&final_cdf, parsed.frame_type.is_intra_only)
        };
        let mfmv_sign: [bool; 7] = match parsed.ref_frame_idx {
            Some(ref_frame_idx) => std::array::from_fn(|i| {
                crate::frame_header_full::relative_dist(
                    prev_ref_order_hint[ref_frame_idx[i] as usize],
                    parsed.order_hint,
                    seq.enable_order_hint,
                    order_hint_bits,
                ) < 0
            }),
            None => [false; 7],
        };
        let grid = crate::tile::store_motion_field(&units, dims.width, dims.height, &mfmv_sign);
        self.motion
            .store_cdf(parsed.refresh_frame_flags, &saved_cdf);
        self.motion.update(
            &prev_ref_order_hint,
            parsed.refresh_frame_flags,
            parsed.ref_frame_idx.as_ref(),
            grid,
            !parsed.frame_type.is_intra_only || parsed.allow_intrabc,
        );

        parsed.decoded = Some(ParsedCodingUnits {
            units: Arc::new(units),
            outcome,
        });
        let parsed = Arc::new(parsed);
        for slot in 0..8 {
            if parsed.refresh_frame_flags & (1 << slot) != 0 {
                self.slots[slot] = Some(Arc::clone(&parsed));
            }
        }
        Ok(parsed)
    }
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
}
