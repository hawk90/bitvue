//! `CodingUnit::bits` / `symbols`: the entropy-coded cost the parser attributes to each CU.

use super::super::parser::ParsedFrame;
use super::parse_all_coding_units;
use super::test_support::*;

const TESTSRC2_IVF: &[u8] = include_bytes!("../../../../../test_data/av1_aomenc_testsrc2.ivf");

fn frame_0(ivf: &[u8]) -> ParsedFrame {
    let (_hdr, frames) = crate::ivf::parse_ivf_frames(ivf).unwrap();
    let seq_bytes = find_seq_header_bytes(&frames).expect("clip has a sequence header");
    let obu_data: Vec<u8> = [seq_bytes.as_slice(), frames[0].data.as_slice()].concat();
    ParsedFrame::parse(&obu_data).unwrap()
}

/// Every bit of a tile belongs to a coding unit, a partition symbol (read outside
/// `parse_coding_unit`, charged to the CUs under it by area) or the trailing padding, so the CU
/// costs including partition shares come to the whole tile minus at most a few bytes of padding.
#[test]
fn cu_bits_account_for_nearly_all_of_the_tile() {
    for (name, ivf) in [
        ("av1_test.ivf", AV1_IVF_FIXTURE),
        ("av1_aomenc_testsrc2.ivf", TESTSRC2_IVF),
    ] {
        let parsed = frame_0(ivf);
        let cus = parse_all_coding_units(&parsed).unwrap();
        let tile_bits = parsed.tile_data.len() as f64 * 8.0;
        let own_bits: f64 = cus.iter().map(|cu| cu.bits).sum();
        let cu_bits: f64 = cus.iter().map(|cu| cu.bits + cu.partition_bits).sum();
        let ratio = cu_bits / tile_bits;
        eprintln!(
            "{name}: {} CUs, own_bits={own_bits:.0} cu_bits={cu_bits:.0} tile_bits={tile_bits:.0} ratio={ratio:.3}",
            cus.len()
        );
        assert!(
            cu_bits <= tile_bits + 1e-6,
            "{name}: CU bits {cu_bits} exceed the tile's {tile_bits}"
        );
        assert!(
            tile_bits - cu_bits < 64.0,
            "{name}: {:.1} tile bits are charged to no CU ({ratio:.4})",
            tile_bits - cu_bits
        );
    }
}

#[test]
fn every_cu_has_a_nonnegative_cost_and_a_symbol_count() {
    let parsed = frame_0(AV1_IVF_FIXTURE);
    let cus = parse_all_coding_units(&parsed).unwrap();
    assert!(!cus.is_empty());
    for cu in cus.iter() {
        assert!(
            cu.bits >= 0.0 && cu.bits.is_finite(),
            "CU at ({},{}) bits {}",
            cu.x,
            cu.y,
            cu.bits
        );
        assert!(cu.symbols > 0, "CU at ({},{}) read no symbols", cu.x, cu.y);
    }
}

/// Frame sizes that are not a multiple of 8 (194x72, 114x136) put blocks across the frame edge,
/// where a partition node's symbol covers only the part of it inside the frame. The shares must
/// still add up: no frame may charge more than its tile holds, and a verified frame (decoded to
/// the end of its tile) leaves only the trailing padding uncharged -- inter frames included.
#[test]
fn partition_shares_add_up_on_frames_with_blocks_across_the_edge() {
    use crate::frame_header_full::RefFrameState;
    use crate::overlay_extraction::{frame_provenance, Provenance};
    let mut verified_frames = 0;
    for (name, ivf) in [
        (
            "194x72",
            &include_bytes!("../../../../../test_data/av1_aomenc_194x72.ivf")[..],
        ),
        (
            "114x136",
            &include_bytes!("../../../../../test_data/av1_aomenc_114x136_palette.ivf")[..],
        ),
    ] {
        let (_hdr, packets) = crate::ivf::parse_ivf_frames(ivf).unwrap();
        let seq = find_seq_header_bytes(&packets).expect("sequence header");
        let mut state = RefFrameState::new();
        for (i, p) in packets.iter().enumerate() {
            let data = [seq.as_slice(), p.data.as_slice()].concat();
            let parsed = ParsedFrame::parse_with_ref_state(&data, &mut state).unwrap();
            let Ok(cus) = parse_all_coding_units(&parsed) else {
                continue;
            };
            let tile_bits = parsed.tile_data.len() as f64 * 8.0;
            let charged: f64 = cus.iter().map(|cu| cu.bits + cu.partition_bits).sum();
            assert!(
                charged <= tile_bits + 1e-6,
                "{name} frame {i}: charged {charged} > tile {tile_bits}"
            );
            let prov = frame_provenance(&parsed);
            if prov == Provenance::Verified {
                verified_frames += 1;
                assert!(
                    tile_bits - charged < 64.0,
                    "{name} frame {i}: {:.1} bits charged to no CU",
                    tile_bits - charged
                );
            }
        }
    }
    assert!(
        verified_frames == 3,
        "only {verified_frames} verified frames"
    );
}
