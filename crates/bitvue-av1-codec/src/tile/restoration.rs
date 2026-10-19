//! Loop-restoration coefficients (spec 5.11.57 `read_lr`, 5.11.58 `read_lr_unit`)
//!
//! Before the partition of a superblock, the tile data carries the filter coefficients of every
//! restoration unit whose top-left corner falls inside that superblock. Nothing in the picture
//! depends on them until the post-filter, but the symbols are in the same arithmetic-coded
//! stream, so skipping them desyncs every symbol after the first superblock.
//!
//! Coefficients are coded as sub-exponential differences from the previous unit's coefficients
//! of the same plane (`RefLrWiener`/`RefSgrXqd` in the spec), restarting from fixed defaults at
//! the start of each tile. Ported from dav1d's `read_restoration_info` and the unit-start logic
//! in `decode_sb_row` (`src/decode.c`).

use crate::frame_header::LoopRestorationType;
use crate::symbol::SymbolDecoder;
use bitvue_engine::Result;

/// Frame-level restoration parameters the tile data depends on.
#[derive(Debug, Clone, Copy)]
pub struct RestorationParams {
    /// Frame restoration type per plane (`FrameRestorationType`).
    pub types: [LoopRestorationType; 3],
    /// Restoration unit size in pixels per plane (chroma already includes `lr_uv_shift`).
    pub unit_size: [u32; 3],
    /// Luma size of the (upscaled-before-filtering) frame. Units are laid out against it.
    pub frame_width: u32,
    pub frame_height: u32,
    pub subsampling_x: bool,
    pub subsampling_y: bool,
    /// `use_superres` -- unit positions then depend on the upscaling ratio, which is not
    /// implemented; see [`read_superblock_restoration`].
    pub superres: bool,
}

impl RestorationParams {
    /// No plane is restored, so the tile data carries no coefficients.
    pub fn none() -> Self {
        Self {
            types: [LoopRestorationType::None; 3],
            unit_size: [64; 3],
            frame_width: 0,
            frame_height: 0,
            subsampling_x: false,
            subsampling_y: false,
            superres: false,
        }
    }
}

/// The coefficients a unit is coded relative to (`RefLrWiener`, `RefSgrXqd`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestorationRef {
    pub filter_v: [i32; 3],
    pub filter_h: [i32; 3],
    pub sgr_weights: [i32; 2],
}

impl Default for RestorationRef {
    fn default() -> Self {
        Self {
            filter_v: [3, -7, 15],
            filter_h: [3, -7, 15],
            sgr_weights: [-32, 31],
        }
    }
}

/// `Sgr_Params` has `r0 == 0` for sets 10..=13 and `r1 == 0` for sets 14..=15; a missing radius
/// means its weight is not coded.
fn sgr_weight_is_coded(set: u32) -> [bool; 2] {
    [!(10..=13).contains(&set), set < 14]
}

fn inv_recenter(r: u32, v: u32) -> u32 {
    if v > 2 * r {
        v
    } else if v & 1 == 1 {
        r - ((v + 1) >> 1)
    } else {
        r + (v >> 1)
    }
}

/// Sub-exponential value in `0..n` coded relative to `reference` (`decode_signed_subexp_with_ref_bool`).
fn decode_subexp(decoder: &mut SymbolDecoder<'_>, reference: i32, n: i32, k: u32) -> Result<i32> {
    let mut k = k;
    let mut a = 0u32;
    if decoder.read_bool_equi()? {
        if decoder.read_bool_equi()? {
            k += decoder.read_bool_equi()? as u32 + 1;
        }
        a = 1 << k;
    }
    let v = decoder.read_bools_n(k)? + a;
    let (r, n) = (reference as u32, n as u32);
    Ok(if r * 2 <= n {
        inv_recenter(r, v) as i32
    } else {
        (n - 1 - inv_recenter(n - 1 - r, v)) as i32
    })
}

/// Read one unit's coefficients (`read_lr_unit`) and make them the reference for the next unit.
fn read_unit(
    decoder: &mut SymbolDecoder<'_>,
    plane: usize,
    frame_type: LoopRestorationType,
    reference: &mut RestorationRef,
) -> Result<()> {
    let unit_type = match frame_type {
        LoopRestorationType::Switchable => match decoder.read_restore_switchable()? {
            0 => LoopRestorationType::None,
            1 => LoopRestorationType::Wiener,
            _ => LoopRestorationType::SgrProj,
        },
        LoopRestorationType::Wiener => {
            if decoder.read_restore_wiener()? {
                LoopRestorationType::Wiener
            } else {
                LoopRestorationType::None
            }
        }
        LoopRestorationType::SgrProj => {
            if decoder.read_restore_sgrproj()? {
                LoopRestorationType::SgrProj
            } else {
                LoopRestorationType::None
            }
        }
        LoopRestorationType::None => return Ok(()),
    };

    match unit_type {
        LoopRestorationType::Wiener => {
            let mut next = *reference;
            // Chroma has a 5-tap filter: its outermost tap is not coded.
            for (taps, base) in [
                (&mut next.filter_v, reference.filter_v),
                (&mut next.filter_h, reference.filter_h),
            ] {
                taps[0] = if plane == 0 {
                    decode_subexp(decoder, base[0] + 5, 16, 1)? - 5
                } else {
                    0
                };
                taps[1] = decode_subexp(decoder, base[1] + 23, 32, 2)? - 23;
                taps[2] = decode_subexp(decoder, base[2] + 17, 64, 3)? - 17;
            }
            *reference = next;
        }
        LoopRestorationType::SgrProj => {
            let set = decoder.read_bools_n(4)?;
            let coded = sgr_weight_is_coded(set);
            let w0 = if coded[0] {
                decode_subexp(decoder, reference.sgr_weights[0] + 96, 128, 4)? - 96
            } else {
                0
            };
            let w1 = if coded[1] {
                decode_subexp(decoder, reference.sgr_weights[1] + 32, 128, 4)? - 32
            } else {
                95
            };
            reference.sgr_weights = [w0, w1];
        }
        _ => {}
    }
    Ok(())
}

/// Whether a restoration unit of `plane` starts inside the superblock whose luma origin is
/// `(x, y)`: the plane position must be unit-aligned, and the last unit, if shorter than half a
/// unit, is merged into its neighbour instead of starting a unit of its own.
fn unit_starts_at(params: &RestorationParams, plane: usize, x: u32, y: u32) -> bool {
    let (ss_x, ss_y) = if plane == 0 {
        (0, 0)
    } else {
        (params.subsampling_x as u32, params.subsampling_y as u32)
    };
    let unit_size = params.unit_size[plane];
    let half_unit = unit_size >> 1;
    let (px, py) = (x >> ss_x, y >> ss_y);
    let plane_w = (params.frame_width + ss_x) >> ss_x;
    let plane_h = (params.frame_height + ss_y) >> ss_y;
    let aligned = px & (unit_size - 1) == 0 && py & (unit_size - 1) == 0;
    let big_enough =
        (py == 0 || py + half_unit <= plane_h) && (px == 0 || px + half_unit <= plane_w);
    aligned && big_enough
}

/// Read the coefficients of all restoration units that start inside the superblock at `(x, y)`
/// (luma pixels, `sb_size` wide), plane by plane, exactly where `decode_sb_row` does: before the
/// superblock's first partition symbol.
///
/// Returns an error for superres frames, whose unit grid depends on the upscaling ratio: decoding
/// on would misplace every later symbol.
pub fn read_superblock_restoration(
    decoder: &mut SymbolDecoder<'_>,
    params: &RestorationParams,
    refs: &mut [RestorationRef; 3],
    x: u32,
    y: u32,
) -> Result<()> {
    for (plane, reference) in refs.iter_mut().enumerate() {
        let frame_type = params.types[plane];
        if frame_type == LoopRestorationType::None {
            continue;
        }
        if params.superres {
            return Err(bitvue_engine::BitvueError::Decode(
                "loop restoration with superres is not supported".into(),
            ));
        }
        if !unit_starts_at(params, plane, x, y) {
            continue;
        }
        read_unit(decoder, plane, frame_type, reference)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inv_recenter_matches_the_spec_table() {
        // Spec 5.9.28 `inverse_recenter`: values far from r map to themselves; near ones
        // alternate below/above r.
        let got: Vec<u32> = (0..9).map(|v| inv_recenter(3, v)).collect();
        assert_eq!(got, [3, 2, 4, 1, 5, 0, 6, 7, 8]);
    }

    #[test]
    fn sgr_sets_without_a_radius_skip_its_weight() {
        assert_eq!(sgr_weight_is_coded(0), [true, true]);
        assert_eq!(sgr_weight_is_coded(9), [true, true]);
        assert_eq!(sgr_weight_is_coded(10), [false, true]);
        assert_eq!(sgr_weight_is_coded(13), [false, true]);
        assert_eq!(sgr_weight_is_coded(14), [true, false]);
        assert_eq!(sgr_weight_is_coded(15), [true, false]);
    }

    fn params(unit_size: [u32; 3], w: u32, h: u32) -> RestorationParams {
        RestorationParams {
            types: [LoopRestorationType::Switchable; 3],
            unit_size,
            frame_width: w,
            frame_height: h,
            subsampling_x: true,
            subsampling_y: true,
            superres: false,
        }
    }

    #[test]
    fn units_start_only_at_aligned_superblocks() {
        // 64-pixel units on 64x64 superblocks: every superblock starts one luma unit.
        let p = params([64, 32, 32], 320, 240);
        assert!(unit_starts_at(&p, 0, 0, 0));
        assert!(unit_starts_at(&p, 0, 64, 128));
        // Chroma units are 32 samples = 64 luma pixels here, so also one per superblock.
        assert!(unit_starts_at(&p, 1, 64, 64));
        // 256-pixel luma units: only every fourth 64x64 superblock starts one.
        let p = params([256, 256, 256], 320, 240);
        assert!(unit_starts_at(&p, 0, 0, 0));
        assert!(!unit_starts_at(&p, 0, 64, 0));
        assert!(!unit_starts_at(&p, 0, 0, 64));
        // Chroma plane position is halved: x=256 -> 128, not unit aligned.
        assert!(!unit_starts_at(&p, 1, 256, 0));
    }

    #[test]
    fn a_trailing_partial_unit_is_merged_into_the_previous_one() {
        // 320 wide with 256 units: x=256 leaves 64 < half a unit, so no unit starts there.
        let p = params([256, 256, 256], 320, 240);
        assert!(!unit_starts_at(&p, 0, 256, 0));
        // 400 wide: the remaining 144 >= 128, so a second unit starts.
        let p = params([256, 256, 256], 400, 240);
        assert!(unit_starts_at(&p, 0, 256, 0));
        // Exactly half a unit left still starts one (round half up), in both directions.
        let p = params([256, 256, 256], 384, 384);
        assert!(unit_starts_at(&p, 0, 256, 0));
        assert!(unit_starts_at(&p, 0, 0, 256));
    }

    #[test]
    fn defaults_match_the_spec_reference_values() {
        let r = RestorationRef::default();
        assert_eq!(r.filter_v, [3, -7, 15]);
        assert_eq!(r.filter_h, [3, -7, 15]);
        assert_eq!(r.sgr_weights, [-32, 31]);
    }

    // ---- Wiener coefficients against the spec's own algorithm -------------------------------
    //
    // The production `decode_subexp` is dav1d's shortcut for `n >> k == 8` (three decision bits
    // and a fixed-width tail). The spec (5.11.58 `decode_signed_subexp_with_ref_bool`) is built
    // on `ns(n)` instead. Both are transcribed here from the spec and run on the same bytes as
    // production, so the shortcut has to give the spec's answer, bit for bit.

    /// Pseudo-random tile data per seed (any byte string is a decodable stream).
    fn tile_bytes(seed: u64) -> Vec<u8> {
        let mut s = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
        (0..256)
            .map(|_| {
                s ^= s >> 12;
                s ^= s << 25;
                s ^= s >> 27;
                (s.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 56) as u8
            })
            .collect()
    }

    /// Spec 4.10.10 `ns(n)`.
    fn spec_ns(d: &mut SymbolDecoder<'_>, n: u32) -> u32 {
        let w = 32 - n.leading_zeros(); // FloorLog2(n) + 1
        let m = (1 << w) - n;
        let v = d.read_bools_n(w - 1).unwrap();
        if v < m {
            v
        } else {
            let extra = d.read_bool_equi().unwrap() as u32;
            (v << 1) - m + extra
        }
    }

    /// Spec 5.11.58 `decode_subexp_bool`; also reports how many "more" bits were set.
    fn spec_subexp_bool(
        d: &mut SymbolDecoder<'_>,
        num_syms: u32,
        k: u32,
        levels: &mut [usize; 3],
    ) -> u32 {
        let (mut i, mut mk) = (0u32, 0u32);
        loop {
            let b2 = if i > 0 { k + i - 1 } else { k };
            let a = 1 << b2;
            if num_syms <= mk + 3 * a {
                levels[i as usize] += 1;
                return spec_ns(d, num_syms - mk) + mk;
            }
            if d.read_bool_equi().unwrap() {
                i += 1;
                mk += a;
            } else {
                levels[i as usize] += 1;
                return d.read_bools_n(b2).unwrap() + mk;
            }
        }
    }

    /// Spec 5.9.28 `inverse_recenter`.
    fn spec_inverse_recenter(r: i32, v: i32) -> i32 {
        if v > 2 * r {
            v
        } else if v & 1 == 1 {
            r - ((v + 1) >> 1)
        } else {
            r + (v >> 1)
        }
    }

    /// Spec `decode_signed_subexp_with_ref_bool(low, high, k, r)`.
    fn spec_signed_subexp(
        d: &mut SymbolDecoder<'_>,
        (low, high): (i32, i32),
        k: u32,
        r: i32,
        levels: &mut [usize; 3],
    ) -> i32 {
        let (mx, r) = (high - low, r - low);
        let v = spec_subexp_bool(d, mx as u32, k, levels) as i32;
        let x = if (r << 1) <= mx {
            spec_inverse_recenter(r, v)
        } else {
            mx - 1 - spec_inverse_recenter(mx - 1 - r, v)
        };
        x + low
    }

    /// The Wiener coefficients of one unit as the spec reads them.
    fn spec_wiener_unit(
        d: &mut SymbolDecoder<'_>,
        plane: usize,
        reference: &RestorationRef,
        levels: &mut [usize; 3],
    ) -> RestorationRef {
        // Wiener_Taps_Min / Max / K per tap.
        const RANGE: [(i32, i32); 3] = [(-5, 10), (-23, 8), (-17, 46)];
        const K: [u32; 3] = [1, 2, 3];
        let mut out = *reference;
        for (pass, taps) in [&mut out.filter_v, &mut out.filter_h]
            .into_iter()
            .enumerate()
        {
            let before = if pass == 0 {
                reference.filter_v
            } else {
                reference.filter_h
            };
            let first = if plane > 0 { 1 } else { 0 };
            if plane > 0 {
                taps[0] = 0;
            }
            for j in first..3 {
                let (min, max) = RANGE[j];
                taps[j] = spec_signed_subexp(d, (min, max + 1), K[j], before[j], levels);
            }
        }
        out
    }

    fn wiener_cdf(d: &SymbolDecoder<'_>) -> Vec<u16> {
        let mut ctx = d.cdf_context.clone();
        ctx.get_restore_wiener_cdf_mut().to_vec()
    }

    fn switchable_cdf(d: &SymbolDecoder<'_>) -> Vec<u16> {
        let mut ctx = d.cdf_context.clone();
        ctx.get_restore_switchable_cdf_mut().to_vec()
    }

    /// A frame four 64-pixel units wide: one unit per plane starts in each 64x64 superblock, and
    /// each unit is coded relative to the previous one of its plane.
    fn four_units_per_plane(types: [LoopRestorationType; 3]) -> RestorationParams {
        RestorationParams {
            types,
            unit_size: [64, 32, 32],
            frame_width: 256,
            frame_height: 64,
            subsampling_x: true,
            subsampling_y: true,
            superres: false,
        }
    }

    #[test]
    fn wiener_frames_read_the_spec_coefficients_for_every_plane_and_carry_the_reference() {
        let p = four_units_per_plane([LoopRestorationType::Wiener; 3]);
        let (mut units, mut changed, mut levels) = (0usize, 0usize, [0usize; 3]);
        let mut mirrored = 0usize;
        for seed in 0..300 {
            let data = tile_bytes(seed);
            let mut prod = SymbolDecoder::new(&data).unwrap();
            let mut refs = [RestorationRef::default(); 3];

            let mut spec = SymbolDecoder::new(&data).unwrap();
            let mut use_wiener = wiener_cdf(&spec);
            let mut want = [RestorationRef::default(); 3];

            for x in [0, 64, 128, 192] {
                read_superblock_restoration(&mut prod, &p, &mut refs, x, 0).unwrap();
                for (plane, slot) in want.iter_mut().enumerate() {
                    // `use_wiener` is symbol 1 of its two-symbol alphabet.
                    if spec.decoder.read_symbol_adaptive(&mut use_wiener).unwrap() == 1 {
                        units += 1;
                        // The spec mirrors a coefficient whose reference is above the middle.
                        for (j, (low, mx)) in
                            [(-5, 16), (-23, 32), (-17, 64)].into_iter().enumerate()
                        {
                            mirrored += usize::from((slot.filter_v[j] - low) * 2 > mx);
                        }
                        *slot = spec_wiener_unit(&mut spec, plane, slot, &mut levels);
                        changed += usize::from(*slot != RestorationRef::default());
                    }
                }
                assert_eq!(refs, want, "seed {seed}, x {x}");
            }
            // Both decoders must have consumed exactly the same bits.
            assert_eq!(
                prod.read_bools_n(16).unwrap(),
                spec.read_bools_n(16).unwrap(),
                "seed {seed}: position after the units differs"
            );
        }
        // The comparison only means something if it reached the interesting branches.
        assert!(units > 500, "only {units} Wiener units");
        assert!(changed > 400, "only {changed} units moved off the defaults");
        assert!(
            levels.iter().all(|&n| n > 200),
            "decision depths seen: {levels:?}"
        );
        assert!(
            mirrored > 200,
            "only {mirrored} references above the middle"
        );
    }

    #[test]
    fn a_switchable_frame_reads_wiener_coefficients_only_when_the_unit_type_is_wiener() {
        let p = four_units_per_plane([
            LoopRestorationType::Switchable,
            LoopRestorationType::None,
            LoopRestorationType::None,
        ]);
        let (mut none, mut wiener, mut complete) = (0usize, 0usize, 0usize);
        let mut levels = [0usize; 3];
        for seed in 0..600 {
            let data = tile_bytes(seed);
            let mut prod = SymbolDecoder::new(&data).unwrap();
            let mut refs = [RestorationRef::default(); 3];

            let mut spec = SymbolDecoder::new(&data).unwrap();
            let mut switchable = switchable_cdf(&spec);
            let mut want = RestorationRef::default();

            let mut all_units = true;
            for x in [0, 64, 128, 192] {
                read_superblock_restoration(&mut prod, &p, &mut refs, x, 0).unwrap();
                // 0 = NONE (nothing follows), 1 = WIENER (coefficients follow), 2 = SGRPROJ (its
                // weights are not compared here, so the walk stops at the first one).
                match spec.decoder.read_symbol_adaptive(&mut switchable).unwrap() {
                    0 => none += 1,
                    1 => {
                        wiener += 1;
                        want = spec_wiener_unit(&mut spec, 0, &want, &mut levels);
                    }
                    _ => {
                        all_units = false;
                        break;
                    }
                }
                assert_eq!(refs[0], want, "seed {seed}, x {x}");
            }
            if all_units {
                complete += 1;
                assert_eq!(
                    prod.read_bools_n(16).unwrap(),
                    spec.read_bools_n(16).unwrap(),
                    "seed {seed}: position after the units differs"
                );
            }
        }
        assert!(none > 100, "only {none} NONE units");
        assert!(wiener > 100, "only {wiener} WIENER units");
        assert!(
            complete > 60,
            "only {complete} frames without an SGRPROJ unit"
        );
    }

    #[test]
    fn superres_with_a_restored_plane_is_an_error_but_an_unrestored_frame_is_not() {
        let data = [0x5a_u8; 64];
        let mut refs = [RestorationRef::default(); 3];

        // Every plane None: nothing to read, superres or not.
        let mut none = RestorationParams::none();
        none.superres = true;
        let mut d = SymbolDecoder::new(&data).unwrap();
        let before = d.decoder.range;
        read_superblock_restoration(&mut d, &none, &mut refs, 0, 0).unwrap();
        assert_eq!(d.decoder.range, before, "no symbol was read");

        // One restored plane: refused, whichever plane it is and wherever the unit starts.
        for plane in 0..3 {
            let mut p = params([64; 3], 128, 128);
            p.types = [LoopRestorationType::None; 3];
            p.types[plane] = LoopRestorationType::Wiener;
            p.superres = true;
            for (x, y) in [(0, 0), (64, 64), (7, 3)] {
                let mut d = SymbolDecoder::new(&data).unwrap();
                let err = read_superblock_restoration(&mut d, &p, &mut refs, x, y).unwrap_err();
                assert!(
                    matches!(&err, bitvue_engine::BitvueError::Decode(m) if m.contains("superres")),
                    "plane {plane} at ({x},{y}): {err:?}"
                );
            }
            // Without superres the same call is fine.
            p.superres = false;
            let mut d = SymbolDecoder::new(&data).unwrap();
            read_superblock_restoration(&mut d, &p, &mut refs, 0, 0).unwrap();
        }
    }
}
