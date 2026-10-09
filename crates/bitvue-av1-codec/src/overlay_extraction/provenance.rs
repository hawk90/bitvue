//! How far a frame's decoded data can be trusted.
//!
//! The overlay extractors used to hand back a grid whether or not the tile had actually been
//! decoded: a parse that failed (or stopped part-way) still produced a grid, padded with invented
//! "scaffold" cells, and nothing told the caller which cells were real. [`Provenance`] says what
//! a frame's data is, and [`DecodeOutcome`] is the evidence it is derived from.

use super::cu_parser::parse_coding_units_checked;
use super::parser::ParsedFrame;

/// What the symbol decoder got through for one frame's tile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DecodeOutcome {
    /// Superblocks the tile holds.
    pub superblocks_total: u32,
    /// Superblocks decoded before the first error (decoding stops there: after an error the
    /// decoder's position is meaningless, so later superblocks would only be noise).
    pub superblocks_decoded: u32,
    /// Whether the data ends where the decoded symbols say it should
    /// ([`crate::symbol::ArithmeticDecoder::padding_is_conformant`]), judged wherever decoding
    /// stopped. Only means something together with a complete decode.
    pub padding_conformant: bool,
}

impl DecodeOutcome {
    /// Every superblock decoded and the tile ends exactly where it should.
    pub fn is_verified(&self) -> bool {
        self.superblocks_total > 0
            && self.superblocks_decoded == self.superblocks_total
            && self.padding_conformant
    }
}

/// What a frame's analysis data is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Decoded, and the tile's padding is where the decoded symbols put it. A desynchronised
    /// decode almost never ends up there, so this is strong evidence the syntax was read
    /// correctly -- not proof that every derived value (an MV, say) is right.
    Verified,
    /// Decoded, but the decode stopped early or the tile does not end where it should: the data
    /// is probably wrong somewhere, and nothing says where.
    Unverified,
    /// Nothing was decoded (no tile data, or the tile could not be parsed at all). Whatever
    /// grid exists for such a frame is invented, not an approximation of the frame.
    Scaffold,
}

impl Provenance {
    /// Wire name used by the sidecar protocol.
    pub fn as_str(self) -> &'static str {
        match self {
            Provenance::Verified => "verified",
            Provenance::Unverified => "unverified",
            Provenance::Scaffold => "scaffold",
        }
    }
}

/// The error every extractor returns for a frame whose tile data cannot be decoded. There is no
/// substitute grid: the caller is told the frame has no analysis data instead.
pub(crate) fn no_decodable_tile() -> bitvue_engine::BitvueError {
    bitvue_engine::BitvueError::Decode("frame has no decodable tile data".into())
}

/// The arithmetic decoder cannot start on fewer bytes. A frame that is all skip blocks can be
/// this small, so no larger guess is used.
const MIN_TILE_BYTES: usize = 2;

/// Whether the extractors try to decode `parsed`'s tile at all. A frame without tile data (a
/// `show_existing_frame`, a tile too small to hold a symbol) is never decoded.
pub(crate) fn has_decodable_tile(parsed: &ParsedFrame) -> bool {
    parsed.tile_data.len() >= MIN_TILE_BYTES
}

/// The [`Provenance`] of `parsed`'s analysis data, from the same (cached) parse every extractor
/// uses, so it always agrees with the grids they return.
pub fn frame_provenance(parsed: &ParsedFrame) -> Provenance {
    if !has_decodable_tile(parsed) {
        return Provenance::Scaffold;
    }
    match parse_coding_units_checked(parsed) {
        Ok(result) if result.outcome.is_verified() => Provenance::Verified,
        Ok(_) => Provenance::Unverified,
        Err(_) => Provenance::Scaffold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(total: u32, decoded: u32, padding: bool) -> DecodeOutcome {
        DecodeOutcome {
            superblocks_total: total,
            superblocks_decoded: decoded,
            padding_conformant: padding,
        }
    }

    #[test]
    fn a_decode_is_verified_only_if_complete_and_ending_where_it_should() {
        assert!(outcome(6, 6, true).is_verified());
        assert!(!outcome(6, 6, false).is_verified(), "padding off");
        assert!(!outcome(6, 5, true).is_verified(), "stopped early");
        assert!(!outcome(0, 0, true).is_verified(), "nothing to decode");
        assert!(!DecodeOutcome::default().is_verified());
    }

    #[test]
    fn provenance_wire_names_are_stable() {
        assert_eq!(Provenance::Verified.as_str(), "verified");
        assert_eq!(Provenance::Unverified.as_str(), "unverified");
        assert_eq!(Provenance::Scaffold.as_str(), "scaffold");
    }

    const FIXTURE: &[u8] = include_bytes!("../../../../test_data/av1_test.ivf");

    /// Parses IVF chunk `chunk` on its own, with the stream's sequence header prepended -- the
    /// stateless way the sidecar parses a frame.
    fn parse_chunk(chunk: usize) -> ParsedFrame {
        use crate::obu::{ObuIterator, ObuType};
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(FIXTURE).unwrap();
        let mut seq = Vec::new();
        let mut it = ObuIterator::new(&frames[0].data);
        while let Some(Ok(found)) = it.next_obu_with_offset() {
            if found.obu.header.obu_type == ObuType::SequenceHeader {
                seq = frames[0].data[found.offset..found.offset + found.consumed].to_vec();
            }
        }
        let data = [seq.as_slice(), frames[chunk].data.as_slice()].concat();
        ParsedFrame::parse(&data).unwrap()
    }

    /// The key frame needs no earlier state and decodes exactly (see `cu_parser`'s dav1d
    /// comparison), so its tile ends where the symbols say it does.
    #[test]
    fn the_fixture_key_frame_is_verified() {
        assert_eq!(frame_provenance(&parse_chunk(0)), Provenance::Verified);
    }

    /// An inter frame parsed on its own starts from default CDFs and has no motion field, so it
    /// does not decode correctly -- and the check notices.
    #[test]
    fn an_inter_frame_parsed_without_its_references_is_unverified() {
        assert_eq!(frame_provenance(&parse_chunk(2)), Provenance::Unverified);
    }

    #[test]
    fn a_tile_that_is_cut_short_or_padded_wrongly_is_unverified() {
        let key = parse_chunk(0);
        let mut cut = key.clone();
        cut.tile_data = key.tile_data[..key.tile_data.len() - 3].into();
        assert_eq!(frame_provenance(&cut), Provenance::Unverified);

        let mut flipped = key.clone();
        let mut bytes = key.tile_data.to_vec();
        let at = bytes.len() / 3;
        bytes[at] ^= 0x10;
        flipped.tile_data = bytes.into();
        assert_eq!(frame_provenance(&flipped), Provenance::Unverified);
    }

    #[test]
    fn a_frame_without_a_tile_is_scaffold() {
        let mut none = parse_chunk(0);
        none.tile_data = Vec::new().into();
        assert_eq!(frame_provenance(&none), Provenance::Scaffold);
    }

    /// The partition grid is built from the same parse as the other grids, so a decode that
    /// stops early shows up as missing blocks, never as invented ones.
    #[test]
    fn the_partition_grid_has_exactly_the_decoded_blocks() {
        let key = parse_chunk(0);
        let units = crate::overlay_extraction::parse_all_coding_units(&key).unwrap();
        let grid = crate::overlay_extraction::extract_partition_grid_from_parsed(&key).unwrap();
        assert_eq!(grid.blocks.len(), units.len());

        let mut cut = key.clone();
        cut.tile_data = key.tile_data[..key.tile_data.len() / 2].into();
        let partial = crate::overlay_extraction::extract_partition_grid_from_parsed(&cut).unwrap();
        assert!(partial.blocks.len() < units.len());
    }

    /// After the first superblock that fails to decode nothing more is read: the decoder has lost
    /// its place, so later superblocks would be noise presented as coding units.
    #[test]
    fn decoding_stops_at_the_first_failed_superblock() {
        let key = parse_chunk(0);
        let mut cut = key.clone();
        cut.tile_data = key.tile_data[..key.tile_data.len() / 3].into();
        let result = parse_coding_units_checked(&cut).unwrap();
        let outcome = result.outcome;
        assert!(outcome.superblocks_decoded < outcome.superblocks_total);
        let sb = key.dimensions.sb_size;
        let sb_cols = key.dimensions.sb_cols;
        // The decoded superblocks are exactly a prefix: no hole where one failed, nothing after.
        let mut seen: Vec<u32> = result
            .units
            .iter()
            .map(|cu| (cu.y / sb) * sb_cols + cu.x / sb)
            .collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen,
            (0..outcome.superblocks_decoded).collect::<Vec<_>>(),
            "coding units must come from superblocks 0..{}",
            outcome.superblocks_decoded
        );
    }

    /// Too few bytes to hold a decodable tile: not decoded, however the parser would cope.
    #[test]
    fn a_tile_too_small_to_decode_is_scaffold() {
        let mut tiny = parse_chunk(0);
        tiny.tile_data = vec![0x80u8; 1].into();
        assert_eq!(frame_provenance(&tiny), Provenance::Scaffold);
    }

    /// Same invariant when the failure is in the middle: flip one bit somewhere in the tile and
    /// the decode goes wrong and usually errors a few superblocks on -- and whatever happens
    /// after that error must not be reported.
    #[test]
    fn a_corrupted_tile_never_reports_superblocks_past_its_first_failure() {
        let key = parse_chunk(0);
        let sb = key.dimensions.sb_size;
        let sb_cols = key.dimensions.sb_cols;
        let len = key.tile_data.len();
        let mut stopped_early = 0;
        for trial in 0..120usize {
            let mut bytes = key.tile_data.to_vec();
            bytes[(trial * 7919) % (len * 3 / 4)] ^= 1 << (trial % 8);
            let mut bad = key.clone();
            bad.tile_data = bytes.into();
            let result = parse_coding_units_checked(&bad).unwrap();
            let decoded = result.outcome.superblocks_decoded;
            if decoded < result.outcome.superblocks_total {
                stopped_early += 1;
            }
            let mut seen: Vec<u32> = result
                .units
                .iter()
                .map(|cu| (cu.y / sb) * sb_cols + cu.x / sb)
                .collect();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen, (0..decoded).collect::<Vec<_>>(), "trial {trial}");
        }
        assert!(stopped_early > 0, "no trial reached a failing superblock");
    }
}
