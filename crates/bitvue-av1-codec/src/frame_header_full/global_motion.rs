//! Global-motion parameters (spec 5.9.24/5.9.25).

use super::*;

// GM_*_PREC_BITS (spec's precBits per param) aren't needed: they only affect the recentred
// *value* (`precDiff = WARPEDMODEL_PREC_BITS - precBits`), not how many bits decode_subexp
// consumes, which is what this parser needs (see skip_global_param's doc).
const GM_ABS_ALPHA_BITS: u32 = 12;
const GM_ABS_TRANS_ONLY_BITS: u32 = 9;
const GM_ABS_TRANS_BITS: u32 = 12;

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

/// `read_global_param`'s bit consumption (AV1 spec 5.9.26 `decode_signed_subexp_with_ref`) --
/// value is discarded (see module doc: not needed here, and computing the real recentred value
/// would need `PrevGmParams`, which correct bit consumption does not).
fn skip_global_param(
    reader: &mut BitReader,
    is_translation_only: bool,
    is_first_two: bool,
    allow_high_precision_mv: bool,
) -> Result<()> {
    let abs_bits: u32 = if is_first_two {
        if is_translation_only {
            GM_ABS_TRANS_ONLY_BITS - u32::from(!allow_high_precision_mv)
        } else {
            GM_ABS_TRANS_BITS
        }
    } else {
        GM_ABS_ALPHA_BITS
    };
    let mx = 1u32 << abs_bits;
    decode_subexp(reader, 2 * mx + 1)?;
    Ok(())
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

/// `global_motion_params()` (spec 5.9.24) -- returns the real `GmType[ref]` classification for
/// each of the 8 `RefFrame` values (index 0/`RefFrame::Intra` unused, stays `GM_TYPE_IDENTITY`;
/// indices 1..=7 filled by this loop, spec's `LAST_FRAME..=ALTREF_FRAME` matching this crate's own
/// `RefFrame` enum numbering). `gm_params[]` values themselves are still only consumed for bit
/// position (see `skip_global_param`'s doc) -- only the classification is needed by
/// `read_motion_mode`'s real `GmType[RefFrame[0]] > TRANSLATION` exclusion.
pub(super) fn parse_global_motion_params(
    reader: &mut BitReader,
    allow_high_precision_mv: bool,
) -> Result<[u8; 8]> {
    let mut gm_type = [GM_TYPE_IDENTITY; 8];
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
        gm_type[ref_frame + 1] =
            classify_gm_type(is_global, gm_type_at_least_rotzoom, gm_type_is_translation);

        if gm_type_at_least_rotzoom {
            // ROTZOOM or AFFINE: idx 2,3 always; idx 4,5 only for AFFINE (is_rot_zoom path above
            // only sets gm_type_at_least_rotzoom for ROTZOOM -- AFFINE is reached via the
            // is_translation==false branch below with gm_type_at_least_rotzoom left false, so
            // handle AFFINE's extra params there instead).
            skip_global_param(reader, false, false, allow_high_precision_mv)?;
            skip_global_param(reader, false, false, allow_high_precision_mv)?;
        } else if is_global && !gm_type_is_translation {
            // AFFINE (is_global, not rot_zoom, not translation).
            skip_global_param(reader, false, false, allow_high_precision_mv)?;
            skip_global_param(reader, false, false, allow_high_precision_mv)?;
            skip_global_param(reader, false, false, allow_high_precision_mv)?;
            skip_global_param(reader, false, false, allow_high_precision_mv)?;
        }
        if is_global {
            // TRANSLATION, ROTZOOM, and AFFINE all read params[0] and params[1].
            skip_global_param(
                reader,
                gm_type_is_translation,
                true,
                allow_high_precision_mv,
            )?;
            skip_global_param(
                reader,
                gm_type_is_translation,
                true,
                allow_high_precision_mv,
            )?;
        }
    }
    Ok(gm_type)
}
