//! Partition, prediction mode, and transform grid extraction
//!
//! Provides functions to extract partition trees, prediction modes,
//! and transform sizes from AV1 bitstreams.

mod grid_build;
mod partition_grid;
mod prediction_mode_grid;
mod transform_grid;

pub use partition_grid::{extract_partition_grid, extract_partition_grid_from_parsed};
pub use prediction_mode_grid::{
    extract_prediction_mode_grid, extract_prediction_mode_grid_from_parsed, PredictionModeGrid,
};
pub use transform_grid::{
    extract_transform_grid, extract_transform_grid_from_parsed, TransformGrid,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay_extraction::ParsedFrame;
    use crate::tile::{PredictionMode, TxSize};

    #[test]
    fn test_tx_size_values() {
        // Assert TxSize enum values match expected sizes
        assert_eq!(TxSize::Tx4x4.size(), 4);
        assert_eq!(TxSize::Tx8x8.size(), 8);
        assert_eq!(TxSize::Tx16x16.size(), 16);
        assert_eq!(TxSize::Tx32x32.size(), 32);
        assert_eq!(TxSize::Tx64x64.size(), 64);
    }

    #[test]
    fn a_frame_without_decodable_tile_data_is_an_error_for_every_grid() {
        // No tile in these bytes: there is nothing to decode, and no substitute grid.
        let obu_data = vec![0x00, 0x01, 0x02, 0x03];
        let parsed = ParsedFrame::parse(&obu_data).unwrap();

        assert!(extract_partition_grid_from_parsed(&parsed).is_err());
        assert!(extract_prediction_mode_grid_from_parsed(&parsed).is_err());
        assert!(extract_transform_grid_from_parsed(&parsed).is_err());
        assert!(super::super::extract_mv_grid_from_parsed(&parsed).is_err());
        assert!(super::super::extract_qp_grid_from_parsed(&parsed, 0, 32).is_err());
    }

    #[test]
    fn test_prediction_mode_grid_bounds() {
        let grid = PredictionModeGrid::new(
            1920,
            1080,
            16,
            16,
            vec![Some(PredictionMode::DcPred); (120 * 68) as usize],
        );

        // Valid bounds
        assert!(grid.get(0, 0).is_some());
        assert!(grid.get(119, 67).is_some());

        // Out of bounds
        assert!(grid.get(120, 0).is_none());
        assert!(grid.get(0, 68).is_none());
    }

    const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../../../test_data/av1_test.ivf");

    fn find_seq_header_bytes(frames: &[crate::ivf::IvfFrame]) -> Option<Vec<u8>> {
        for frame in frames.iter().take(8) {
            let mut iter = crate::obu::ObuIterator::new(&frame.data);
            while let Some(Ok(found)) = iter.next_obu_with_offset() {
                if found.obu.header.obu_type == crate::obu::ObuType::SequenceHeader {
                    return Some(frame.data[found.offset..found.offset + found.consumed].to_vec());
                }
            }
        }
        None
    }

    /// `delta_q_enabled` must come from the exact frame-header parse, not the approximate one.
    ///
    /// An earlier version of this module's tests asserted that a fixture frame with
    /// `delta_q_enabled = true` parses differently with the flag off, from a count of 82 such
    /// frames out of 250. That count came from `parse_frame_header_basic`, which only approximates
    /// everything after the quantizer fields for non-key frames and reported `delta_q_present` for
    /// bits that belong to other syntax elements. The exact parse (`parse_frame_header_full`) says
    /// none of the fixture's frames use delta-q at all, and a dav1d 1.5.1 trace of frame 0 agrees
    /// (no `Post-delta_q` after the first block's `Post-cdef_idx`, although that block sits at a
    /// superblock origin, where `delta_q` is read whenever it is present). Passing the wrong
    /// `true` made the parser read `delta_q` symbols that were never coded, desyncing every later
    /// symbol of those frames.
    ///
    /// The `delta_q` read itself stays covered by the synthetic goldens
    /// (`delta-q-lf`, `delta-q-lf-sb128`, ...), which turn the flag on explicitly.
    #[test]
    fn real_fixture_has_no_delta_q_frames_per_the_exact_header_parse() {
        let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
        let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");

        let mut parsed_frames = 0;
        for (idx, frame) in frames.iter().enumerate() {
            let obu_data: Vec<u8> = [seq_bytes.as_slice(), frame.data.as_slice()].concat();
            let Ok(parsed) = super::super::parser::ParsedFrame::parse(&obu_data) else {
                continue;
            };
            parsed_frames += 1;
            assert!(
                !parsed.delta_q_enabled,
                "frame {idx}: delta_q_enabled must follow the exact header parse"
            );
        }
        assert!(parsed_frames > 200, "only {parsed_frames} frames parsed");
    }
}
