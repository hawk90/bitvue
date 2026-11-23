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
/// `parse_coding_unit`) or the trailing padding, so the CU bits must come to most of the tile but
/// can never exceed it. A real key frame has few partition symbols per bit, so "most" is tight.
#[test]
fn cu_bits_account_for_nearly_all_of_the_tile() {
    for (name, ivf) in [
        ("av1_test.ivf", AV1_IVF_FIXTURE),
        ("av1_aomenc_testsrc2.ivf", TESTSRC2_IVF),
    ] {
        let parsed = frame_0(ivf);
        let cus = parse_all_coding_units(&parsed).unwrap();
        let tile_bits = parsed.tile_data.len() as f64 * 8.0;
        let cu_bits: f64 = cus.iter().map(|cu| cu.bits).sum();
        let ratio = cu_bits / tile_bits;
        eprintln!(
            "{name}: {} CUs, cu_bits={cu_bits:.0} tile_bits={tile_bits:.0} ratio={ratio:.3}",
            cus.len()
        );
        assert!(
            cu_bits <= tile_bits,
            "{name}: CU bits {cu_bits} exceed the tile's {tile_bits}"
        );
        assert!(
            ratio > 0.95,
            "{name}: CU bits are only {ratio:.2} of the tile"
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
