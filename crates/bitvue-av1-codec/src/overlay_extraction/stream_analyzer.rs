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
use super::stream_state::StreamDecodeState;
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

    /// Decodes every frame OBU of the packet, in order, and returns the last one with tile data.
    fn decode_packet(&mut self, index: usize) -> Result<Arc<ParsedFrame>, BitvueError> {
        let data = &self.frames[index].data;
        let mut last = None;
        let mut iter = ObuIterator::new(data);
        while let Some(Ok(found)) = iter.next_obu_with_offset() {
            if found.obu.header.obu_type != ObuType::Frame {
                continue;
            }
            let raw = &data[found.offset..found.offset + found.consumed];
            let obu_data = [self.seq_bytes.as_slice(), raw].concat();
            last = Some(self.state.decode_next(&obu_data, &self.seq)?);
        }
        match last {
            Some(frame) => Ok(frame),
            None => {
                // No Frame OBU (a `show_existing_frame` header, or a frame header with separate
                // tile groups, which is not decoded): keep the header state current.
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
