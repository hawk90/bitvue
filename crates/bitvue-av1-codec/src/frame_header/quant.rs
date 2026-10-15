//! `quantization_params()` (5.9.12), `delta_q_params()` (5.9.17) and `delta_lf_params()` (5.9.18).

use crate::bitreader::BitReader;
use bitvue_engine::BitvueError;

/// Read a signed delta-Q value: 1-bit present flag, then su(7) if present.
/// Per AV1 spec Section 5.9.14 (read_delta_q).
pub(super) fn read_delta_q(reader: &mut BitReader) -> Result<Option<i8>, BitvueError> {
    let delta_coded = reader.read_bit()?;
    if delta_coded {
        // su(7): signed 7-bit value in [-64, 63]
        let val = reader.read_su(7)?;
        Ok(Some(val.clamp(-128, 127) as i8))
    } else {
        Ok(Some(0i8))
    }
}

/// `quantization_params()` (spec 5.9.12): the frame's base index and its five delta-Q terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuantizationParams {
    pub base_q_idx: u8,
    pub y_dc: i8,
    pub u_dc: i8,
    pub u_ac: i8,
    pub v_dc: i8,
    pub v_ac: i8,
}

impl QuantizationParams {
    /// Spec `CodedLossless` needs every delta to be zero (as well as `base_q_idx == 0`).
    pub(crate) fn all_deltas_zero(&self) -> bool {
        self.y_dc == 0 && self.u_dc == 0 && self.u_ac == 0 && self.v_dc == 0 && self.v_ac == 0
    }
}

/// Parse quantization_params() per AV1 spec Section 5.9.12.
///
/// `num_planes` and `separate_uv_delta_q` come from the sequence header color config; a stream
/// with `separate_uv_delta_q` (rav1e writes it) codes a `diff_uv_delta` flag before the U deltas
/// and, when set, separate V deltas, and a `qm_v` of its own. The V terms equal the U ones
/// otherwise. A monochrome stream has no chroma deltas at all.
pub(crate) fn parse_quantization_params(
    reader: &mut BitReader,
    num_planes: u8,
    separate_uv_delta_q: bool,
) -> Result<QuantizationParams, BitvueError> {
    let delta = |reader: &mut BitReader| -> Result<i8, BitvueError> {
        Ok(read_delta_q(reader)?.unwrap_or(0))
    };
    // base_q_idx  u(8)
    let base_q_idx = reader.read_bits(8)? as u8;
    let y_dc = delta(reader)?;
    let (mut u_dc, mut u_ac, mut v_dc, mut v_ac) = (0, 0, 0, 0);
    if num_planes > 1 {
        let diff_uv_delta = separate_uv_delta_q && reader.read_bit()?;
        u_dc = delta(reader)?;
        u_ac = delta(reader)?;
        if diff_uv_delta {
            v_dc = delta(reader)?;
            v_ac = delta(reader)?;
        } else {
            v_dc = u_dc;
            v_ac = u_ac;
        }
    }
    // using_qmatrix (1 bit), then qm_y, qm_u and -- only with separate U/V deltas -- qm_v.
    if reader.read_bit()? {
        reader.read_bits(4)?;
        reader.read_bits(4)?;
        if separate_uv_delta_q {
            reader.read_bits(4)?;
        }
    }
    Ok(QuantizationParams {
        base_q_idx,
        y_dc,
        u_dc,
        u_ac,
        v_dc,
        v_ac,
    })
}

/// Parse delta_q_params() per AV1 spec Section 5.9.17.
///
/// Returns `delta_q_present` flag.
pub(super) fn parse_delta_q_params(
    reader: &mut BitReader,
    base_q_idx: u8,
) -> Result<bool, BitvueError> {
    let delta_q_present = if base_q_idx > 0 {
        reader.read_bit()?
    } else {
        false
    };

    if delta_q_present {
        // delta_q_res  u(2)
        reader.read_bits(2)?;
    }

    Ok(delta_q_present)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packs `(value, width)` pairs MSB first, as the spec's `f(n)` elements are laid out.
    fn pack(fields: &[(u32, u8)]) -> Vec<u8> {
        let mut bits = Vec::new();
        for &(value, width) in fields {
            for i in (0..width).rev() {
                bits.push(((value >> i) & 1) as u8);
            }
        }
        let mut out = vec![0u8; bits.len().div_ceil(8)];
        for (i, bit) in bits.iter().enumerate() {
            out[i / 8] |= bit << (7 - (i % 8));
        }
        out
    }

    fn total_bits(fields: &[(u32, u8)]) -> u64 {
        fields.iter().map(|&(_, w)| u64::from(w)).sum()
    }

    /// The 7-bit two's-complement field of a value in `-64..=63`.
    fn su7(v: i32) -> (u32, u8) {
        ((v & 0x7f) as u32, 7)
    }

    /// `delta_coded = 1` plus the su(7) field.
    fn coded(v: i32) -> [(u32, u8); 2] {
        [(1, 1), su7(v)]
    }

    const UNCODED: (u32, u8) = (0, 1);

    /// Runs `parse` over exactly `fields`: it must consume every bit, and every shorter byte
    /// prefix (down to the empty one) must be an error.
    fn parse_exact<T>(
        fields: &[(u32, u8)],
        parse: impl Fn(&mut BitReader) -> Result<T, BitvueError>,
    ) -> T {
        let bytes = pack(fields);
        let mut reader = BitReader::new(&bytes);
        let value = parse(&mut reader).expect("parse");
        assert_eq!(reader.position(), total_bits(fields), "bits consumed");
        // The last byte holds bits of the last element only when the field count says so; every
        // prefix that drops a byte holding a consumed bit must fail.
        let needed = total_bits(fields).div_ceil(8) as usize;
        for cut in 0..needed {
            let mut short = BitReader::new(&bytes[..cut]);
            assert!(
                parse(&mut short).is_err(),
                "a {cut}-byte prefix of {needed} must fail"
            );
        }
        value
    }

    // ---- read_delta_q ----

    #[test]
    fn an_uncoded_delta_is_zero_and_costs_one_bit() {
        let v = parse_exact(&[UNCODED], read_delta_q);
        assert_eq!(v, Some(0));
        // The bits after the flag are not part of it.
        let bytes = pack(&[UNCODED, (0x7f, 7)]);
        let mut reader = BitReader::new(&bytes);
        assert_eq!(read_delta_q(&mut reader).unwrap(), Some(0));
        assert_eq!(reader.position(), 1);
    }

    #[test]
    fn every_coded_delta_in_the_su7_range_round_trips() {
        for v in -64..=63 {
            let got = parse_exact(&coded(v), read_delta_q);
            assert_eq!(got, Some(v as i8), "delta {v}");
        }
    }

    #[test]
    fn a_coded_zero_and_the_extremes() {
        assert_eq!(parse_exact(&coded(0), read_delta_q), Some(0));
        assert_eq!(parse_exact(&coded(-64), read_delta_q), Some(-64));
        assert_eq!(parse_exact(&coded(63), read_delta_q), Some(63));
        assert_eq!(parse_exact(&coded(-1), read_delta_q), Some(-1));
    }

    #[test]
    fn a_coded_delta_cut_short_is_an_error() {
        // Flag set with only 5 of the 7 value bits left in the data.
        let mut reader = BitReader::new(&[0b1111_1111]);
        reader.read_bits(2).unwrap();
        assert!(read_delta_q(&mut reader).is_err());
        // The same data with the flag clear does not need the value bits.
        let mut reader = BitReader::new(&[0b1100_0000]);
        reader.read_bits(2).unwrap();
        assert_eq!(read_delta_q(&mut reader).unwrap(), Some(0));
        // No data at all.
        assert!(read_delta_q(&mut BitReader::new(&[])).is_err());
    }

    // ---- parse_quantization_params ----

    fn quant(planes: u8, separate: bool, fields: &[(u32, u8)]) -> QuantizationParams {
        parse_exact(fields, |r| parse_quantization_params(r, planes, separate))
    }

    fn q(base: u8, y_dc: i8, u_dc: i8, u_ac: i8, v_dc: i8, v_ac: i8) -> QuantizationParams {
        QuantizationParams {
            base_q_idx: base,
            y_dc,
            u_dc,
            u_ac,
            v_dc,
            v_ac,
        }
    }

    #[test]
    fn a_monochrome_frame_has_a_base_index_and_a_luma_delta_only() {
        // base_q_idx, y_dc (uncoded), using_qmatrix = 0.
        let got = quant(1, false, &[(77, 8), UNCODED, (0, 1)]);
        assert_eq!(got, q(77, 0, 0, 0, 0, 0));
        // y_dc coded.
        let [a, b] = coded(-9);
        let got = quant(1, false, &[(200, 8), a, b, (0, 1)]);
        assert_eq!(got, q(200, -9, 0, 0, 0, 0));
        // separate_uv_delta_q does not matter without chroma planes: no diff_uv_delta bit.
        let got = quant(1, true, &[(5, 8), UNCODED, (0, 1)]);
        assert_eq!(got, q(5, 0, 0, 0, 0, 0));
    }

    #[test]
    fn the_base_index_covers_its_whole_range() {
        for base in [0, 1, 127, 128, 254, 255] {
            let got = quant(1, false, &[(base, 8), UNCODED, (0, 1)]);
            assert_eq!(got.base_q_idx, base as u8);
        }
    }

    #[test]
    fn chroma_deltas_are_read_in_dc_then_ac_order_and_v_copies_u() {
        // y_dc = 1, u_dc = 2, u_ac = 3 (all different so a swap shows).
        let mut fields = vec![(100, 8)];
        fields.extend(coded(1));
        fields.extend(coded(2));
        fields.extend(coded(3));
        fields.push((0, 1));
        let got = quant(3, false, &fields);
        assert_eq!(got, q(100, 1, 2, 3, 2, 3));
    }

    #[test]
    fn without_separate_uv_delta_q_there_is_no_diff_flag_even_if_the_bit_would_be_set() {
        // If a diff_uv_delta bit were wrongly read, the first chroma delta's flag would be eaten.
        let mut fields = vec![(100, 8), UNCODED];
        fields.extend(coded(4));
        fields.extend(coded(-5));
        fields.push((0, 1));
        let got = quant(3, false, &fields);
        assert_eq!(got, q(100, 0, 4, -5, 4, -5));
    }

    #[test]
    fn separate_uv_delta_q_without_a_difference_still_copies_u_to_v() {
        // diff_uv_delta = 0.
        let mut fields = vec![(100, 8), UNCODED, (0, 1)];
        fields.extend(coded(6));
        fields.extend(coded(7));
        fields.push((0, 1));
        let got = quant(3, true, &fields);
        assert_eq!(got, q(100, 0, 6, 7, 6, 7));
    }

    #[test]
    fn a_diff_uv_delta_reads_separate_v_deltas_in_dc_then_ac_order() {
        let mut fields = vec![(100, 8), UNCODED, (1, 1)];
        fields.extend(coded(6)); // u_dc
        fields.extend(coded(-7)); // u_ac
        fields.extend(coded(8)); // v_dc
        fields.extend(coded(-9)); // v_ac
        fields.push((0, 1));
        let got = quant(3, true, &fields);
        assert_eq!(got, q(100, 0, 6, -7, 8, -9));
    }

    #[test]
    fn uncoded_chroma_deltas_are_zero_and_a_zero_v_delta_stays_zero() {
        let got = quant(
            3,
            true,
            &[
                (10, 8),
                UNCODED,
                (1, 1),
                UNCODED,
                UNCODED,
                UNCODED,
                UNCODED,
                (0, 1),
            ],
        );
        assert_eq!(got, q(10, 0, 0, 0, 0, 0));
        // u coded but v explicitly uncoded: v is 0, not a copy.
        let mut fields = vec![(10, 8), UNCODED, (1, 1)];
        fields.extend(coded(3));
        fields.extend(coded(4));
        fields.extend([UNCODED, UNCODED, (0, 1)]);
        let got = quant(3, true, &fields);
        assert_eq!(got, q(10, 0, 3, 4, 0, 0));
    }

    #[test]
    fn the_quantizer_matrix_fields_are_skipped_whatever_their_value() {
        // using_qmatrix = 1: qm_y(4) qm_u(4), no qm_v without separate_uv_delta_q.
        let got = quant(
            3,
            false,
            &[
                (9, 8),
                UNCODED,
                UNCODED,
                UNCODED,
                (1, 1),
                (0xF, 4),
                (0x5, 4),
            ],
        );
        assert_eq!(got, q(9, 0, 0, 0, 0, 0));
        // With separate_uv_delta_q there is a qm_v of its own.
        let got = quant(
            3,
            true,
            &[
                (9, 8),
                UNCODED,
                (0, 1),
                UNCODED,
                UNCODED,
                (1, 1),
                (1, 4),
                (2, 4),
                (3, 4),
            ],
        );
        assert_eq!(got, q(9, 0, 0, 0, 0, 0));
        // Monochrome with a matrix: qm_y, qm_u (and qm_v only with separate_uv_delta_q).
        let got = quant(1, false, &[(9, 8), UNCODED, (1, 1), (3, 4), (4, 4)]);
        assert_eq!(got, q(9, 0, 0, 0, 0, 0));
        let got = quant(1, true, &[(9, 8), UNCODED, (1, 1), (3, 4), (4, 4), (5, 4)]);
        assert_eq!(got, q(9, 0, 0, 0, 0, 0));
    }

    #[test]
    fn the_matrix_flag_is_what_gates_the_matrix_fields() {
        // using_qmatrix = 0: parsing stops right there; a following byte is not consumed.
        let bytes = pack(&[(9, 8), UNCODED, (0, 1), (0xFF, 8)]);
        let mut reader = BitReader::new(&bytes);
        parse_quantization_params(&mut reader, 1, false).unwrap();
        assert_eq!(reader.position(), 10);
    }

    #[test]
    fn truncated_quantization_params_are_errors_at_every_cut() {
        let mut fields = vec![(100, 8), UNCODED, (1, 1)];
        fields.extend(coded(6));
        fields.extend(coded(-7));
        fields.extend(coded(8));
        fields.extend(coded(-9));
        fields.extend([(1, 1), (1, 4), (2, 4), (3, 4)]);
        // parse_exact tries every shorter byte prefix.
        let got = quant(3, true, &fields);
        assert_eq!(got, q(100, 0, 6, -7, 8, -9));
    }

    #[test]
    fn random_bytes_never_panic_and_keep_the_reader_inside_the_data() {
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..2000 {
            let len = (next() % 12) as usize;
            let data: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            for (planes, separate) in [(1, false), (1, true), (3, false), (3, true)] {
                let mut reader = BitReader::new(&data);
                let _ = parse_quantization_params(&mut reader, planes, separate);
                assert!(reader.position() <= data.len() as u64 * 8);
            }
        }
    }

    #[test]
    fn all_deltas_zero_looks_at_every_delta_separately() {
        assert!(q(0, 0, 0, 0, 0, 0).all_deltas_zero());
        // The base index is not a delta.
        assert!(q(200, 0, 0, 0, 0, 0).all_deltas_zero());
        assert!(!q(0, 1, 0, 0, 0, 0).all_deltas_zero());
        assert!(!q(0, 0, 1, 0, 0, 0).all_deltas_zero());
        assert!(!q(0, 0, 0, -1, 0, 0).all_deltas_zero());
        assert!(!q(0, 0, 0, 0, 1, 0).all_deltas_zero());
        assert!(!q(0, 0, 0, 0, 0, -1).all_deltas_zero());
    }

    // ---- parse_delta_q_params ----

    #[test]
    fn no_delta_q_flag_is_read_for_a_lossless_base_index() {
        let bytes = [0xFF];
        let mut reader = BitReader::new(&bytes);
        assert!(!parse_delta_q_params(&mut reader, 0).unwrap());
        assert_eq!(reader.position(), 0, "base_q_idx 0 reads nothing");
        // Not even from an empty buffer.
        let mut reader = BitReader::new(&[]);
        assert!(!parse_delta_q_params(&mut reader, 0).unwrap());
    }

    #[test]
    fn the_delta_q_flag_gates_the_two_bit_resolution() {
        for base in [1, 2, 128, 255] {
            // present = 0: one bit.
            let v = parse_exact(&[(0, 1)], |r| parse_delta_q_params(r, base));
            assert!(!v, "base {base}");
            // present = 1: flag plus delta_q_res(2), whatever its value.
            for res in 0..4 {
                let v = parse_exact(&[(1, 1), (res, 2)], |r| parse_delta_q_params(r, base));
                assert!(v, "base {base} res {res}");
            }
        }
    }

    #[test]
    fn the_delta_q_res_bits_are_consumed_and_not_the_ones_after() {
        let bytes = pack(&[(1, 1), (3, 2), (0xFF, 8)]);
        let mut reader = BitReader::new(&bytes);
        assert!(parse_delta_q_params(&mut reader, 10).unwrap());
        assert_eq!(reader.position(), 3);
    }
}
