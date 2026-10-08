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
}
