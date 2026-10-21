//! The intra mode-info tail of a non-IntraBC intra block: `y_mode`, `angle_delta_y`, `uv_mode`/
//! `cfl_alpha`/`angle_delta_uv`, `palette_mode_info`, `filter_intra_mode_info`, the per-pixel
//! palette color map, and `tx_size()`. Real dav1d runs this whole sequence under
//! `if (b->intra)`, and `b->intra = !intrabc_flag`, so IntraBC blocks skip all of it.
//!
//! The same code path serves key-frame intra blocks and intra blocks inside inter frames; the one
//! real difference is which CDF/context `y_mode` draws from (see
//! `SymbolDecoder::read_intra_mode_inter_frame`).

use super::contexts::y_mode_size_context;
use super::palette::{read_palette_mode_info, read_palette_tokens};
use super::types::{CodingUnit, PredictionMode};
use crate::symbol::SymbolDecoder;
use crate::tile::{BlockRect, FrameCodingParams, MiRect, TileContext};
use bitvue_engine::{BitvueError, Result};

/// Fills `cu.mode`, `cu.palette` and `cu.tx_size`. Returns the intra mode that
/// `read_transform_type_is_1d` needs later as its `y_mode_raw`: the raw `y_mode` symbol
/// (0..=12), or for a filter-intra block the mode its filter maps to.
pub(super) fn read_intra_mode_info(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
) -> Result<u8> {
    let mode_symbol = read_luma_mode(decoder, tile_ctx, mi, frame, cu)?;

    // `HasChroma` (dav1d `decode_b`): blocks narrower or shorter than 8 luma samples share one
    // chroma block with their neighbours, and only the last one (odd 4x4 column/row) codes it.
    let has_chroma = super::contexts::has_chroma(mi, &frame.tx_type_flags);
    let uv_mode_symbol = if has_chroma {
        read_chroma_mode(decoder, rect, mi, frame, mode_symbol)?
    } else {
        0
    };

    read_palette_info(
        decoder,
        tile_ctx,
        mi,
        frame,
        cu,
        (mode_symbol, uv_mode_symbol, has_chroma),
    )?;
    let tx_type_mode = read_filter_intra(decoder, rect, mi, frame, cu, mode_symbol)?;

    // Real per-pixel palette color-index map read (spec: right after the mode-info tail
    // above, before `tx_size` -- `read_palette_mode_info`'s doc) -- required for bitstream
    // sync whenever either plane actually selected palette mode.
    if cu.palette.y_size > 0 || cu.palette.uv_size > 0 {
        read_palette_tokens(
            decoder,
            mi.x4,
            mi.y4,
            mi.width,
            mi.height,
            has_chroma,
            &cu.palette,
            frame.mi_rows,
            frame.mi_cols,
        )?;
    }

    super::tx_size::read_intra_tx_size(decoder, tile_ctx, rect, mi, frame, cu)?;
    Ok(tx_type_mode)
}

/// `y_mode` and `angle_delta_y`; records the mode in the neighbour context and returns the raw
/// symbol.
fn read_luma_mode(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
) -> Result<u8> {
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    // Read INTRA prediction mode -- real per-context CDF + adaptation. Key frames use
    // `kfym` (real above/left neighbor-mode-class context, `read_intra_mode`'s doc);
    // non-key-frame intra CUs use a real block-size-class context instead (`y_mode_cdf`,
    // `read_intra_mode_inter_frame`'s doc) -- a real, deliberate CDF-source swap on the
    // SAME unified intra mode-info path, not two independent implementations.
    let mode_symbol = if frame.is_key_frame {
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
    Ok(mode_symbol)
}

/// uv_mode / cfl_alpha / angle_delta_uv -- real per-context CDF + adaptation, only called when
/// the block has chroma. Returns the raw `uv_mode` symbol. `cfl_allowed` (dav1d): lossless ->
/// the chroma block is exactly 4x4 (`cbw4 == 1 && cbh4 == 1`); otherwise both luma dims `<= 32`.
/// This crate only tracks frame-wide `coded_lossless`, not per-segment.
fn read_chroma_mode(
    decoder: &mut SymbolDecoder,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    mode_symbol: u8,
) -> Result<u8> {
    let flags = &frame.tx_type_flags;
    let (width_4x4, height_4x4) = (mi.width, mi.height);
    let ss_x = u32::from(flags.subsampling_x);
    let ss_y = u32::from(flags.subsampling_y);
    let cfl_allowed = if flags.coded_lossless {
        (width_4x4 + ss_x) >> ss_x == 1 && (height_4x4 + ss_y) >> ss_y == 1
    } else {
        rect.width <= 32 && rect.height <= 32
    };
    let uv_mode_symbol = decoder.read_uv_mode(cfl_allowed, mode_symbol)?;
    if uv_mode_symbol == 13 {
        decoder.read_cfl_alphas()?;
    } else if width_4x4.ilog2() + height_4x4.ilog2() >= 2 && (1..=8).contains(&uv_mode_symbol) {
        decoder.read_angle_delta(uv_mode_symbol - 1)?;
    }
    Ok(uv_mode_symbol)
}

/// palette_mode_info (spec 5.11.46): reads it when the block is eligible, otherwise clears the
/// palette sizes the block covers. `modes` is `(y_mode, uv_mode, has_chroma)`.
fn read_palette_info(
    decoder: &mut SymbolDecoder,
    tile_ctx: &mut TileContext,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &mut CodingUnit,
    modes: (u8, u8, bool),
) -> Result<()> {
    let MiRect {
        x4,
        y4,
        width: width_4x4,
        height: height_4x4,
    } = mi;
    let (mode_symbol, uv_mode_symbol, has_chroma) = modes;
    // Real spec eligibility gate (`read_pal_indices`'s call site in dav1d's `decode_b`):
    // `allow_screen_content_tools`, `max(bw4,bh4)<=16` (both dims `<=64px`), `bw4+bh4>=4`
    // (excludes only 4x4/4x8/8x4).
    let palette_eligible = frame.allow_screen_content_tools
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
    } else {
        // A block that cannot have a palette still overwrites its neighbours' palette sizes with
        // zero (dav1d stores `b->pal_sz` for every block); otherwise a later block would see the
        // size of whatever palette block came before it.
        tile_ctx.set_pal_size(0, x4, y4, width_4x4, height_4x4, 0);
        tile_ctx.set_pal_size(1, x4, y4, width_4x4, height_4x4, 0);
    }
    Ok(())
}

/// filter_intra_mode_info -- real per-`BlockSize` CDF + adaptation. Real spec gate:
/// `y_mode == DC_PRED`, no Y palette, both dims `<=32px` (`max(log2(bw4),log2(bh4))<=3`), and the
/// sequence header enables it. Returns the mode the transform-type CDF is indexed by: the luma
/// mode, except that a filter-intra block (`y_mode == FILTER_PRED`) uses the intra mode its
/// filter corresponds to (dav1d's `dav1d_filter_mode_to_y_mode`: DC, V, H, HOR_DOWN, DC).
fn read_filter_intra(
    decoder: &mut SymbolDecoder,
    rect: BlockRect,
    mi: MiRect,
    frame: &FrameCodingParams,
    cu: &CodingUnit,
    mode_symbol: u8,
) -> Result<u8> {
    let mut tx_type_mode = mode_symbol;
    if mode_symbol == 0
        && cu.palette.y_size == 0
        && mi.width.ilog2().max(mi.height.ilog2()) <= 3
        && frame.enable_filter_intra
        && decoder.read_use_filter_intra(block_size_for_dimensions(rect.width, rect.height))?
    {
        const FILTER_MODE_TO_Y_MODE: [u8; 5] = [0, 1, 2, 6, 0];
        let filter_mode = decoder.read_filter_intra_mode()?;
        tx_type_mode = FILTER_MODE_TO_Y_MODE[usize::from(filter_mode).min(4)];
    }
    Ok(tx_type_mode)
}

/// Convert INTRA mode symbol to PredictionMode
fn intra_mode_from_symbol(symbol: u8) -> Result<PredictionMode> {
    match symbol {
        0 => Ok(PredictionMode::DcPred),
        1 => Ok(PredictionMode::VPred),
        2 => Ok(PredictionMode::HPred),
        3 => Ok(PredictionMode::D45Pred),
        4 => Ok(PredictionMode::D135Pred),
        5 => Ok(PredictionMode::D113Pred),
        6 => Ok(PredictionMode::D157Pred),
        7 => Ok(PredictionMode::D203Pred),
        8 => Ok(PredictionMode::D67Pred),
        9 => Ok(PredictionMode::SmoothPred),
        10 => Ok(PredictionMode::SmoothVPred),
        11 => Ok(PredictionMode::SmoothHPred),
        12 => Ok(PredictionMode::PaethPred),
        _ => Err(BitvueError::InvalidData(format!(
            "Invalid INTRA mode symbol: {}",
            symbol
        ))),
    }
}

/// Map a CU's real pixel dimensions to this crate's `BlockSize` enum -- used only by
/// `read_use_filter_intra`'s CDF lookup (the real spec table is indexed by exact block size, not
/// by the coarser `bsize_ctx`/`tx_size` classes used elsewhere). Every block size the partition
/// tree can produce is covered; the fallback exists only for defensive safety.
fn block_size_for_dimensions(width: u32, height: u32) -> crate::tile::BlockSize {
    use crate::tile::BlockSize::*;
    match (width, height) {
        (4, 4) => Block4x4,
        (4, 8) => Block4x8,
        (8, 4) => Block8x4,
        (8, 8) => Block8x8,
        (8, 16) => Block8x16,
        (16, 8) => Block16x8,
        (16, 16) => Block16x16,
        (16, 32) => Block16x32,
        (32, 16) => Block32x16,
        (32, 32) => Block32x32,
        (32, 64) => Block32x64,
        (64, 32) => Block64x32,
        (64, 64) => Block64x64,
        (64, 128) => Block64x128,
        (128, 64) => Block128x64,
        (128, 128) => Block128x128,
        (16, 4) => Block16x4,
        (4, 16) => Block4x16,
        (32, 8) => Block32x8,
        (64, 16) => Block64x16,
        (128, 32) => Block128x32,
        (8, 32) => Block8x32,
        (16, 64) => Block16x64,
        (32, 128) => Block32x128,
        _ => Block4x4,
    }
}

#[cfg(test)]
#[path = "intra_tests.rs"]
mod intra_tests;
