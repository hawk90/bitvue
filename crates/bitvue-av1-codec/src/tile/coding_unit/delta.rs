//! Per-superblock side information read right after `skip`/`segment_id`, before any mode info:
//! `cdef_idx()` (spec 5.11.56) and the `delta_q`/`delta_lf` pair (spec 5.11.38). The real spec
//! (and dav1d's `decode_b`) reads these ahead of `intra_frame_mode_info()`/
//! `inter_frame_mode_info()`; reading them anywhere later desyncs the shared `SymbolDecoder`.

use crate::symbol::SymbolDecoder;
use crate::tile::{FrameCodingParams, MiRect, SuperblockCtx};
use bitvue_engine::Result;

/// `cdef_idx()` -- previously never read at all in this crate. Gated only on `!skip` (NOT on
/// whether CDEF is enabled: `cdef_bits` is already `0` then, making the read a no-op), and read at
/// most once per 64x64 CDEF unit within the superblock; `sb.cdef_idx` (`-1` = "not yet read",
/// reset per superblock) tracks that, mirroring dav1d's `cur_sb_cdef_idx_ptr`. A 128x128
/// superblock has 4 units (2x2) addressed by `idx`; a 64x64 one always uses unit `0` (NOT simply
/// `x4 & 16`, which would stay non-zero across successive 64-superblocks at frame-absolute MI
/// coordinates). A block spanning several units propagates its single value to all of them.
pub(super) fn read_cdef_idx(
    decoder: &mut SymbolDecoder,
    sb: &mut SuperblockCtx,
    mi: MiRect,
    skip: bool,
    cdef_bits: u8,
) -> Result<()> {
    if skip {
        return Ok(());
    }
    let idx = if sb.size4 == 32 {
        ((mi.x4 & 16) >> 4) + ((mi.y4 & 16) >> 3)
    } else {
        0
    } as usize;
    if sb.cdef_idx[idx] == -1 {
        let v = decoder.read_bools_n(cdef_bits as u32)? as i8;
        sb.cdef_idx[idx] = v;
        if mi.width > 16 {
            sb.cdef_idx[idx + 1] = v;
        }
        if mi.height > 16 {
            sb.cdef_idx[idx + 2] = v;
        }
        if mi.width == 32 && mi.height == 32 {
            sb.cdef_idx[idx + 3] = v;
        }
    }
    Ok(())
}

/// `read_delta_qindex()` + `read_delta_lf()`. Returns the QP in effect for this block.
///
/// Real spec gate (dav1d's `decode_b`): only at the first leaf visited within each superblock
/// (`x4/y4 == sb.x4/sb.y4`, always the top-left-most leaf given AV1's partition decode order), and
/// -- when that leaf covers the *whole* superblock -- only when it isn't `skip` (a skipped
/// full-superblock block has nothing to dequantize, so the encoder never signals a delta for it).
/// `delta_lf` bits are nested inside that same gate: one component per plane when
/// `delta_lf_multi` (4 normally, 2 for monochrome), otherwise a single shared component.
pub(super) fn read_delta_q_lf(
    decoder: &mut SymbolDecoder,
    sb: &SuperblockCtx,
    mi: MiRect,
    skip: bool,
    frame: &FrameCodingParams,
    current_qp: i16,
) -> i16 {
    let is_first_cu_in_sb = mi.x4 == sb.x4 && mi.y4 == sb.y4;
    let is_full_sb_size = mi.width == sb.size4 && mi.height == sb.size4;
    let have_delta = frame.delta_q_enabled && is_first_cu_in_sb && (!is_full_sb_size || !skip);
    if !have_delta {
        return current_qp;
    }
    let (x, y) = (mi.x4 * 4, mi.y4 * 4);

    let new_qp = match decoder.read_delta_q() {
        Ok(delta_q) => {
            let qp = (current_qp + delta_q).clamp(0, 255);
            tracing::debug!(
                "Delta Q applied at ({}, {}): {} + {} = {}",
                x,
                y,
                current_qp,
                delta_q,
                qp
            );
            qp
        }
        Err(e) => {
            tracing::warn!(
                "Failed to read delta Q at ({}, {}): {}, using current QP",
                x,
                y,
                e
            );
            current_qp
        }
    };

    if frame.delta_lf_present {
        let n_lfs = if frame.delta_lf_multi {
            if frame.tx_type_flags.mono_chrome {
                2
            } else {
                4
            }
        } else {
            1
        };
        for i in 0..n_lfs {
            let cdf_index = if frame.delta_lf_multi { i + 1 } else { 0 };
            if let Err(e) = decoder.read_delta_lf(cdf_index) {
                tracing::warn!("Failed to read delta_lf[{}] at ({}, {}): {}", i, x, y, e);
                break;
            }
        }
    }
    new_qp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mi(x4: u32, y4: u32, width: u32, height: u32) -> MiRect {
        MiRect {
            x4,
            y4,
            width,
            height,
        }
    }

    #[test]
    fn cdef_idx_is_read_once_and_spans_every_unit_the_block_covers() {
        let data = [0xA7u8; 64];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let mut sb = SuperblockCtx::new(0, 0, 128);
        read_cdef_idx(&mut decoder, &mut sb, mi(0, 0, 32, 32), false, 2).unwrap();
        let v = sb.cdef_idx[0];
        assert!(v >= 0);
        assert_eq!(sb.cdef_idx, [v; 4], "128x128 block covers all four units");

        // Already read: a later block must not consume more symbols or overwrite the value.
        let before = sb.cdef_idx;
        read_cdef_idx(&mut decoder, &mut sb, mi(0, 0, 32, 32), false, 2).unwrap();
        assert_eq!(sb.cdef_idx, before);
    }

    #[test]
    fn cdef_idx_covers_two_units_for_128x64_and_64x128() {
        let data = [0x5Cu8; 64];
        let mut sb = SuperblockCtx::new(0, 0, 128);
        read_cdef_idx(
            &mut SymbolDecoder::new(&data).unwrap(),
            &mut sb,
            mi(0, 0, 32, 16),
            false,
            2,
        )
        .unwrap();
        let v = sb.cdef_idx[0];
        assert_eq!(sb.cdef_idx, [v, v, -1, -1]);

        let mut sb = SuperblockCtx::new(0, 0, 128);
        read_cdef_idx(
            &mut SymbolDecoder::new(&data).unwrap(),
            &mut sb,
            mi(0, 0, 16, 32),
            false,
            2,
        )
        .unwrap();
        let v = sb.cdef_idx[0];
        assert_eq!(sb.cdef_idx, [v, -1, v, -1]);
    }

    #[test]
    fn cdef_idx_is_not_read_for_skipped_blocks() {
        let data = [0xA7u8; 64];
        let mut sb = SuperblockCtx::new(0, 0, 64);
        read_cdef_idx(
            &mut SymbolDecoder::new(&data).unwrap(),
            &mut sb,
            mi(0, 0, 16, 16),
            true,
            2,
        )
        .unwrap();
        assert_eq!(sb.cdef_idx, [-1; 4]);
    }

    // ---- read_delta_q_lf ----------------------------------------------------------------

    fn noise(seed: u64, len: usize) -> Vec<u8> {
        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x >> 24) as u8
            })
            .collect()
    }

    fn params(delta_q: bool, lf: bool, multi: bool, mono: bool) -> FrameCodingParams {
        let mut f = FrameCodingParams::for_tests();
        f.delta_q_enabled = delta_q;
        f.delta_lf_present = lf;
        f.delta_lf_multi = multi;
        f.tx_type_flags.mono_chrome = mono;
        f
    }

    /// What the decoder yields next, as a fingerprint of how many symbols were consumed so far.
    fn next_bits(decoder: &mut SymbolDecoder) -> u32 {
        decoder.read_bools_n(24).unwrap()
    }

    /// `read_delta_q_lf` against a decoder that performs the spec reads by hand: one `delta_q`,
    /// then `lf_indices` (the `delta_lf` CDF index of every component read).
    fn assert_matches_manual_reads(
        data: &[u8],
        frame: &FrameCodingParams,
        lf_indices: &[usize],
        current_qp: i16,
    ) {
        let sb = SuperblockCtx::new(0, 0, 64);
        let mut got = SymbolDecoder::new(data).unwrap();
        let qp = read_delta_q_lf(&mut got, &sb, mi(0, 0, 4, 4), false, frame, current_qp);

        let mut want = SymbolDecoder::new(data).unwrap();
        let delta = want.read_delta_q().unwrap();
        for &i in lf_indices {
            want.read_delta_lf(i).unwrap();
        }
        assert_eq!(qp, (current_qp + delta).clamp(0, 255), "{lf_indices:?}");
        assert_eq!(next_bits(&mut got), next_bits(&mut want), "{lf_indices:?}");
    }

    #[test]
    fn delta_q_is_read_only_for_the_first_block_of_a_superblock_that_is_not_a_skipped_full_one() {
        let sb = SuperblockCtx::new(64, 64, 64); // x4 = y4 = 16, size4 = 16
        let frame_on = params(true, false, false, false);
        for seed in 1..=6u64 {
            let data = noise(seed, 48);
            let mut reference = SymbolDecoder::new(&data).unwrap();
            let delta = reference.read_delta_q().unwrap();
            let after_read = next_bits(&mut reference);
            let untouched = next_bits(&mut SymbolDecoder::new(&data).unwrap());

            // (mi, skip, frame, is read?)
            let cases = [
                (mi(16, 16, 16, 16), false, &frame_on, true), // whole superblock, coded
                (mi(16, 16, 16, 16), true, &frame_on, false), // whole superblock, skipped
                (mi(16, 16, 8, 8), true, &frame_on, true),    // smaller block: skip is irrelevant
                (mi(16, 16, 16, 8), true, &frame_on, true),   // only one side is full-size
                (mi(20, 16, 8, 8), false, &frame_on, false),  // not the first block
                (mi(16, 20, 8, 8), false, &frame_on, false),
                (
                    mi(16, 16, 16, 16),
                    false,
                    &params(false, true, true, false),
                    false,
                ), // disabled
            ];
            for (n, (rect, skip, frame, read)) in cases.iter().enumerate() {
                let mut d = SymbolDecoder::new(&data).unwrap();
                let qp = read_delta_q_lf(&mut d, &sb, *rect, *skip, frame, 100);
                if *read {
                    assert_eq!(qp, (100 + delta).clamp(0, 255), "seed {seed} case {n}");
                    assert_eq!(next_bits(&mut d), after_read, "seed {seed} case {n}");
                } else {
                    assert_eq!(qp, 100, "seed {seed} case {n}");
                    assert_eq!(next_bits(&mut d), untouched, "seed {seed} case {n}");
                }
            }
            assert_ne!(
                after_read, untouched,
                "seed {seed}: the read must be observable"
            );
        }
    }

    #[test]
    fn delta_q_result_is_clamped_to_the_qindex_range() {
        let (mut hit_low, mut hit_high) = (false, false);
        for seed in 1..=64u64 {
            let data = noise(seed, 48);
            let delta = SymbolDecoder::new(&data).unwrap().read_delta_q().unwrap();
            let sb = SuperblockCtx::new(0, 0, 64);
            let frame = params(true, false, false, false);
            for current in [0i16, 255] {
                let mut d = SymbolDecoder::new(&data).unwrap();
                let qp = read_delta_q_lf(&mut d, &sb, mi(0, 0, 4, 4), false, &frame, current);
                assert!((0..=255).contains(&qp));
                assert_eq!(qp, (current + delta).clamp(0, 255));
                hit_low |= current + delta < 0;
                hit_high |= current + delta > 255;
            }
        }
        assert!(hit_low && hit_high, "the seeds must exercise both clamps");
    }

    #[test]
    fn delta_lf_component_count_follows_multi_and_monochrome() {
        // (delta_lf_multi, mono_chrome) -> the delta_lf CDF indices spec 5.11.38 reads.
        let cases: [(bool, bool, &[usize]); 4] = [
            (false, false, &[0]),
            (false, true, &[0]),
            (true, false, &[1, 2, 3, 4]),
            (true, true, &[1, 2]),
        ];
        for seed in 1..=4u64 {
            let data = noise(seed, 64);
            for (multi, mono, indices) in cases {
                let frame = params(true, true, multi, mono);
                assert_matches_manual_reads(&data, &frame, indices, 100);
            }
            // The fingerprint tells the counts apart: reading one component fewer or more
            // leaves the decoder somewhere else.
            let bits = |lf: &[usize]| {
                let mut d = SymbolDecoder::new(&data).unwrap();
                d.read_delta_q().unwrap();
                for &i in lf {
                    d.read_delta_lf(i).unwrap();
                }
                next_bits(&mut d)
            };
            assert_ne!(bits(&[1, 2]), bits(&[1, 2, 3, 4]), "seed {seed}");
            // Each index owns its own adapting CDF: four reads of one CDF drift away from one read of
            // each of four.
            assert_ne!(bits(&[0, 0, 0, 0]), bits(&[1, 2, 3, 4]), "seed {seed}");
        }
    }

    #[test]
    fn delta_lf_is_not_read_without_delta_lf_present() {
        let data = noise(3, 48);
        let frame = params(true, false, true, false);
        assert_matches_manual_reads(&data, &frame, &[], 100);
    }

    #[test]
    fn a_broken_delta_q_cdf_keeps_the_current_qp_and_the_delta_lf_reads_still_run() {
        let data = noise(5, 48);
        let sb = SuperblockCtx::new(0, 0, 64);
        let frame = params(true, true, false, false);

        let mut got = SymbolDecoder::new(&data).unwrap();
        got.cdf_context.get_delta_q_cdf_mut().fill(32768);
        assert!(
            got.read_delta_q().is_err(),
            "the injected CDF must overrun the alphabet"
        );
        let mut got = SymbolDecoder::new(&data).unwrap();
        got.cdf_context.get_delta_q_cdf_mut().fill(32768);
        let qp = read_delta_q_lf(&mut got, &sb, mi(0, 0, 4, 4), false, &frame, 77);
        assert_eq!(qp, 77);

        // The failed read consumed nothing, so the single delta_lf read starts at the beginning.
        let mut want = SymbolDecoder::new(&data).unwrap();
        want.read_delta_lf(0).unwrap();
        assert_eq!(next_bits(&mut got), next_bits(&mut want));
    }

    #[test]
    fn a_broken_delta_lf_cdf_stops_the_remaining_components_but_keeps_the_qp() {
        let data = noise(9, 64);
        let sb = SuperblockCtx::new(0, 0, 64);
        let frame = params(true, true, true, false);

        let mut got = SymbolDecoder::new(&data).unwrap();
        got.cdf_context.get_delta_lf_cdf_mut(2).fill(32768); // the second component
        let qp = read_delta_q_lf(&mut got, &sb, mi(0, 0, 4, 4), false, &frame, 100);

        // delta_q, component 1 (index 1) read normally; component 2 fails and ends the loop.
        let mut want = SymbolDecoder::new(&data).unwrap();
        let delta = want.read_delta_q().unwrap();
        want.read_delta_lf(1).unwrap();
        assert_eq!(
            qp,
            (100 + delta).clamp(0, 255),
            "the qp survives a delta_lf failure"
        );
        assert_eq!(
            next_bits(&mut got),
            next_bits(&mut want),
            "components 3 and 4 are skipped"
        );
    }
}
