//! Loop filter, delta LF, CDEF and loop-restoration parameters (spec 5.9.11-5.9.20).

use super::*;

const RESTORATION_TILESIZE_MAX: u32 = 256;
/// `delta_lf_params()` (spec 5.9.14) -- real `delta_lf_present`/`delta_lf_multi` (`delta_lf_res` is
/// consumed for bitstream position but not retained: it only scales the *decoded* per-CU
/// `delta_lf` magnitude, never affects tile-data bit position -- same reasoning as this crate not
/// tracking `delta_q_res` either).
pub(super) fn parse_delta_lf_params(
    reader: &mut BitReader,
    delta_q_present: bool,
    allow_intrabc: bool,
) -> Result<(bool, bool)> {
    if !delta_q_present {
        return Ok((false, false));
    }
    let delta_lf_present = if !allow_intrabc {
        reader.read_bit()?
    } else {
        false
    };
    let delta_lf_multi = if delta_lf_present {
        reader.read_bits(2)?; // delta_lf_res
        reader.read_bit()?
    } else {
        false
    };
    Ok((delta_lf_present, delta_lf_multi))
}

/// Parse `loop_filter_params()` per AV1 spec Section 5.9.11. Unlike `skip_loop_filter_params`
/// (`crate::frame_header`), this retains every value -- needed by `get_deblocking_analysis` to
/// report real per-plane filter levels/sharpness/deltas alongside boundary-strength derivation
/// (see `overlay_extraction::deblocking`'s module doc for how BS itself is computed from
/// coding-unit data, independent of these values).
///
/// Note: the spec's `loop_filter_delta_update` only rewrites deltas that are explicitly signaled
/// (`update_ref_delta`/`update_mode_delta`); un-signaled deltas keep their *previous frame's*
/// value (`PrevRefDeltas`/`PrevModeDeltas`, spec 7.20). This parser has no cross-frame delta
/// state (unlike `RefFrameState`'s order-hint tracking, which is required for correct bit
/// alignment) -- un-signaled deltas here reset to 0 rather than carrying forward. This only
/// affects the *reported* delta values, never bit consumption (every delta's presence bit is
/// still read correctly either way).
pub(super) fn parse_loop_filter_params(
    reader: &mut BitReader,
    num_planes: u8,
    coded_lossless: bool,
    allow_intrabc: bool,
) -> Result<LoopFilterInfo> {
    if coded_lossless || allow_intrabc {
        return Ok(LoopFilterInfo::default());
    }
    let mut level = [0u8; 4];
    level[0] = reader.read_bits(6)? as u8;
    level[1] = reader.read_bits(6)? as u8;
    if num_planes > 1 && (level[0] != 0 || level[1] != 0) {
        level[2] = reader.read_bits(6)? as u8;
        level[3] = reader.read_bits(6)? as u8;
    }
    let sharpness = reader.read_bits(3)? as u8;
    let delta_enabled = reader.read_bit()?;
    let mut ref_deltas = [0i8; NUM_REF_FRAMES];
    let mut mode_deltas = [0i8; 2];
    if delta_enabled {
        let delta_update = reader.read_bit()?;
        if delta_update {
            for delta in ref_deltas.iter_mut() {
                if reader.read_bit()? {
                    *delta = reader.read_su(7)? as i8;
                }
            }
            for delta in mode_deltas.iter_mut() {
                if reader.read_bit()? {
                    *delta = reader.read_su(7)? as i8;
                }
            }
        }
    }
    Ok(LoopFilterInfo {
        level,
        sharpness,
        delta_enabled,
        ref_deltas,
        mode_deltas,
    })
}

pub(super) fn parse_cdef_params(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    coded_lossless: bool,
    allow_intrabc: bool,
) -> Result<CdefInfo> {
    if coded_lossless || allow_intrabc || !seq.enable_cdef {
        return Ok(CdefInfo {
            enabled: false,
            damping: 3,
            ..Default::default()
        });
    }
    let damping = (reader.read_bits(2)? + 3) as u8;
    let cdef_bits = reader.read_bits(2)?;
    let mut y_primary_strength = 0u8;
    let mut y_secondary_strength = 0u8;
    let mut uv_primary_strength = 0u8;
    let mut uv_secondary_strength = 0u8;
    for i in 0..(1u32 << cdef_bits) {
        let y_pri = reader.read_bits(4)? as u8;
        let mut y_sec = reader.read_bits(2)? as u8;
        if y_sec == 3 {
            y_sec += 1;
        }
        let uv_pri = reader.read_bits(4)? as u8;
        let mut uv_sec = reader.read_bits(2)? as u8;
        if uv_sec == 3 {
            uv_sec += 1;
        }
        if i == 0 {
            y_primary_strength = y_pri;
            y_secondary_strength = y_sec;
            uv_primary_strength = uv_pri;
            uv_secondary_strength = uv_sec;
        }
    }
    Ok(CdefInfo {
        enabled: true,
        damping,
        y_primary_strength,
        y_secondary_strength,
        uv_primary_strength,
        uv_secondary_strength,
        bits: cdef_bits as u8,
    })
}

pub(super) fn parse_lr_params(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    all_lossless: bool,
    allow_intrabc: bool,
) -> Result<LoopRestorationInfo> {
    if all_lossless || allow_intrabc || !seq.enable_restoration {
        return Ok(LoopRestorationInfo {
            enabled: false,
            ..Default::default()
        });
    }
    // Remap_Lr_Type (AV1 spec 5.9.20): the 2-bit code is not the enum order.
    const REMAP_LR_TYPE: [LoopRestorationType; 4] = [
        LoopRestorationType::None,
        LoopRestorationType::Switchable,
        LoopRestorationType::Wiener,
        LoopRestorationType::SgrProj,
    ];
    let num_planes = seq.color_config.num_planes;
    let mut types = [LoopRestorationType::None; 3];
    let mut uses_lr = false;
    let mut uses_chroma_lr = false;
    for (i, slot) in types.iter_mut().enumerate().take(num_planes as usize) {
        let code = reader.read_bits(2)?;
        let t = REMAP_LR_TYPE[code as usize & 0x3];
        *slot = t;
        if t != LoopRestorationType::None {
            uses_lr = true;
            if i > 0 {
                uses_chroma_lr = true;
            }
        }
    }
    let mut unit_size = RESTORATION_TILESIZE_MAX;
    let mut uv_unit_size = unit_size;
    if uses_lr {
        let mut lr_unit_shift: u32 = if seq.use_128x128_superblock {
            1 + u32::from(reader.read_bit()?)
        } else {
            let shift = reader.read_bit()?;
            if shift {
                1 + u32::from(reader.read_bit()?)
            } else {
                0
            }
        };
        lr_unit_shift = lr_unit_shift.min(2);
        unit_size = RESTORATION_TILESIZE_MAX >> (2 - lr_unit_shift);
        uv_unit_size = unit_size;
        if seq.color_config.subsampling_x && seq.color_config.subsampling_y && uses_chroma_lr {
            // lr_uv_shift
            uv_unit_size >>= u32::from(reader.read_bit()?);
        }
    }
    Ok(LoopRestorationInfo {
        enabled: uses_lr,
        unit_size,
        uv_unit_size,
        y_type: types[0],
        u_type: if num_planes > 1 {
            types[1]
        } else {
            LoopRestorationType::None
        },
        v_type: if num_planes > 1 {
            types[2]
        } else {
            LoopRestorationType::None
        },
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::base_seq;
    use super::*;

    /// MSB-first bit writer; the reader is checked for its exact end position, so no padding
    /// beyond the last byte boundary is relied on.
    struct Bits(Vec<bool>);

    impl Bits {
        fn new() -> Self {
            Bits(Vec::new())
        }
        fn put(&mut self, value: u32, width: u8) -> &mut Self {
            for i in (0..width).rev() {
                self.0.push((value >> i) & 1 == 1);
            }
            self
        }
        /// su(7): 7-bit two's complement.
        fn su7(&mut self, value: i32) -> &mut Self {
            self.put((value & 0x7F) as u32, 7)
        }
        fn len(&self) -> u64 {
            self.0.len() as u64
        }
        fn bytes(&self) -> Vec<u8> {
            let mut out = vec![0u8; self.0.len().div_ceil(8) + 1];
            for (i, &b) in self.0.iter().enumerate() {
                if b {
                    out[i / 8] |= 1 << (7 - i % 8);
                }
            }
            out
        }
    }

    // --- delta_lf_params -----------------------------------------------------

    #[test]
    fn delta_lf_without_delta_q_reads_nothing() {
        let data = [0xFFu8; 2];
        let mut r = BitReader::new(&data);
        assert_eq!(
            parse_delta_lf_params(&mut r, false, false).unwrap(),
            (false, false)
        );
        assert_eq!(r.position(), 0);
    }

    #[test]
    fn delta_lf_with_intrabc_reads_nothing_even_when_delta_q_is_present() {
        let data = [0xFFu8; 2];
        let mut r = BitReader::new(&data);
        assert_eq!(
            parse_delta_lf_params(&mut r, true, true).unwrap(),
            (false, false)
        );
        assert_eq!(r.position(), 0);
    }

    #[test]
    fn delta_lf_present_zero_stops_after_one_bit() {
        let mut b = Bits::new();
        b.put(0, 1).put(0b111, 3);
        let data = b.bytes();
        let mut r = BitReader::new(&data);
        assert_eq!(
            parse_delta_lf_params(&mut r, true, false).unwrap(),
            (false, false)
        );
        assert_eq!(r.position(), 1);
    }

    #[test]
    fn delta_lf_present_reads_res_then_multi() {
        for multi in [0u32, 1] {
            let mut b = Bits::new();
            b.put(1, 1).put(0b10, 2).put(multi, 1).put(0b1111, 4);
            let data = b.bytes();
            let mut r = BitReader::new(&data);
            assert_eq!(
                parse_delta_lf_params(&mut r, true, false).unwrap(),
                (true, multi == 1)
            );
            assert_eq!(r.position(), 4, "multi={multi}");
        }
    }

    #[test]
    fn delta_lf_truncated_after_the_present_bit_is_an_error() {
        let data = [0x80u8];
        let mut r = BitReader::new(&data[..1]);
        // present = 1 leaves 7 bits: res + multi fit; a one-byte buffer cut to 0 bits does not.
        assert!(parse_delta_lf_params(&mut r, true, false).is_ok());
        let mut r = BitReader::new(&[]);
        assert!(parse_delta_lf_params(&mut r, true, false).is_err());
    }

    // --- loop_filter_params --------------------------------------------------

    fn lf(b: &Bits, planes: u8) -> (Result<LoopFilterInfo>, u64) {
        let data = b.bytes();
        let mut r = BitReader::new(&data);
        let result = parse_loop_filter_params(&mut r, planes, false, false);
        (result, r.position())
    }

    #[test]
    fn lossless_or_intrabc_frames_have_no_loop_filter_bits() {
        let data = [0xFFu8; 4];
        for (lossless, intrabc) in [(true, false), (false, true), (true, true)] {
            let mut r = BitReader::new(&data);
            let info = parse_loop_filter_params(&mut r, 3, lossless, intrabc).unwrap();
            assert_eq!(
                format!("{info:?}"),
                format!("{:?}", LoopFilterInfo::default())
            );
            assert_eq!(r.position(), 0);
        }
    }

    #[test]
    fn chroma_levels_are_read_only_when_a_luma_level_is_nonzero_and_there_is_chroma() {
        // (level0, level1, planes, chroma levels are read)
        for (l0, l1, planes, chroma) in [
            (0u32, 0u32, 3u8, false),
            (5, 0, 3, true),
            (0, 7, 3, true),
            (63, 63, 3, true),
            (5, 7, 1, false),
        ] {
            let mut b = Bits::new();
            b.put(l0, 6).put(l1, 6);
            if chroma {
                b.put(21, 6).put(42, 6);
            }
            b.put(0b101, 3).put(0, 1); // sharpness, delta_enabled
            let (info, pos) = lf(&b, planes);
            let info = info.unwrap();
            assert_eq!(pos, b.len(), "{l0},{l1},{planes}");
            assert_eq!(info.level[0], l0 as u8);
            assert_eq!(info.level[1], l1 as u8);
            assert_eq!(info.level[2], if chroma { 21 } else { 0 });
            assert_eq!(info.level[3], if chroma { 42 } else { 0 });
            assert_eq!(info.sharpness, 5);
            assert!(!info.delta_enabled);
        }
    }

    #[test]
    fn deltas_are_only_read_when_enabled_and_updated() {
        // delta_enabled = 0: nothing follows
        let mut b = Bits::new();
        b.put(1, 6).put(1, 6).put(0, 12).put(0, 3).put(0, 1);
        let (info, pos) = lf(&b, 3);
        assert_eq!(pos, b.len());
        assert!(!info.unwrap().delta_enabled);

        // delta_enabled = 1, delta_update = 0: one more bit, deltas stay at zero
        let mut b = Bits::new();
        b.put(1, 6)
            .put(1, 6)
            .put(0, 12)
            .put(0, 3)
            .put(1, 1)
            .put(0, 1);
        let (info, pos) = lf(&b, 3);
        let info = info.unwrap();
        assert_eq!(pos, b.len());
        assert!(info.delta_enabled);
        assert_eq!(info.ref_deltas, [0; 8]);
        assert_eq!(info.mode_deltas, [0; 2]);
    }

    #[test]
    fn signalled_deltas_replace_only_their_own_slot_including_negative_and_extreme_values() {
        let refs: [Option<i32>; 8] = [
            Some(1),
            None,
            Some(-1),
            Some(63),
            None,
            Some(-64),
            None,
            Some(0),
        ];
        let modes: [Option<i32>; 2] = [None, Some(-33)];
        let mut b = Bits::new();
        b.put(1, 6)
            .put(1, 6)
            .put(0, 12)
            .put(0, 3)
            .put(1, 1)
            .put(1, 1);
        for d in refs.iter().chain(modes.iter()) {
            match d {
                Some(v) => {
                    b.put(1, 1).su7(*v);
                }
                None => {
                    b.put(0, 1);
                }
            }
        }
        let (info, pos) = lf(&b, 3);
        let info = info.unwrap();
        assert_eq!(pos, b.len());
        assert_eq!(info.ref_deltas, [1, 0, -1, 63, 0, -64, 0, 0]);
        assert_eq!(info.mode_deltas, [0, -33]);
    }

    #[test]
    fn a_header_cut_inside_the_loop_filter_is_an_error_at_every_length() {
        let mut b = Bits::new();
        b.put(9, 6)
            .put(8, 6)
            .put(7, 6)
            .put(6, 6)
            .put(3, 3)
            .put(1, 1)
            .put(1, 1);
        for _ in 0..10 {
            b.put(1, 1).su7(-5);
        }
        let full = b.bytes();
        let needed = b.len() as usize;
        assert!(parse_loop_filter_params(&mut BitReader::new(&full), 3, false, false).is_ok());
        // `bytes()` pads one spare byte, so cut by whole bytes below what is needed
        for cut in 0..needed / 8 {
            let r = parse_loop_filter_params(&mut BitReader::new(&full[..cut]), 3, false, false);
            assert!(r.is_err(), "{cut} bytes");
        }
    }

    // --- cdef_params ----------------------------------------------------------

    #[test]
    fn cdef_is_off_and_reads_nothing_when_lossless_intrabc_or_disabled_in_the_sequence() {
        let data = [0xFFu8; 8];
        let mut no_cdef = base_seq();
        no_cdef.enable_cdef = false;
        let on = base_seq();
        for (seq, lossless, intrabc) in [
            (&on, true, false),
            (&on, false, true),
            (&no_cdef, false, false),
        ] {
            let mut r = BitReader::new(&data);
            let info = parse_cdef_params(&mut r, seq, lossless, intrabc).unwrap();
            assert!(!info.enabled);
            assert_eq!(info.damping, 3);
            assert_eq!(r.position(), 0);
        }
    }

    #[test]
    fn cdef_reads_two_to_the_bits_strength_sets_and_keeps_the_first() {
        let seq = base_seq();
        for cdef_bits in 0..=3u32 {
            let mut b = Bits::new();
            b.put(2, 2).put(cdef_bits, 2); // damping 5
            for i in 0..(1u32 << cdef_bits) {
                // first set: y 9/1, uv 4/2; later sets differ in every field
                let (yp, ys, up, us) = if i == 0 { (9, 1, 4, 2) } else { (15, 2, 14, 1) };
                b.put(yp, 4).put(ys, 2).put(up, 4).put(us, 2);
            }
            let data = b.bytes();
            let mut r = BitReader::new(&data);
            let info = parse_cdef_params(&mut r, &seq, false, false).unwrap();
            assert_eq!(r.position(), b.len(), "cdef_bits={cdef_bits}");
            assert!(info.enabled);
            assert_eq!(info.damping, 5);
            assert_eq!(info.bits as u32, cdef_bits);
            assert_eq!((info.y_primary_strength, info.y_secondary_strength), (9, 1));
            assert_eq!(
                (info.uv_primary_strength, info.uv_secondary_strength),
                (4, 2)
            );
        }
    }

    #[test]
    fn a_secondary_strength_of_three_means_four() {
        let seq = base_seq();
        let mut b = Bits::new();
        b.put(0, 2).put(1, 2); // damping 3, two sets
        b.put(1, 4).put(3, 2).put(2, 4).put(3, 2); // first set: both secondaries = 3
        b.put(0, 4).put(3, 2).put(0, 4).put(3, 2); // second set
        let data = b.bytes();
        let mut r = BitReader::new(&data);
        let info = parse_cdef_params(&mut r, &seq, false, false).unwrap();
        assert_eq!(info.y_secondary_strength, 4);
        assert_eq!(info.uv_secondary_strength, 4);
        assert_eq!(info.damping, 3);
        assert_eq!(r.position(), b.len());
    }

    #[test]
    fn cdef_cut_short_is_an_error() {
        let seq = base_seq();
        let mut b = Bits::new();
        b.put(0, 2).put(3, 2); // eight sets promised
        for _ in 0..8 {
            b.put(0, 12);
        }
        let full = b.bytes();
        for cut in 0..b.len() as usize / 8 {
            let r = parse_cdef_params(&mut BitReader::new(&full[..cut]), &seq, false, false);
            assert!(r.is_err(), "{cut} bytes");
        }
    }

    // --- lr_params ------------------------------------------------------------

    fn lr(b: &Bits, seq: &SequenceHeader) -> (LoopRestorationInfo, u64) {
        let data = b.bytes();
        let mut r = BitReader::new(&data);
        let info = parse_lr_params(&mut r, seq, false, false).unwrap();
        (info, r.position())
    }

    #[test]
    fn restoration_is_off_and_reads_nothing_when_lossless_intrabc_or_disabled_in_the_sequence() {
        let data = [0xFFu8; 4];
        let mut off = base_seq();
        off.enable_restoration = false;
        let on = base_seq();
        for (seq, lossless, intrabc) in
            [(&on, true, false), (&on, false, true), (&off, false, false)]
        {
            let mut r = BitReader::new(&data);
            let info = parse_lr_params(&mut r, seq, lossless, intrabc).unwrap();
            assert!(!info.enabled);
            assert_eq!(r.position(), 0);
        }
    }

    #[test]
    fn no_plane_using_restoration_reads_only_the_type_codes() {
        let seq = base_seq();
        let mut b = Bits::new();
        b.put(0, 2).put(0, 2).put(0, 2);
        let (info, pos) = lr(&b, &seq);
        assert_eq!(pos, 6);
        assert!(!info.enabled);
        assert_eq!(info.unit_size, 256);
        assert_eq!(info.uv_unit_size, 256);
        assert_eq!(info.y_type, LoopRestorationType::None);
    }

    #[test]
    fn the_unit_size_shift_reads_one_or_two_bits_depending_on_the_superblock_size() {
        // (128x128 superblock, shift bits, expected luma unit size)
        let cases: [(bool, &[u32], u32); 5] = [
            (false, &[0], 64),
            (false, &[1, 0], 128),
            (false, &[1, 1], 256),
            (true, &[0], 128),
            (true, &[1], 256),
        ];
        for (sb128, shift_bits, size) in cases {
            let mut seq = base_seq();
            seq.use_128x128_superblock = sb128;
            // luma uses WIENER (code 2); chroma none, so no uv shift follows
            let mut b = Bits::new();
            b.put(2, 2).put(0, 2).put(0, 2);
            for &s in shift_bits {
                b.put(s, 1);
            }
            let (info, pos) = lr(&b, &seq);
            assert_eq!(pos, b.len(), "sb128={sb128} {shift_bits:?}");
            assert_eq!(info.unit_size, size, "sb128={sb128} {shift_bits:?}");
            assert_eq!(info.uv_unit_size, size);
            assert!(info.enabled);
        }
    }

    #[test]
    fn the_uv_shift_is_read_only_for_420_with_chroma_restoration() {
        // (subsampling_x, subsampling_y, y type code, u type code, v type code, uv shift present)
        let cases = [
            (true, true, 0u32, 1u32, 0u32, true), // chroma only, 4:2:0
            (true, true, 2, 0, 0, false),         // luma only: no uv shift
            (false, false, 0, 2, 0, false),       // 4:4:4
            (true, false, 0, 2, 0, false),        // 4:2:2
            (true, true, 3, 3, 3, true),
        ];
        for (sx, sy, y, u, v, has_shift) in cases {
            for shift in [0u32, 1] {
                if !has_shift && shift == 1 {
                    continue;
                }
                let mut seq = base_seq();
                seq.color_config.subsampling_x = sx;
                seq.color_config.subsampling_y = sy;
                seq.use_128x128_superblock = true;
                let mut b = Bits::new();
                b.put(y, 2).put(u, 2).put(v, 2).put(1, 1); // lr_unit_shift: 1 + 1 => 256
                if has_shift {
                    b.put(shift, 1);
                }
                let (info, pos) = lr(&b, &seq);
                assert_eq!(pos, b.len(), "{sx}{sy} {y}{u}{v} shift={shift}");
                assert_eq!(info.unit_size, 256);
                assert_eq!(info.uv_unit_size, 256 >> shift, "{sx}{sy} {y}{u}{v}");
            }
        }
    }

    #[test]
    fn a_monochrome_stream_reads_one_type_code_and_reports_no_chroma_types() {
        let mut seq = base_seq();
        seq.color_config.num_planes = 1;
        let mut b = Bits::new();
        b.put(3, 2).put(1, 1).put(0, 1); // SGRPROJ, lr_unit_shift 1 + 0 (64x64 sb: bit 1, then 0)
        let (info, pos) = lr(&b, &seq);
        assert_eq!(pos, b.len());
        assert_eq!(info.y_type, LoopRestorationType::SgrProj);
        assert_eq!(info.u_type, LoopRestorationType::None);
        assert_eq!(info.v_type, LoopRestorationType::None);
        assert_eq!(info.unit_size, 128);
    }

    #[test]
    fn restoration_cut_short_is_an_error() {
        let seq = base_seq();
        let mut b = Bits::new();
        b.put(1, 2)
            .put(2, 2)
            .put(3, 2)
            .put(1, 1)
            .put(1, 1)
            .put(1, 1);
        let full = b.bytes();
        for cut in 0..b.len() as usize / 8 {
            let r = parse_lr_params(&mut BitReader::new(&full[..cut]), &seq, false, false);
            assert!(r.is_err(), "{cut} bytes");
        }
    }
}
