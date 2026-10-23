//! Global-motion parameters (spec 5.9.24/5.9.25).

use super::*;

const GM_ABS_ALPHA_BITS: u32 = 12;
const GM_ALPHA_PREC_BITS: u32 = 15;
const GM_ABS_TRANS_ONLY_BITS: u32 = 9;
const GM_TRANS_ONLY_PREC_BITS: u32 = 3;
const GM_ABS_TRANS_BITS: u32 = 12;
const GM_TRANS_PREC_BITS: u32 = 6;
const WARPEDMODEL_PREC_BITS: u32 = 16;

/// One reference's six warp parameters (spec `gm_params[ref]`).
pub type GmRefParams = [i32; 6];
/// `gm_params[ref]` for the 8 `RefFrame` values (index 0/`Intra` unused, stays the identity).
pub type GmParams = [GmRefParams; 8];

/// The identity warp (spec `set_default_gm`): `1 << WARPEDMODEL_PREC_BITS` on the diagonal.
pub const GM_IDENTITY_PARAMS: GmRefParams = [
    0,
    0,
    1 << WARPEDMODEL_PREC_BITS,
    0,
    0,
    1 << WARPEDMODEL_PREC_BITS,
];

/// `decode_subexp(numSyms)` (AV1 spec 5.9.27) -- consumes exactly the bits a real decoder would,
/// self-terminating and independent of any reference value (see module doc). Returns the decoded
/// value even though callers here only need correct bit consumption, since computing it is no
/// extra work and makes the function independently testable against the spec's own worked
/// examples.
fn decode_subexp(reader: &mut BitReader, num_syms: u32) -> Result<u32> {
    let mut i = 0u32;
    let mut mk = 0u32;
    let k = 3u32;
    loop {
        let b2 = if i != 0 { k + i - 1 } else { k };
        let a = 1u32 << b2;
        if num_syms <= mk + 3 * a {
            let subexp_final_bits = read_ns(reader, num_syms - mk)?;
            return Ok(subexp_final_bits + mk);
        }
        let subexp_more_bits = reader.read_bit()?;
        if subexp_more_bits {
            i += 1;
            mk += a;
        } else {
            let subexp_bits = reader.read_bits(b2 as u8)?;
            return Ok(subexp_bits + mk);
        }
    }
}

/// `inverse_recenter(r, v)` (AV1 spec 5.9.28).
fn inverse_recenter(r: i64, v: i64) -> i64 {
    if v > 2 * r {
        v
    } else if v & 1 == 1 {
        r - ((v + 1) >> 1)
    } else {
        r + (v >> 1)
    }
}

/// `decode_signed_subexp_with_ref(low, high, r)` (spec 5.9.26) via `decode_unsigned_subexp_with_ref`
/// (5.9.27).
fn decode_signed_subexp_with_ref(
    reader: &mut BitReader,
    low: i64,
    high: i64,
    r: i64,
) -> Result<i64> {
    let mx = high - low;
    let r = r - low;
    let v = i64::from(decode_subexp(reader, mx as u32)?);
    let x = if (r << 1) <= mx {
        inverse_recenter(r, v)
    } else {
        mx - 1 - inverse_recenter(mx - 1 - r, v)
    };
    Ok(x + low)
}

/// `read_global_param(type, ref, idx)` (spec 5.9.25): one warp parameter, recentred on the value
/// the previous frame left in `prev` (`PrevGmParams[ref][idx]`).
fn read_global_param(
    reader: &mut BitReader,
    gm_type: u8,
    idx: usize,
    prev: i32,
    allow_high_precision_mv: bool,
) -> Result<i32> {
    let (abs_bits, prec_bits) = if idx < 2 {
        if gm_type == GM_TYPE_TRANSLATION {
            let lost = u32::from(!allow_high_precision_mv);
            (
                GM_ABS_TRANS_ONLY_BITS - lost,
                GM_TRANS_ONLY_PREC_BITS - lost,
            )
        } else {
            (GM_ABS_TRANS_BITS, GM_TRANS_PREC_BITS)
        }
    } else {
        (GM_ABS_ALPHA_BITS, GM_ALPHA_PREC_BITS)
    };
    let prec_diff = WARPEDMODEL_PREC_BITS - prec_bits;
    let diagonal = idx % 3 == 2;
    let round: i64 = if diagonal {
        1 << WARPEDMODEL_PREC_BITS
    } else {
        0
    };
    let sub: i64 = if diagonal { 1 << prec_bits } else { 0 };
    let mx = 1i64 << abs_bits;
    let r = (i64::from(prev) >> prec_diff) - sub;
    let v = decode_signed_subexp_with_ref(reader, -mx, mx + 1, r)?;
    Ok(((v << prec_diff) + round) as i32)
}

/// `GmType` values (spec 5.9.24), in the real spec's numeric order (`GmType[ref] > TRANSLATION`
/// comparisons elsewhere, e.g. `read_motion_mode`'s doc, rely on this exact ordering).
pub(crate) const GM_TYPE_IDENTITY: u8 = 0;
pub(crate) const GM_TYPE_TRANSLATION: u8 = 1;
pub(crate) const GM_TYPE_ROTZOOM: u8 = 2;
pub(super) const GM_TYPE_AFFINE: u8 = 3;

/// Pure classification (no bit reads): maps `global_motion_params()`'s 3 flag bits to a real
/// `GmType` value (spec 5.9.24's `is_global`/`is_rot_zoom`/`is_translation` decision tree).
/// Extracted from `parse_global_motion_params` so the mapping is directly unit-testable without
/// needing to hand-encode `decode_subexp`'s variable-length `gm_params[]` bits.
pub(super) fn classify_gm_type(
    is_global: bool,
    gm_type_at_least_rotzoom: bool,
    gm_type_is_translation: bool,
) -> u8 {
    if !is_global {
        GM_TYPE_IDENTITY
    } else if gm_type_at_least_rotzoom {
        GM_TYPE_ROTZOOM
    } else if gm_type_is_translation {
        GM_TYPE_TRANSLATION
    } else {
        GM_TYPE_AFFINE
    }
}

/// `global_motion_params()` (spec 5.9.24/5.9.25) -- returns `GmType[ref]` and `gm_params[ref]` for
/// each of the 8 `RefFrame` values (index 0/`RefFrame::Intra` unused, stays identity; indices
/// 1..=7 filled by this loop, spec's `LAST_FRAME..=ALTREF_FRAME` matching this crate's own
/// `RefFrame` enum numbering). `prev` is `PrevGmParams`: the identity when the frame has no
/// primary reference frame, otherwise what that reference's slot saved.
pub(super) fn parse_global_motion_params(
    reader: &mut BitReader,
    allow_high_precision_mv: bool,
    prev: &GmParams,
) -> Result<([u8; 8], GmParams)> {
    let mut gm_type = [GM_TYPE_IDENTITY; 8];
    let mut params: GmParams = [GM_IDENTITY_PARAMS; 8];
    for ref_frame in 0..REFS_PER_FRAME {
        let is_global = reader.read_bit()?;
        let (gm_type_is_translation, gm_type_at_least_rotzoom) = if is_global {
            let is_rot_zoom = reader.read_bit()?;
            if is_rot_zoom {
                (false, true)
            } else {
                let is_translation = reader.read_bit()?;
                (is_translation, false)
            }
        } else {
            (false, false)
        };
        // Loop index 0 is `LAST_FRAME` (`RefFrame::Last as usize == 1`), so `ref_frame + 1` is the
        // real `RefFrame` index this classification belongs to.
        let ty = classify_gm_type(is_global, gm_type_at_least_rotzoom, gm_type_is_translation);
        let slot = ref_frame + 1;
        gm_type[slot] = ty;

        let p = &mut params[slot];
        let before = &prev[slot];
        if ty >= GM_TYPE_ROTZOOM {
            p[2] = read_global_param(reader, ty, 2, before[2], allow_high_precision_mv)?;
            p[3] = read_global_param(reader, ty, 3, before[3], allow_high_precision_mv)?;
            if ty == GM_TYPE_AFFINE {
                p[4] = read_global_param(reader, ty, 4, before[4], allow_high_precision_mv)?;
                p[5] = read_global_param(reader, ty, 5, before[5], allow_high_precision_mv)?;
            } else {
                p[4] = -p[3];
                p[5] = p[2];
            }
        }
        if ty >= GM_TYPE_TRANSLATION {
            p[0] = read_global_param(reader, ty, 0, before[0], allow_high_precision_mv)?;
            p[1] = read_global_param(reader, ty, 1, before[1], allow_high_precision_mv)?;
        }
    }
    Ok((gm_type, params))
}
