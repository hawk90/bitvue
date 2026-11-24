//! The bit grid on real frames: spreading coding-unit costs over cells must neither lose nor invent
//! any (#458, #488). Every frame of the fixture, including inter frames and the odd-sized clips
//! used by the tile-accounting tests.

use bitvue_av1_codec::frame_header_full::RefFrameState;
use bitvue_av1_codec::obu::{ObuIterator, ObuType};
use bitvue_av1_codec::overlay_extraction::{
    extract_bit_grid_from_parsed, parse_all_coding_units, ParsedFrame,
};
use bitvue_av1_codec::parse_ivf_frames;

fn check_clip(path: &str, expect_bits: bool) {
    let data = std::fs::read(format!(
        "{}/../../test_data/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let (_h, packets) = parse_ivf_frames(&data).unwrap();
    let seq = packets
        .iter()
        .find_map(|p| {
            let mut obus = ObuIterator::new(&p.data);
            while let Some(Ok(f)) = obus.next_obu_with_offset() {
                if f.obu.header.obu_type == ObuType::SequenceHeader {
                    return Some(p.data[f.offset..f.offset + f.consumed].to_vec());
                }
            }
            None
        })
        .unwrap();
    let mut state = RefFrameState::new();
    let mut any_bits = false;
    for (i, p) in packets.iter().enumerate() {
        let bytes = [seq.as_slice(), p.data.as_slice()].concat();
        let parsed = ParsedFrame::parse_with_ref_state(&bytes, &mut state).unwrap();
        let Ok(units) = parse_all_coding_units(&parsed) else {
            continue;
        };
        let grid = extract_bit_grid_from_parsed(&parsed).unwrap();
        let (w, h) = (parsed.dimensions.width, parsed.dimensions.height);
        let mut cell_bits = 0.0;
        for gy in 0..grid.grid_h {
            for gx in 0..grid.grid_w {
                let visible = (w.min((gx + 1) * grid.block_w) - gx * grid.block_w) as f64
                    * (h.min((gy + 1) * grid.block_h) - gy * grid.block_h) as f64;
                cell_bits += grid.bits_per_pixel[(gy * grid.grid_w + gx) as usize] * visible;
            }
        }
        let unit_bits: f64 = units.iter().map(|u| u.bits + u.partition_bits).sum();
        assert!(
            (cell_bits - unit_bits).abs() <= 1e-6 * unit_bits.max(1.0),
            "{path} frame {i}: cells hold {cell_bits}, coding units {unit_bits}"
        );
        assert!(grid
            .bits_per_pixel
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0));
        assert!(grid
            .symbols_per_bit
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0));
        any_bits |= unit_bits > 0.0;
    }
    assert_eq!(any_bits, expect_bits, "{path}");
}

#[test]
fn cell_totals_equal_coding_unit_totals_on_real_clips() {
    check_clip("av1_test.ivf", true);
    check_clip("av1_aomenc_lossless_tiled.ivf", true);
    check_clip("av1_aomenc_intrabc_glyphs.ivf", true);
}

bitvue_test_alloc::install!();
