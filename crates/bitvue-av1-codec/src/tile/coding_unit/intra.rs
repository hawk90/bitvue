//! The intra mode-info tail of a non-IntraBC intra block: `y_mode`, `angle_delta_y`, `uv_mode`/
//! `cfl_alpha`/`angle_delta_uv`, `palette_mode_info`, `filter_intra_mode_info`, the per-pixel
//! palette color map, and `tx_size()`. Real dav1d runs this whole sequence under
//! `if (b->intra)`, and `b->intra = !intrabc_flag`, so IntraBC blocks skip all of it.
//!
//! The same code path serves key-frame intra blocks and intra blocks inside inter frames; the one
//! real difference is which CDF/context `y_mode` draws from (see
//! `SymbolDecoder::read_intra_mode_inter_frame`).

use super::contexts::{block_size_for_dimensions, intra_mode_from_symbol, y_mode_size_context};
use super::palette::{read_palette_mode_info, read_palette_tokens};
use super::types::CodingUnit;
use crate::symbol::SymbolDecoder;
use crate::tile::{BlockRect, FrameCodingParams, MiRect, TileContext};
use bitvue_engine::Result;

/// Fills `cu.mode`, `cu.palette` and `cu.tx_size`. Returns the raw intra `y_mode` symbol
/// (0..=12), which `read_transform_type_is_1d` needs later as its `y_mode_raw`.
pub(super) fn read_intra_mode_info(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
) -> Result<u8> {
    let BlockRect { width, height, .. } = rect;
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    let FrameCodingParams {
        is_key_frame,
        allow_screen_content_tools,
        enable_filter_intra,
        tx_type_flags,
        mi_rows,
        mi_cols,
        ..
    } = *frame;
    // Read INTRA prediction mode -- real per-context CDF + adaptation. Key frames use
    // `kfym` (real above/left neighbor-mode-class context, `read_intra_mode`'s doc);
    // non-key-frame intra CUs use a real block-size-class context instead (`y_mode_cdf`,
    // `read_intra_mode_inter_frame`'s doc) -- a real, deliberate CDF-source swap on the
    // SAME unified intra mode-info path, not two independent implementations.
    let mode_symbol = if is_key_frame {
        let (above_class, left_class) = tile_ctx.intra_mode_context(x4, y4);
        decoder.read_intra_mode(above_class, left_class)?
    } else {
        decoder.read_intra_mode_inter_frame(y_mode_size_context(width_4x4, height_4x4))?
    };
    cu.mode = intra_mode_from_symbol(mode_symbol)?;
    tile_ctx.set_mode(x4, y4, width_4x4, height_4x4, mode_symbol);

    // angle_delta_y (spec `intra_angle_info_y`) -- real per-mode CDF + adaptation. Real
    // spec gate: block isn't the smallest class (`log2(bw4)+log2(bh4) >= 2`) AND the mode
    // is directional (`V_PRED..=D67_PRED`, raw symbols `1..=8` -- see `intra_mode_from_
    // symbol`'s exact numbering, verified to match dav1d's `VERT_PRED..VERT_LEFT_PRED`).
    if width_4x4.ilog2() + height_4x4.ilog2() >= 2 && (1..=8).contains(&mode_symbol) {
        decoder.read_angle_delta(mode_symbol - 1)?;
    }

    // Real spec `HasChroma` approximation -- deliberately the SAME expression as the
    // chroma-residual site below (minus the always-true-here `!cu.use_intrabc` term), kept
    // in sync by hand since it can't share a variable across that later, wider-scoped call
    // site (reached by every CU kind, not just plain intra) -- see that site's doc for the
    // approximation itself.
    let has_chroma = !tx_type_flags.mono_chrome
        && tx_type_flags.subsampling_x
        && tx_type_flags.subsampling_y
        && (8..=128).contains(&width)
        && (8..=128).contains(&height);

    // uv_mode / cfl_alpha / angle_delta_uv -- real per-context CDF + adaptation, only read
    // at all when `has_chroma`. `cfl_allowed`: real spec `is_cfl_allowed()` (non-lossless:
    // both dims `<=32`; lossless: chroma block is exactly 4x4, i.e. luma `8x8` in 4:2:0 --
    // this crate only tracks frame-wide `coded_lossless`, not per-segment, same approximation
    // as `tx_size`'s resolution just below).
    let mut uv_mode_symbol: u8 = 0;
    if has_chroma {
        let cfl_allowed = if tx_type_flags.coded_lossless {
            width == 8 && height == 8
        } else {
            width <= 32 && height <= 32
        };
        uv_mode_symbol = decoder.read_uv_mode(cfl_allowed, mode_symbol)?;
        if uv_mode_symbol == 13 {
            decoder.read_cfl_alphas()?;
        } else if width_4x4.ilog2() + height_4x4.ilog2() >= 2 && (1..=8).contains(&uv_mode_symbol) {
            decoder.read_angle_delta(uv_mode_symbol - 1)?;
        }
    }

    // palette_mode_info (spec 5.11.46) -- real spec eligibility gate (`read_pal_indices`'s
    // call site in dav1d's `decode_b`): `allow_screen_content_tools`, `max(bw4,bh4)<=16`
    // (both dims `<=64px`), `bw4+bh4>=4` (excludes only 4x4/4x8/8x4).
    let palette_eligible = allow_screen_content_tools
        && width_4x4.max(height_4x4) <= 16
        && width_4x4 + height_4x4 >= 4;
    if palette_eligible {
        let bsize_ctx = (width_4x4.ilog2() + height_4x4.ilog2()).saturating_sub(2) as u8;
        cu.palette = read_palette_mode_info(
            decoder,
            tile_ctx,
            x4,
            y4,
            width_4x4,
            height_4x4,
            bsize_ctx,
            mode_symbol == 0,
            has_chroma,
            uv_mode_symbol == 0,
        )?;
    }

    // filter_intra_mode_info -- real per-`BlockSize` CDF + adaptation. Real spec gate:
    // `y_mode == DC_PRED`, no Y palette, both dims `<=32px`
    // (`max(log2(bw4),log2(bh4))<=3`), and the sequence header enables it.
    if mode_symbol == 0
        && cu.palette.y_size == 0
        && width_4x4.ilog2().max(height_4x4.ilog2()) <= 3
        && enable_filter_intra
        && decoder.read_use_filter_intra(block_size_for_dimensions(width, height))?
    {
        decoder.read_filter_intra_mode()?;
    }

    // Real per-pixel palette color-index map read (spec: right after the mode-info tail
    // above, before `tx_size` -- `read_palette_mode_info`'s doc) -- required for bitstream
    // sync whenever either plane actually selected palette mode.
    if cu.palette.y_size > 0 || cu.palette.uv_size > 0 {
        read_palette_tokens(
            decoder,
            x4,
            y4,
            width_4x4,
            height_4x4,
            has_chroma,
            &cu.palette,
            mi_rows,
            mi_cols,
        )?;
    }

    super::tx_size::read_intra_tx_size(decoder, tile_ctx, rect, mi, frame, cu)?;
    Ok(mode_symbol)
}
