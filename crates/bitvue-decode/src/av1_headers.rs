//! The frame headers of the packets sent to dav1d, read with the full parser.
//!
//! dav1d's Rust binding exposes neither the frame type nor the quantiser of a picture, so both are
//! read from the packet that was sent. `parse_frame_header_basic` is not good enough for that: it
//! reads without the sequence header and guesses the layout, so its `base_q_idx` and
//! `refresh_frame_flags` are wrong on nearly every frame (the key frame of
//! `av1_aomenc_testsrc2.ivf` reads 128 where FFmpeg's parser and `parse_frame_header_full` both
//! read 14). The full parser needs the sequence header and the reference-slot state built up from
//! every earlier frame, so this keeps both and is fed the packets in order.

use bitvue_av1_codec::frame_header_full::{parse_frame_header_full, RefFrameState};
use bitvue_av1_codec::{
    parse_all_obus, parse_sequence_header, FrameHeader, ObuType, SequenceHeader,
};

/// Reads packet after packet; see the module doc.
pub(crate) struct PacketHeaders {
    seq: Option<SequenceHeader>,
    ref_state: RefFrameState,
    /// The reference state can no longer be trusted (a packet did not parse, dav1d refused one,
    /// the decoder was flushed). Nothing is reported until the next shown key frame, which
    /// refreshes every slot and so starts the state afresh.
    lost: bool,
}

impl PacketHeaders {
    pub(crate) fn new() -> Self {
        Self {
            seq: None,
            ref_state: RefFrameState::new(),
            lost: false,
        }
    }

    /// Gives up the reference state; see `lost`.
    pub(crate) fn lose(&mut self) {
        self.ref_state = RefFrameState::new();
        self.lost = true;
    }

    /// The frame headers of `packet` in order, advancing the reference state past them. `None`
    /// when the packet cannot be read or the state is lost.
    pub(crate) fn read(&mut self, packet: &[u8]) -> Option<Vec<FrameHeader>> {
        let Ok(obus) = parse_all_obus(packet) else {
            self.lose();
            return None;
        };
        let mut headers = Vec::new();
        for obu in &obus {
            match obu.header.obu_type {
                ObuType::SequenceHeader => match parse_sequence_header(&obu.payload) {
                    Ok(seq) => self.seq = Some(seq),
                    Err(_) => {
                        self.lose();
                        return None;
                    }
                },
                ObuType::Frame | ObuType::FrameHeader => {
                    let parsed = self.seq.as_ref().and_then(|seq| {
                        parse_frame_header_full(&obu.payload, seq, &mut self.ref_state).ok()
                    });
                    match parsed {
                        Some(header) => headers.push(header),
                        None => {
                            self.lose();
                            return None;
                        }
                    }
                }
                _ => {}
            }
        }
        if self.lost {
            let restarts = headers
                .iter()
                .any(|h| !h.show_existing_frame && h.show_frame && is_key(h));
            if !restarts {
                return None;
            }
            self.lost = false;
        }
        Some(headers)
    }
}

fn is_key(header: &FrameHeader) -> bool {
    header.frame_type == bitvue_engine::FrameType::Key
}

/// The header of the picture a packet makes visible.
///
/// Known only when the packet names exactly one frame that is shown (`show_frame` set, not a
/// `show_existing_frame`): then that frame is the picture dav1d returns, whatever hidden frames
/// came before it in the same packet. `None` when the packet repeats an earlier frame
/// (`show_existing_frame`: the header carries no type or quantiser for it), shows no frame or
/// several (spatial layers).
pub(crate) fn shown_header(headers: &[FrameHeader]) -> Option<&FrameHeader> {
    let mut shown = None;
    for header in headers {
        if header.show_existing_frame {
            return None;
        }
        if header.show_frame {
            if shown.is_some() {
                return None;
            }
            shown = Some(header);
        }
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvue_engine::FrameType;

    /// The IVF packets of a clip below `test_data`.
    fn packets(name: &str) -> Vec<Vec<u8>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_data")
            .join(name);
        let data = std::fs::read(path).unwrap();
        let (mut pos, mut out) = (32usize, Vec::new());
        while pos + 12 <= data.len() {
            let size = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
            out.push(data[pos + 12..pos + 12 + size].to_vec());
            pos += 12 + size;
        }
        out
    }

    /// Reads the first `n` packets of the testsrc2 clip in order, returning the last one's headers.
    fn read_through(reader: &mut PacketHeaders, n: usize) -> Vec<FrameHeader> {
        let mut last = Vec::new();
        for p in packets("av1_aomenc_testsrc2.ivf").iter().take(n) {
            last = reader.read(p).expect("packet reads");
        }
        last
    }

    /// `(show_existing_frame, show_frame, base_q_idx, refresh_frame_flags)` of every header of
    /// packet `n`; the numbers are what FFmpeg's `trace_headers` bitstream filter reports for the
    /// clip (an AV1 parser written independently of this repository's).
    fn summary(headers: &[FrameHeader]) -> Vec<(bool, bool, Option<u8>, Option<u8>)> {
        headers
            .iter()
            .map(|h| {
                (
                    h.show_existing_frame,
                    h.show_frame,
                    h.base_q_idx,
                    h.refresh_frame_flags,
                )
            })
            .collect()
    }

    #[test]
    fn the_headers_carry_the_values_ffmpegs_parser_reads() {
        let mut reader = PacketHeaders::new();
        // packet 0: the shown key frame
        assert_eq!(
            summary(&read_through(&mut reader, 1)),
            [(false, true, Some(14), Some(255))]
        );
        // packet 1: three hidden frames, then the shown one
        assert_eq!(
            summary(&read_through(&mut PacketHeaders::new(), 2)),
            [
                (false, false, Some(52), Some(2)),
                (false, false, Some(69), Some(4)),
                (false, false, Some(77), Some(8)),
                (false, true, Some(85), Some(16)),
            ]
        );
        // packet 2 repeats a frame and has no quantiser or refresh flags
        let p2 = summary(&read_through(&mut PacketHeaders::new(), 3));
        assert_eq!(p2, [(true, true, None, None)]);
    }

    #[test]
    fn the_picture_of_a_packet_is_its_one_shown_frame() {
        let mut reader = PacketHeaders::new();
        let packets = packets("av1_aomenc_testsrc2.ivf");
        let mut shown = Vec::new();
        for p in &packets[..4] {
            let headers = reader.read(p).unwrap();
            shown.push(shown_header(&headers).map(|h| (h.frame_type, h.base_q_idx)));
        }
        assert_eq!(
            shown,
            [
                Some((FrameType::Key, Some(14))),
                Some((FrameType::Inter, Some(85))), // after three hidden frames
                None,                               // a repeated frame
                Some((FrameType::Inter, Some(85))),
            ]
        );
    }

    #[test]
    fn the_selection_rules_on_edited_headers() {
        let headers = read_through(&mut PacketHeaders::new(), 2); // 3 hidden + 1 shown
        assert_eq!(shown_header(&headers).map(|h| h.base_q_idx), Some(Some(85)));

        // two shown frames: no single picture
        let mut two = headers.clone();
        two[0].show_frame = true;
        assert!(shown_header(&two).is_none());

        // nothing shown
        let mut none = headers.clone();
        none[3].show_frame = false;
        assert!(shown_header(&none).is_none());

        // a repeated frame anywhere in the packet leaves it open, even after a shown one
        let mut repeated = headers.clone();
        repeated[3].show_existing_frame = true;
        assert!(shown_header(&repeated).is_none());
        let mut after = headers.clone();
        after.push(after[0].clone());
        after[4].show_existing_frame = true;
        assert!(shown_header(&after).is_none());

        assert!(shown_header(&[]).is_none());
    }

    #[test]
    fn nothing_is_reported_without_a_sequence_header() {
        let mut reader = PacketHeaders::new();
        // packet 1 holds frames but no sequence header, and none came before it
        assert!(reader
            .read(&packets("av1_aomenc_testsrc2.ivf")[1])
            .is_none());
    }

    #[test]
    fn a_packet_that_does_not_parse_loses_the_state_until_the_next_key_frame() {
        let clip = packets("av1_aomenc_testsrc2.ivf");
        let mut reader = PacketHeaders::new();
        assert!(reader.read(&clip[0]).is_some());
        assert!(reader.read(&[0xFF, 0xFF, 0xFF]).is_none());
        // the reference state is gone, so the inter packets after it are not reported
        assert!(reader.read(&clip[1]).is_none());
        assert!(reader.read(&clip[3]).is_none());
        // a shown key frame starts afresh (here packet 0 again: it carries the sequence header)
        assert!(reader.read(&clip[0]).is_some());
        assert!(reader.read(&clip[1]).is_some());
    }

    #[test]
    fn losing_the_state_by_hand_behaves_the_same() {
        let clip = packets("av1_aomenc_testsrc2.ivf");
        let mut reader = PacketHeaders::new();
        reader.read(&clip[0]).unwrap();
        reader.lose();
        assert!(reader.read(&clip[1]).is_none());
        assert!(reader.read(&clip[0]).is_some());
    }
}
