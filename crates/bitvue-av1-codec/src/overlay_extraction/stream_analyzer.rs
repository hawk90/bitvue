//! Random access to the analysis (coding units, grids) of the frames of one IVF stream.
//!
//! The coding units of a frame can only be decoded with the state of the frames before it (see
//! [`StreamDecodeState`]), so asking for frame `n` means decoding frames `0..=n` in order. A
//! [`StreamAnalyzer`] makes that affordable for the usual access patterns (stepping forward,
//! scrubbing back and forth):
//!
//! - it keeps the decode state after the frames decoded so far and continues from there for a
//!   later frame;
//! - it keeps a **checkpoint** of the state every [`CHECKPOINT_INTERVAL`] frames (bounded: older
//!   ones are thinned out), so an earlier frame is decoded from the nearest checkpoint before it
//!   instead of from frame 0;
//! - it keeps the last [`RESULT_CACHE`] results.
//!
//! The unit of access is an IVF packet (what the rest of the application calls a frame index). A
//! packet can hold several frames (a hidden one and the shown one); the result is the last frame
//! of the packet that has tile data, normally the shown one. A packet without a decodable frame
//! (for example a `show_existing_frame` header) yields a frame without coding units.

use super::parser::ParsedFrame;
use super::stream_state::{frame_units, StreamDecodeState};
use crate::ivf::{parse_ivf_frames, IvfFrame};
use crate::obu::{ObuIterator, ObuType};
use crate::sequence::{parse_sequence_header, SequenceHeader};
use bitvue_engine::BitvueError;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

/// Frames between two checkpoints.
pub const CHECKPOINT_INTERVAL: usize = 16;
/// Checkpoints kept at most; beyond this every other one is dropped (keeping the first).
const MAX_CHECKPOINTS: usize = 64;
/// Results kept.
pub const RESULT_CACHE: usize = 32;
/// The sequence header is looked for in this many leading packets.
const SEQUENCE_HEADER_SCAN_LIMIT: usize = 8;

pub struct StreamAnalyzer {
    frames: Vec<IvfFrame>,
    seq_bytes: Vec<u8>,
    seq: SequenceHeader,
    checkpoint_interval: usize,
    result_cache: usize,
    /// The next packet to decode; `state` is the state before it.
    cursor: usize,
    state: StreamDecodeState,
    /// Packet index -> the state before that packet.
    checkpoints: BTreeMap<usize, StreamDecodeState>,
    results: VecDeque<(usize, Arc<ParsedFrame>)>,
}

impl StreamAnalyzer {
    /// `ivf`: the whole file. Fails when it is not an IVF file or no sequence header is found in
    /// its first packets.
    pub fn new(ivf: &[u8]) -> Result<Self, BitvueError> {
        Self::with_limits(ivf, CHECKPOINT_INTERVAL, RESULT_CACHE)
    }

    /// [`Self::new`] with explicit checkpoint spacing and result cache size (tests use tiny ones
    /// to exercise eviction).
    pub fn with_limits(
        ivf: &[u8],
        checkpoint_interval: usize,
        result_cache: usize,
    ) -> Result<Self, BitvueError> {
        let (_header, frames) = parse_ivf_frames(ivf)?;
        let (seq_bytes, seq) = find_sequence_header(&frames).ok_or_else(|| {
            BitvueError::Decode("no sequence header in the first packets".to_string())
        })?;
        let mut analyzer = Self {
            frames,
            seq_bytes,
            seq,
            checkpoint_interval: checkpoint_interval.max(1),
            result_cache: result_cache.max(1),
            cursor: 0,
            state: StreamDecodeState::new(),
            checkpoints: BTreeMap::new(),
            results: VecDeque::new(),
        };
        analyzer.checkpoints.insert(0, analyzer.state.clone());
        Ok(analyzer)
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn sequence_header(&self) -> &SequenceHeader {
        &self.seq
    }

    /// The analysis of packet `index`.
    pub fn analyze(&mut self, index: usize) -> Result<Arc<ParsedFrame>, BitvueError> {
        if index >= self.frames.len() {
            return Err(BitvueError::Decode(format!(
                "frame_index {index} out of range (stream has {} frames)",
                self.frames.len()
            )));
        }
        if let Some((_, hit)) = self.results.iter().find(|(i, _)| *i == index) {
            return Ok(Arc::clone(hit));
        }
        if index < self.cursor {
            // Continue from the nearest checkpoint at or before the packet.
            let (&at, state) = self
                .checkpoints
                .range(..=index)
                .next_back()
                .expect("checkpoint 0 always exists");
            self.state = state.clone();
            self.cursor = at;
        }
        let mut result = None;
        while self.cursor <= index {
            if self.cursor.is_multiple_of(self.checkpoint_interval) {
                self.checkpoints
                    .entry(self.cursor)
                    .or_insert_with(|| self.state.clone());
                self.thin_checkpoints();
            }
            let packet = self.cursor;
            self.cursor += 1;
            let decoded = self.decode_packet(packet)?;
            self.remember(packet, Arc::clone(&decoded));
            result = Some(decoded);
        }
        Ok(result.expect("the loop ran at least once: the packet was not decoded yet"))
    }

    fn remember(&mut self, index: usize, result: Arc<ParsedFrame>) {
        self.results.retain(|(i, _)| *i != index);
        self.results.push_back((index, result));
        while self.results.len() > self.result_cache {
            self.results.pop_front();
        }
    }

    fn thin_checkpoints(&mut self) {
        if self.checkpoints.len() <= MAX_CHECKPOINTS {
            return;
        }
        let drop: Vec<usize> = self
            .checkpoints
            .keys()
            .skip(1)
            .step_by(2)
            .copied()
            .collect();
        for key in drop {
            self.checkpoints.remove(&key);
        }
    }

    /// Decodes every frame of the packet, in order, and returns the last one with tile data.
    fn decode_packet(&mut self, index: usize) -> Result<Arc<ParsedFrame>, BitvueError> {
        let data = &self.frames[index].data;
        let mut last = None;
        for unit in frame_units(data, self.seq.reduced_still_picture_header) {
            let obu_data = [self.seq_bytes.as_slice(), unit.as_slice()].concat();
            last = Some(self.state.decode_next(&obu_data, &self.seq)?);
        }
        match last {
            Some(frame) => Ok(frame),
            None => {
                // No frame (a `show_existing_frame` header): keep the header state current.
                let obu_data = [self.seq_bytes.as_slice(), data.as_slice()].concat();
                self.state.skip_unit(&obu_data)
            }
        }
    }
}

fn find_sequence_header(frames: &[IvfFrame]) -> Option<(Vec<u8>, SequenceHeader)> {
    for frame in frames.iter().take(SEQUENCE_HEADER_SCAN_LIMIT) {
        let mut iter = ObuIterator::new(&frame.data);
        while let Some(Ok(found)) = iter.next_obu_with_offset() {
            if found.obu.header.obu_type == ObuType::SequenceHeader {
                let seq = parse_sequence_header(&found.obu.payload).ok()?;
                let bytes = frame.data[found.offset..found.offset + found.consumed].to_vec();
                return Some((bytes, seq));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay_extraction::{frame_provenance, Provenance};

    const RAV1E: &[u8] = include_bytes!("../../../../test_data/av1_rav1e_testsrc2.ivf");
    const AOMENC: &[u8] = include_bytes!("../../../../test_data/av1_aomenc_testsrc2.ivf");
    const SVT: &[u8] = include_bytes!("../../../../test_data/av1_svtav1_testsrc2.ivf");

    /// A fingerprint of everything a frame's analysis consists of.
    fn fingerprint(frame: &ParsedFrame) -> String {
        let decoded = frame.decoded.as_ref();
        format!(
            "{:?}|{:?}",
            decoded.map(|d| d.outcome),
            decoded.map(|d| d
                .units
                .iter()
                .map(|u| (u.x, u.y, u.width, u.height, u.skip, u.mv[0].x, u.mv[0].y))
                .collect::<Vec<_>>())
        )
    }

    /// With the state of the frames before them, every frame of the three encoders' clips decodes
    /// completely and ends exactly where the arithmetic coder says it should: `verified`.
    #[test]
    fn every_frame_of_the_encoder_clips_is_verified() {
        for (name, clip) in [("rav1e", RAV1E), ("aomenc", AOMENC), ("svt-av1", SVT)] {
            let mut analyzer = StreamAnalyzer::new(clip).unwrap();
            for index in 0..analyzer.frame_count() {
                let frame = analyzer.analyze(index).unwrap();
                assert_eq!(
                    frame_provenance(&frame),
                    Provenance::Verified,
                    "{name}: packet {index}"
                );
            }
        }
    }

    /// Asking for frames out of order, with checkpoints and results evicted along the way, gives
    /// the same analysis as decoding straight through.
    /// `clip` with every `Frame` OBU rewritten as a `FrameHeader` OBU (header bits, then the
    /// trailing bits) followed by a `TileGroup` OBU, and, when `redundant` is set, a
    /// `RedundantFrameHeader` copy in between. The tile data is the same bytes, so the decoded
    /// frames must not change.
    fn with_split_frame_obus(clip: &[u8], redundant: bool) -> Vec<u8> {
        use crate::frame_header_full::{parse_frame_header_full, RefFrameState};
        fn obu(obu_type: u8, payload: &[u8]) -> Vec<u8> {
            let mut out = vec![(obu_type << 3) | 0b10];
            let mut size = payload.len();
            loop {
                let low = (size & 0x7f) as u8;
                size >>= 7;
                out.push(if size > 0 { low | 0x80 } else { low });
                if size == 0 {
                    break;
                }
            }
            out.extend_from_slice(payload);
            out
        }

        let (_hdr, packets) = parse_ivf_frames(clip).unwrap();
        let (_, seq) = find_sequence_header(&packets).unwrap();
        let mut ref_state = RefFrameState::new();
        let mut out = clip[..32].to_vec();
        for (pts, packet) in packets.iter().enumerate() {
            let mut data = Vec::new();
            let mut iter = ObuIterator::new(&packet.data);
            while let Some(Ok(found)) = iter.next_obu_with_offset() {
                let raw = &packet.data[found.offset..found.offset + found.consumed];
                if found.obu.header.obu_type != ObuType::Frame {
                    if found.obu.header.obu_type == ObuType::FrameHeader {
                        parse_frame_header_full(&found.obu.payload, &seq, &mut ref_state).unwrap();
                    }
                    data.extend_from_slice(raw);
                    continue;
                }
                assert!(
                    !found.obu.header.has_extension,
                    "no OBU extensions expected"
                );
                let payload = &found.obu.payload;
                let header = parse_frame_header_full(payload, &seq, &mut ref_state).unwrap();
                let bits = header.header_size_bits as usize;
                // Header bits, then trailing bits: a one and zeros up to the byte boundary.
                let mut header_obu = payload[..bits / 8].to_vec();
                header_obu.push(if bits.is_multiple_of(8) {
                    0x80
                } else {
                    (payload[bits / 8] & (0xffu8 << (8 - bits % 8))) | (0x80 >> (bits % 8))
                });
                data.extend(obu(ObuType::FrameHeader as u8, &header_obu));
                if redundant {
                    data.extend(obu(ObuType::RedundantFrameHeader as u8, &header_obu));
                }
                data.extend(obu(
                    ObuType::TileGroup as u8,
                    &payload[header.header_size_bytes..],
                ));
            }
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(pts as u64).to_le_bytes());
            out.extend_from_slice(&data);
        }
        out
    }

    /// A frame header OBU followed by its tile group OBU (and redundant copies of the header)
    /// decodes exactly like the same frame in one `Frame` OBU.
    #[test]
    fn a_frame_header_with_a_separate_tile_group_decodes_like_a_frame_obu() {
        for (name, clip) in [("rav1e", RAV1E), ("aomenc", AOMENC), ("svt-av1", SVT)] {
            let mut whole = StreamAnalyzer::new(clip).unwrap();
            for redundant in [false, true] {
                let split_clip = with_split_frame_obus(clip, redundant);
                let mut split = StreamAnalyzer::new(&split_clip).unwrap();
                assert_eq!(split.frame_count(), whole.frame_count());
                for index in 0..whole.frame_count() {
                    let split_frame = split.analyze(index).unwrap();
                    assert_eq!(
                        fingerprint(&split_frame),
                        fingerprint(&whole.analyze(index).unwrap()),
                        "{name} (redundant={redundant}): packet {index}"
                    );
                    assert_eq!(
                        frame_provenance(&split_frame),
                        Provenance::Verified,
                        "{name} (redundant={redundant}): packet {index}"
                    );
                }
            }
        }
    }

    #[test]
    fn frame_units_groups_headers_with_their_tile_groups() {
        let (_hdr, packets) = parse_ivf_frames(&with_split_frame_obus(AOMENC, true)).unwrap();
        // The second packet holds several frames.
        let units = frame_units(&packets[1].data, false);
        assert!(units.len() > 1);
        for unit in &units {
            let types: Vec<ObuType> = {
                let mut iter = ObuIterator::new(unit);
                std::iter::from_fn(|| iter.next_obu_with_offset().and_then(|r| r.ok()))
                    .map(|found| found.obu.header.obu_type)
                    .collect()
            };
            assert_eq!(types, [ObuType::FrameHeader, ObuType::TileGroup]);
        }
    }

    #[test]
    fn random_access_matches_sequential_decoding() {
        let mut sequential = StreamAnalyzer::new(AOMENC).unwrap();
        let expected: Vec<String> = (0..sequential.frame_count())
            .map(|i| fingerprint(&sequential.analyze(i).unwrap()))
            .collect();

        // A checkpoint every 3 packets, only 2 results kept.
        let mut analyzer = StreamAnalyzer::with_limits(AOMENC, 3, 2).unwrap();
        for index in [9, 3, 11, 0, 7, 7, 4, 10, 1, 5] {
            assert_eq!(
                fingerprint(&analyzer.analyze(index).unwrap()),
                expected[index],
                "packet {index}"
            );
        }
    }

    #[test]
    fn out_of_range_and_non_streams_are_errors() {
        let mut analyzer = StreamAnalyzer::new(RAV1E).unwrap();
        assert!(analyzer.analyze(analyzer.frame_count()).is_err());
        assert!(StreamAnalyzer::new(&[0u8; 64]).is_err());
    }
}
